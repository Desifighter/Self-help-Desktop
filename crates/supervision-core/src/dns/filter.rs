use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::Instant;
use crate::classification::{
    BlocklistProvider, HeuristicDomainClassifier, UpstreamCategorizationProvider,
    DomainClassification, ClassificationCategory,
};

pub struct DnsFilter {
    blocklist: Arc<BlocklistProvider>,
    heuristics: Arc<HeuristicDomainClassifier>,
    upstream: Arc<UpstreamCategorizationProvider>,
    cache: Arc<RwLock<HashMap<String, (DomainClassification, Instant)>>>,
    pub block_adult: Arc<RwLock<bool>>,
    pub block_social: Arc<RwLock<bool>>,
    pub block_gambling: Arc<RwLock<bool>>,
}

impl DnsFilter {
    pub fn new() -> Self {
        Self {
            blocklist: Arc::new(BlocklistProvider::new()),
            heuristics: Arc::new(HeuristicDomainClassifier::new()),
            upstream: Arc::new(UpstreamCategorizationProvider::new()),
            cache: Arc::new(RwLock::new(HashMap::new())),
            block_adult: Arc::new(RwLock::new(true)),
            block_social: Arc::new(RwLock::new(false)),
            block_gambling: Arc::new(RwLock::new(true)),
        }
    }

    pub async fn evaluate(&self, raw_domain: &str) -> Option<DomainClassification> {
        let domain = raw_domain.trim_end_matches('.').to_lowercase();

        // 0. Cache check
        {
            let cache = self.cache.read();
            if let Some((classification, created)) = cache.get(&domain) {
                if created.elapsed().as_secs() < 3600 {
                    if classification.category == ClassificationCategory::Safe {
                        return None;
                    } else {
                        return Some(classification.clone());
                    }
                }
            }
        }

        let adult = *self.block_adult.read();
        let social = *self.block_social.read();
        let gambling = *self.block_gambling.read();

        // 1. Direct blocklist & allowlist check
        if let Some(classification) = self.blocklist.check_domain(&domain, adult, social, gambling) {
            self.cache.write().insert(domain.clone(), (classification.clone(), Instant::now()));
            if classification.category == ClassificationCategory::Safe {
                return None;
            } else {
                return Some(classification);
            }
        }

        // 2. High-confidence heuristic classifier
        if adult {
            if let Some(classification) = self.heuristics.classify(&domain) {
                self.cache.write().insert(domain.clone(), (classification.clone(), Instant::now()));
                return Some(classification);
            }
        }

        // 3. Upstream privacy-preserving DNS categorization query
        if adult {
            if let Some(classification) = self.upstream.query_categorization(&domain).await {
                self.cache.write().insert(domain.clone(), (classification.clone(), Instant::now()));
                return Some(classification);
            }
        }

        // Default: Domain is safe
        None
    }

    pub fn blocklist(&self) -> &Arc<BlocklistProvider> {
        &self.blocklist
    }
}
