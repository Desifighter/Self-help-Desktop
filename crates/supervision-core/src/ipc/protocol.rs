use serde::{Deserialize, Serialize};
use crate::classification::DomainClassification;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub is_running: bool,
    pub uptime_secs: u64,
    pub total_dns_queries: u64,
    pub blocked_dns_queries: u64,
    pub adult_blocked: u64,
    pub social_blocked: u64,
    pub gambling_blocked: u64,
    pub custom_blocked: u64,
    pub filter_adult: bool,
    pub filter_social: bool,
    pub filter_gambling: bool,
    pub custom_blocked_domains: Vec<String>,
    pub restricted_apps: Vec<String>,
    pub is_locked: bool,
    pub remaining_cooling_off_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockEvent {
    pub timestamp: String,
    pub target: String,
    pub category: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcRequest {
    GetStatus,
    UpdateFilterSettings {
        filter_adult: bool,
        filter_social: bool,
        filter_gambling: bool,
    },
    AddCustomBlockedDomain {
        domain: String,
    },
    RemoveCustomBlockedDomain {
        domain: String,
    },
    AddRestrictedApp {
        app_name: String,
    },
    RemoveRestrictedApp {
        app_name: String,
    },
    RequestUnlock,
    ConfirmUnlock {
        pin: Option<String>,
    },
    GetRecentEvents,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcResponse {
    Status(DaemonStatus),
    Success { message: String },
    Error { error: String },
    RecentEvents(Vec<BlockEvent>),
}
