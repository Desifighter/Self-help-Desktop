use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClassificationCategory {
    Safe,
    Pornography,
    SocialMedia,
    Gambling,
    CustomBlocked,
    Uncertain,
    Unknown,
}

impl std::fmt::Display for ClassificationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Safe => write!(f, "SAFE"),
            Self::Pornography => write!(f, "PORNOGRAPHY"),
            Self::SocialMedia => write!(f, "SOCIAL_MEDIA"),
            Self::Gambling => write!(f, "GAMBLING"),
            Self::CustomBlocked => write!(f, "CUSTOM_BLOCKED"),
            Self::Uncertain => write!(f, "UNCERTAIN"),
            Self::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassificationSource {
    LocalBlocklist,
    Heuristic,
    UpstreamDns,
    SafeAllowlist,
    CustomRule,
    DefaultPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassificationStatus {
    Confirmed,
    Pending,
    Temporary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainClassification {
    pub domain: String,
    pub category: ClassificationCategory,
    pub status: ClassificationStatus,
    pub source: ClassificationSource,
    pub confidence: f32,
    pub detected_at: DateTime<Utc>,
}
