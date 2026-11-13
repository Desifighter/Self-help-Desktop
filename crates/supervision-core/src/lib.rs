pub mod classification;
pub mod dns;
pub mod monitor;
pub mod config;
pub mod ipc;

pub use classification::*;
pub use dns::*;
pub use monitor::*;
pub use config::*;
pub use ipc::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocklist_embedded_and_subdomains() {
        let blocklist = BlocklistProvider::new();

        // 1. Exact match
        let res1 = blocklist.check_domain("pornhub.com", true, false, false);
        assert!(res1.is_some());
        assert_eq!(res1.unwrap().category, ClassificationCategory::Pornography);

        // 2. Subdomain match
        let res2 = blocklist.check_domain("static.video.pornhub.com", true, false, false);
        assert!(res2.is_some());
        assert_eq!(res2.unwrap().category, ClassificationCategory::Pornography);

        // 3. Safe override
        let res3 = blocklist.check_domain("github.com", true, false, false);
        assert!(res3.is_some());
        assert_eq!(res3.unwrap().category, ClassificationCategory::Safe);
    }

    #[test]
    fn test_heuristic_classifier() {
        let heuristics = HeuristicDomainClassifier::new();

        // Adult TLD
        let res1 = heuristics.classify("content.stream.xxx");
        assert!(res1.is_some());
        assert_eq!(res1.unwrap().category, ClassificationCategory::Pornography);

        // Strong keyword
        let res2 = heuristics.classify("watch-freesextube-online.org");
        assert!(res2.is_some());

        // Regional / Desi compound
        let res3 = heuristics.classify("desibabevideos.in");
        assert!(res3.is_some());

        // Normal domain should not be flagged
        let res4 = heuristics.classify("rust-lang.org");
        assert!(res4.is_none());
    }

    #[test]
    fn test_dns_packet_synthesis() {
        let packet_parser = dns::DnsMessage::parse(&[
            0x12, 0x34, // ID
            0x01, 0x00, // Query flags
            0x00, 0x01, // 1 question
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x07, b'e', b'x', b'a', b'm', b'p', b'l', b'e',
            0x03, b'c', b'o', b'm',
            0x00, // root
            0x00, 0x01, // Type A
            0x00, 0x01, // Class IN
        ]);

        assert!(packet_parser.is_some());
        let msg = packet_parser.unwrap();
        assert_eq!(msg.questions[0].name, "example.com");

        let blocked_resp = msg.build_blocked_a_response();
        assert_eq!(blocked_resp[0], 0x12);
        assert_eq!(blocked_resp[1], 0x34);
        // Answer count should be 1
        assert_eq!(blocked_resp[7], 0x01);

        let nxdomain_resp = msg.build_nxdomain_response();
        // RCODE in low nibble of byte 3 should be 3 (NXDOMAIN)
        assert_eq!(nxdomain_resp[3] & 0x0F, 0x03);
    }

    #[test]
    fn test_app_config_whitelist() {
        let config = AppConfig::default();
        assert!(config.is_essential_process("code.exe"));
        assert!(config.is_essential_process("cargo.exe"));
        assert!(config.is_essential_process("teams.exe"));
        assert!(config.is_restricted_process("telegram.exe"));
        assert!(!config.is_restricted_process("code.exe"));
    }
}
