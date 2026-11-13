use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};
use parking_lot::RwLock;
use tracing::{info, warn, debug};
use super::packet::DnsMessage;
use super::filter::DnsFilter;
use crate::classification::DomainClassification;

pub struct DnsProxyStats {
    pub total_queries: u64,
    pub blocked_queries: u64,
    pub adult_blocked: u64,
    pub social_blocked: u64,
    pub gambling_blocked: u64,
    pub custom_blocked: u64,
}

pub struct DnsProxy {
    filter: Arc<DnsFilter>,
    listen_addr: SocketAddr,
    upstream_addr: SocketAddr,
    stats: Arc<RwLock<DnsProxyStats>>,
    recent_blocks: Arc<RwLock<Vec<DomainClassification>>>,
}

impl DnsProxy {
    pub fn new(filter: Arc<DnsFilter>, listen_addr: SocketAddr, upstream_addr: SocketAddr) -> Self {
        Self {
            filter,
            listen_addr,
            upstream_addr,
            stats: Arc::new(RwLock::new(DnsProxyStats {
                total_queries: 0,
                blocked_queries: 0,
                adult_blocked: 0,
                social_blocked: 0,
                gambling_blocked: 0,
                custom_blocked: 0,
            })),
            recent_blocks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn stats(&self) -> Arc<RwLock<DnsProxyStats>> {
        Arc::clone(&self.stats)
    }

    pub fn recent_blocks(&self) -> Arc<RwLock<Vec<DomainClassification>>> {
        Arc::clone(&self.recent_blocks)
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let socket = UdpSocket::bind(self.listen_addr).await?;
        info!("Supervision DNS Proxy listening on UDP {}", self.listen_addr);

        let mut buf = [0u8; 2048];

        loop {
            let (len, client_addr) = match socket.recv_from(&mut buf).await {
                Ok(res) => res,
                Err(e) => {
                    warn!("Error receiving DNS packet: {}", e);
                    continue;
                }
            };

            self.stats.write().total_queries += 1;

            let query_buf = buf[..len].to_vec();
            let filter = Arc::clone(&self.filter);
            let stats = Arc::clone(&self.stats);
            let recent = Arc::clone(&self.recent_blocks);
            let upstream = self.upstream_addr;

            // Spawn lightweight task to process query with sub-millisecond response
            let socket_ref = socket.try_clone()?;
            tokio::spawn(async move {
                if let Some(msg) = DnsMessage::parse(&query_buf) {
                    if let Some(q) = msg.questions.first() {
                        let domain = &q.name;
                        debug!("DNS Query: {} (type {})", domain, q.qtype);

                        if let Some(classification) = filter.evaluate(domain).await {
                            info!("BLOCKED DNS QUERY: {} [{}]", domain, classification.category);
                            
                            // Update counters
                            {
                                let mut s = stats.write();
                                s.blocked_queries += 1;
                                match classification.category {
                                    crate::classification::ClassificationCategory::Pornography => s.adult_blocked += 1,
                                    crate::classification::ClassificationCategory::SocialMedia => s.social_blocked += 1,
                                    crate::classification::ClassificationCategory::Gambling => s.gambling_blocked += 1,
                                    crate::classification::ClassificationCategory::CustomBlocked => s.custom_blocked += 1,
                                    _ => {},
                                }
                            }

                            // Record in recent blocks list (capped to 50 items)
                            {
                                let mut r = recent.write();
                                r.insert(0, classification);
                                if r.len() > 50 {
                                    r.truncate(50);
                                }
                            }

                            // Return synthetic 0.0.0.0 / NXDOMAIN response
                            let resp = msg.build_blocked_a_response();
                            let _ = socket_ref.send_to(&resp, client_addr).await;
                            return;
                        }
                    }
                }

                // If allowed, forward query to upstream DNS
                match Self::forward_query(&query_buf, upstream).await {
                    Ok(resp) => {
                        let _ = socket_ref.send_to(&resp, client_addr).await;
                    }
                    Err(e) => {
                        debug!("Upstream DNS forward error: {}", e);
                    }
                }
            });
        }
    }

    async fn forward_query(query: &[u8], upstream: SocketAddr) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let upstream_sock = UdpSocket::bind("0.0.0.0:0").await?;
        upstream_sock.send_to(query, upstream).await?;

        let mut buf = [0u8; 2048];
        let (len, _) = timeout(Duration::from_millis(2500), upstream_sock.recv_from(&mut buf)).await??;
        Ok(buf[..len].to_vec())
    }
}
