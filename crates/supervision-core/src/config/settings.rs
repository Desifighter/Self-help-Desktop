use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub filter_adult: bool,
    pub filter_social: bool,
    pub filter_gambling: bool,
    pub custom_blocked_domains: Vec<String>,
    pub custom_allowed_domains: Vec<String>,
    pub restricted_applications: Vec<String>,
    pub essential_processes_whitelist: Vec<String>,
    pub anti_tamper_pin_hash: Option<String>,
    pub cooling_off_period_secs: u64,
    pub lock_until_timestamp: Option<i64>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            filter_adult: true,
            filter_social: false,
            filter_gambling: true,
            custom_blocked_domains: vec![],
            custom_allowed_domains: vec![],
            restricted_applications: vec![
                "telegram.exe".to_string(),
                "discord.exe".to_string(),
                "steam.exe".to_string(),
                "steamwebhelper.exe".to_string(),
                "epicgameslauncher.exe".to_string(),
                "battle.net.exe".to_string(),
            ],
            essential_processes_whitelist: vec![
                "code.exe".to_string(),
                "cargo.exe".to_string(),
                "rustc.exe".to_string(),
                "git.exe".to_string(),
                "node.exe".to_string(),
                "python.exe".to_string(),
                "docker.exe".to_string(),
                "teams.exe".to_string(),
                "ms-teams.exe".to_string(),
                "slack.exe".to_string(),
                "zoom.exe".to_string(),
                "outlook.exe".to_string(),
                "windowsterminal.exe".to_string(),
                "powershell.exe".to_string(),
                "pwsh.exe".to_string(),
                "cmd.exe".to_string(),
            ],
            anti_tamper_pin_hash: None,
            cooling_off_period_secs: 900, // 15 minutes friction barrier
            lock_until_timestamp: None,
        }
    }
}

impl AppConfig {
    pub fn load_or_default(path: &Path) -> Self {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(cfg) = serde_json::from_str::<Self>(&content) {
                    return cfg;
                }
            }
        }
        let default = Self::default();
        let _ = default.save(path);
        default
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)
    }

    pub fn is_essential_process(&self, process_name: &str) -> bool {
        let lower = process_name.to_lowercase();
        self.essential_processes_whitelist.iter().any(|p| p.to_lowercase() == lower)
    }

    pub fn is_restricted_process(&self, process_name: &str) -> bool {
        let lower = process_name.to_lowercase();
        if self.is_essential_process(&lower) {
            return false;
        }
        self.restricted_applications.iter().any(|p| p.to_lowercase() == lower)
    }
}
