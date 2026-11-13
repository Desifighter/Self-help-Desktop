use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::timeout;
use chrono::Utc;
use super::model::{ClassificationCategory, ClassificationSource, ClassificationStatus, DomainClassification};

const CLOUDFLARE_FAMILY_DNS: &str = "1.1.1.3:53";
const CLEANBROWSING_ADULT_DNS: &str = "185.228.168.168:53";
const QUERY_TIMEOUT: Duration = Duration::from_millis(1500);

pub struct UpstreamCategorizationProvider;

impl UpstreamCategorizationProvider {
    pub fn new() -> Self {
        Self
    }

    pub async fn query_categorization(&self, domain: &str) -> Option<DomainClassification> {
        let norm = domain.to_lowercase().trim_start_matches("www.").to_string();

        // 1. Check Cloudflare 1.1.1.3 Families
        if let Ok(is_blocked) = Self::check_upstream(&norm, CLOUDFLARE_FAMILY_DNS).await {
            if is_blocked {
                return Some(DomainClassification {
                    domain: norm,
                    category: ClassificationCategory::Pornography,
                    status: ClassificationStatus::Confirmed,
                    source: ClassificationSource::UpstreamDns,
                    confidence: 0.98,
                    detected_at: Utc::now(),
                });
            }
        }

        // 2. Cross-check CleanBrowsing 185.228.168.168
        if let Ok(is_blocked) = Self::check_upstream(&norm, CLEANBROWSING_ADULT_DNS).await {
            if is_blocked {
                return Some(DomainClassification {
                    domain: norm,
                    category: ClassificationCategory::Pornography,
                    status: ClassificationStatus::Confirmed,
                    source: ClassificationSource::UpstreamDns,
                    confidence: 0.98,
                    detected_at: Utc::now(),
                });
            }
        }

        None
    }

    async fn check_upstream(domain: &str, server: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let target: SocketAddr = server.parse()?;
        let query = Self::build_dns_query(domain);

        socket.send_to(&query, target).await?;

        let mut buf = [0u8; 512];
        let (len, _) = timeout(QUERY_TIMEOUT, socket.recv_from(&mut buf)).await??;

        let is_blocked = Self::is_blocked_ip_in_response(&buf[..len]);
        Ok(is_blocked)
    }

    fn build_dns_query(domain: &str) -> Vec<u8> {
        let mut packet = Vec::with_capacity(64);
        // Transaction ID
        packet.extend_from_slice(&[0x13, 0x37]);
        // Flags: Standard query, recursion desired
        packet.extend_from_slice(&[0x01, 0x00]);
        // Questions: 1, Answer RRs: 0, Authority RRs: 0, Additional RRs: 0
        packet.extend_from_slice(&[0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        for part in domain.split('.') {
            packet.push(part.len() as u8);
            packet.extend_from_slice(part.as_bytes());
        }
        packet.push(0x00); // Root null byte

        // Type A (1), Class IN (1)
        packet.extend_from_slice(&[0x00, 0x01, 0x00, 0x01]);
        packet
    }

    fn is_blocked_ip_in_response(packet: &[u8]) -> bool {
        if packet.len() < 12 {
            return false;
        }

        let ancount = u16::from_be_bytes([packet[6], packet[7]]) as usize;
        if ancount == 0 {
            // Check RCODE (bits 0-3 in byte 3) -> if NXDOMAIN (3) or similar
            let rcode = packet[3] & 0x0F;
            if rcode == 3 {
                return true;
            }
            return false;
        }

        // Skip header
        let mut pos = 12;

        // Skip Question section
        while pos < packet.len() && packet[pos] != 0 {
            if packet[pos] & 0xC0 == 0xC0 {
                pos += 2;
                break;
            }
            pos += 1 + packet[pos] as usize;
        }
        if pos < packet.len() && packet[pos] == 0 {
            pos += 1;
        }
        pos += 4; // Skip QTYPE and QCLASS

        // Parse Answer section looking for A records
        for _ in 0..ancount {
            if pos >= packet.len() {
                break;
            }
            // Skip NAME
            if packet[pos] & 0xC0 == 0xC0 {
                pos += 2;
            } else {
                while pos < packet.len() && packet[pos] != 0 {
                    pos += 1 + packet[pos] as usize;
                }
                pos += 1;
            }

            if pos + 10 > packet.len() {
                break;
            }
            let rtype = u16::from_be_bytes([packet[pos], packet[pos + 1]]);
            let rdlength = u16::from_be_bytes([packet[pos + 8], packet[pos + 9]]) as usize;
            pos += 10;

            if rtype == 1 && rdlength == 4 && pos + 4 <= packet.len() {
                let ip = &packet[pos..pos + 4];
                // 0.0.0.0 (Cloudflare/CleanBrowsing block)
                if ip == [0, 0, 0, 0] {
                    return true;
                }
                // 185.228.168.10 (CleanBrowsing Adult block IP)
                if ip == [185, 228, 168, 10] {
                    return true;
                }
                // 127.x.x.x
                if ip[0] == 127 {
                    return true;
                }
            }
            pos += rdlength;
        }

        false
    }
}
