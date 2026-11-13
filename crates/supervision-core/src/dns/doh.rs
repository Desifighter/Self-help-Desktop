use std::collections::HashSet;

const EMBEDDED_DOH_IPS: &str = include_str!("../../../../assets/blocklists/doh_ips.txt");

pub struct DohBlocker {
    blocked_ips: HashSet<String>,
}

impl DohBlocker {
    pub fn new() -> Self {
        let mut ips = HashSet::new();
        for line in EMBEDDED_DOH_IPS.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                ips.insert(trimmed.to_string());
            }
        }
        Self { blocked_ips: ips }
    }

    pub fn is_doh_ip(&self, ip: &str) -> bool {
        self.blocked_ips.contains(ip)
    }

    pub fn get_blocked_ips(&self) -> Vec<String> {
        self.blocked_ips.iter().cloned().collect()
    }
}
