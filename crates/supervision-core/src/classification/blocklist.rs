use std::collections::HashSet;
use parking_lot::RwLock;
use std::sync::Arc;
use chrono::Utc;
use super::model::{ClassificationCategory, ClassificationSource, ClassificationStatus, DomainClassification};

const EMBEDDED_ADULT_DOMAINS: &str = include_str!("../../../../assets/blocklists/adult_domains.txt");
const EMBEDDED_ESSENTIAL_ALLOWLIST: &str = include_str!("../../../../assets/blocklists/essential_allowlist.txt");

pub struct BlocklistProvider {
    adult_domains: Arc<RwLock<HashSet<String>>>,
    social_media_domains: Arc<RwLock<HashSet<String>>>,
    gambling_domains: Arc<RwLock<HashSet<String>>>,
    custom_blocked: Arc<RwLock<HashSet<String>>>,
    safe_allowlist: Arc<RwLock<HashSet<String>>>,
}

impl BlocklistProvider {
    pub fn new() -> Self {
        let mut adult = HashSet::new();
        for line in EMBEDDED_ADULT_DOMAINS.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                let clean = trimmed.to_lowercase()
                    .trim_start_matches("www.")
                    .trim_start_matches("0.0.0.0 ")
                    .trim_start_matches("127.0.0.1 ")
                    .trim()
                    .to_string();
                if clean.contains('.') {
                    adult.insert(clean);
                }
            }
        }

        let mut safe = HashSet::new();
        let mut in_domains_section = true;
        for line in EMBEDDED_ESSENTIAL_ALLOWLIST.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_domains_section = trimmed == "[domains]";
                continue;
            }
            if in_domains_section && !trimmed.is_empty() && !trimmed.starts_with('#') {
                let clean = trimmed.to_lowercase().trim_start_matches("www.").to_string();
                if clean.contains('.') {
                    safe.insert(clean);
                }
            }
        }

        let mut social = HashSet::new();
        let default_social = [
            "instagram.com", "cdninstagram.com", "tiktok.com", "tiktokcdn.com",
            "twitter.com", "x.com", "twimg.com", "reddit.com", "redd.it",
            "facebook.com", "fbcdn.net", "snapchat.com", "threads.net",
            "pinterest.com", "tumblr.com", "vk.com", "weibo.com"
        ];
        for d in default_social {
            social.insert(d.to_string());
        }

        let mut gambling = HashSet::new();
        let default_gambling = [
            "stake.com", "bet365.com", "bovada.lv", "1xbet.com", "draftkings.com",
            "fanduel.com", "pokerstars.com", "888casino.com", "roobet.com",
            "parimatch.com", "betway.com", "ggbet.com", "betonline.ag"
        ];
        for d in default_gambling {
            gambling.insert(d.to_string());
        }

        Self {
            adult_domains: Arc::new(RwLock::new(adult)),
            social_media_domains: Arc::new(RwLock::new(social)),
            gambling_domains: Arc::new(RwLock::new(gambling)),
            custom_blocked: Arc::new(RwLock::new(HashSet::new())),
            safe_allowlist: Arc::new(RwLock::new(safe)),
        }
    }

    pub fn is_safe_allowlisted(&self, domain: &str) -> bool {
        let safe = self.safe_allowlist.read();
        let norm = domain.to_lowercase().trim_start_matches("www.").to_string();
        if safe.contains(&norm) {
            return true;
        }

        // Subdomain check (e.g. docs.github.com matches github.com)
        let parts: Vec<&str> = norm.split('.').collect();
        for i in 1..parts.len().saturating_sub(1) {
            let parent = parts[i..].join(".");
            if safe.contains(&parent) {
                return true;
            }
        }
        false
    }

    pub fn check_domain(&self, domain: &str, check_adult: bool, check_social: bool, check_gambling: bool) -> Option<DomainClassification> {
        let norm = domain.to_lowercase().trim_start_matches("www.").to_string();

        // 1. Safe allowlist has absolute precedence
        if self.is_safe_allowlisted(&norm) {
            return Some(DomainClassification {
                domain: norm,
                category: ClassificationCategory::Safe,
                status: ClassificationStatus::Confirmed,
                source: ClassificationSource::SafeAllowlist,
                confidence: 1.0,
                detected_at: Utc::now(),
            });
        }

        // 2. Custom user blocklist
        {
            let custom = self.custom_blocked.read();
            if custom.contains(&norm) || self.matches_parent(&norm, &custom) {
                return Some(DomainClassification {
                    domain: norm,
                    category: ClassificationCategory::CustomBlocked,
                    status: ClassificationStatus::Confirmed,
                    source: ClassificationSource::CustomRule,
                    confidence: 1.0,
                    detected_at: Utc::now(),
                });
            }
        }

        // 3. Adult domain blocklist
        if check_adult {
            let adult = self.adult_domains.read();
            if adult.contains(&norm) || self.matches_parent(&norm, &adult) {
                return Some(DomainClassification {
                    domain: norm,
                    category: ClassificationCategory::Pornography,
                    status: ClassificationStatus::Confirmed,
                    source: ClassificationSource::LocalBlocklist,
                    confidence: 1.0,
                    detected_at: Utc::now(),
                });
            }
        }

        // 4. Social media blocklist
        if check_social {
            let social = self.social_media_domains.read();
            if social.contains(&norm) || self.matches_parent(&norm, &social) {
                return Some(DomainClassification {
                    domain: norm,
                    category: ClassificationCategory::SocialMedia,
                    status: ClassificationStatus::Confirmed,
                    source: ClassificationSource::LocalBlocklist,
                    confidence: 1.0,
                    detected_at: Utc::now(),
                });
            }
        }

        // 5. Gambling blocklist
        if check_gambling {
            let gambling = self.gambling_domains.read();
            if gambling.contains(&norm) || self.matches_parent(&norm, &gambling) {
                return Some(DomainClassification {
                    domain: norm,
                    category: ClassificationCategory::Gambling,
                    status: ClassificationStatus::Confirmed,
                    source: ClassificationSource::LocalBlocklist,
                    confidence: 1.0,
                    detected_at: Utc::now(),
                });
            }
        }

        None
    }

    fn matches_parent(&self, domain: &str, set: &HashSet<String>) -> bool {
        let parts: Vec<&str> = domain.split('.').collect();
        for i in 1..parts.len().saturating_sub(1) {
            let parent = parts[i..].join(".");
            if set.contains(&parent) {
                return true;
            }
        }
        false
    }

    pub fn add_custom_blocked(&self, domain: &str) {
        let clean = domain.to_lowercase().trim_start_matches("www.").to_string();
        self.custom_blocked.write().insert(clean);
    }

    pub fn remove_custom_blocked(&self, domain: &str) {
        let clean = domain.to_lowercase().trim_start_matches("www.").to_string();
        self.custom_blocked.write().remove(&clean);
    }

    pub fn add_safe_domain(&self, domain: &str) {
        let clean = domain.to_lowercase().trim_start_matches("www.").to_string();
        self.safe_allowlist.write().insert(clean);
    }

    pub fn get_adult_count(&self) -> usize {
        self.adult_domains.read().len()
    }

    pub fn get_custom_blocked_list(&self) -> Vec<String> {
        self.custom_blocked.read().iter().cloned().collect()
    }
}
