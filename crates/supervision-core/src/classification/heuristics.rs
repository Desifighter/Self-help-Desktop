use chrono::Utc;
use super::model::{ClassificationCategory, ClassificationSource, ClassificationStatus, DomainClassification};

pub struct HeuristicDomainClassifier {
    adult_tlds: Vec<&'static str>,
    strong_keywords: Vec<&'static str>,
    weak_keywords: Vec<&'static str>,
}

impl HeuristicDomainClassifier {
    pub fn new() -> Self {
        Self {
            adult_tlds: vec![
                ".xxx", ".porn", ".sex", ".adult", ".sexy", ".cam", ".tube", ".erotica"
            ],
            strong_keywords: vec![
                "porn", "pornhub", "xxx", "xvideo", "xvideos", "xnxx", "xhamster", "xham", "redtube",
                "youporn", "hentai", "milf", "brazzers", "bangbros", "chaturbate", "livejasmin",
                "stripchat", "camgirl", "onlyfans", "fansly", "manyvids", "pornstar", "spankbang",
                "tube8", "porntube", "sextube", "eporner", "beeg", "hqporner", "tnaflix", "thumbzilla",
                "motherless", "txxx", "tubepornclassic", "drtuber", "nuvid", "vporn", "daftsex",
                "daftporn", "pornone", "bravoteens", "fapello", "heavy-r", "leakgirls", "cam4",
                "camsoda", "bongacams", "flirt4free", "myfreecams", "porngo", "pornhat", "porn300",
                "4tube", "cumlouder", "porndoe", "hdporn", "jav", "javhd", "nsfw",
                // Desi & Regional Adult Keywords
                "desibabe", "bhabhi", "antvasna", "kamababa", "desikahani", "savita", "chut", "choot",
                "malluporn", "tamilporn", "teluguporn", "banglaporn", "hindisex", "indianporn", "indiansex",
                // Explicit Anatomy & Acts
                "pussy", "boobs", "tits", "cock", "blowjob", "handjob", "creampie", "gangbang",
                "deepthroat", "cuckold", "dildo", "masturbat", "orgasm", "ejaculat", "cumshot",
                "slut", "whore", "cunt", "shemale", "transsexual", "voyeur", "upskirt", "incest",
                "porno", "sexo", "sexi", "erotik"
            ],
            weak_keywords: vec![
                "nude", "naked", "erotic", "fetish", "bdsm"
            ],
        }
    }

    pub fn classify(&self, domain: &str) -> Option<DomainClassification> {
        let norm = domain.to_lowercase();

        // 1. Adult TLD check
        for tld in &self.adult_tlds {
            if norm.ends_with(tld) {
                return Some(DomainClassification {
                    domain: norm,
                    category: ClassificationCategory::Pornography,
                    status: ClassificationStatus::Confirmed,
                    source: ClassificationSource::Heuristic,
                    confidence: 0.95,
                    detected_at: Utc::now(),
                });
            }
        }

        // Domain stem (without TLD)
        let stem = match norm.rfind('.') {
            Some(idx) => &norm[..idx],
            None => &norm[..],
        };

        // 2. Strong Adult Keyword check
        for kw in &self.strong_keywords {
            if stem.contains(kw) {
                return Some(DomainClassification {
                    domain: norm,
                    category: ClassificationCategory::Pornography,
                    status: ClassificationStatus::Confirmed,
                    source: ClassificationSource::Heuristic,
                    confidence: 0.90,
                    detected_at: Utc::now(),
                });
            }
        }

        // 3. Desi / Regional Compound Heuristic
        if stem.contains("desi") {
            let desi_triggers = [
                "babe", "sex", "porn", "hot", "kahani", "bhabhi", "mms", "scandal",
                "leak", "video", "clip", "aunty", "chut", "maal"
            ];
            for trigger in &desi_triggers {
                if stem.contains(trigger) {
                    return Some(DomainClassification {
                        domain: norm,
                        category: ClassificationCategory::Pornography,
                        status: ClassificationStatus::Confirmed,
                        source: ClassificationSource::Heuristic,
                        confidence: 0.92,
                        detected_at: Utc::now(),
                    });
                }
            }
        }

        // 4. Babe Compound Heuristic
        if stem.contains("babe") {
            let babe_triggers = [
                "desi", "hot", "sexy", "teen", "cam", "tube", "naked", "nude", "milf", "live"
            ];
            for trigger in &babe_triggers {
                if stem.contains(trigger) {
                    return Some(DomainClassification {
                        domain: norm,
                        category: ClassificationCategory::Pornography,
                        status: ClassificationStatus::Confirmed,
                        source: ClassificationSource::Heuristic,
                        confidence: 0.88,
                        detected_at: Utc::now(),
                    });
                }
            }
        }

        None
    }

    /// Automatically inspects active browser window titles for explicit adult content
    pub fn classify_window_title(&self, title: &str, process_name: &str) -> bool {
        let p_lower = process_name.to_lowercase();
        // Only inspect known browser processes or electron wrappers
        let is_browser = p_lower.contains("chrome")
            || p_lower.contains("edge")
            || p_lower.contains("firefox")
            || p_lower.contains("brave")
            || p_lower.contains("opera")
            || p_lower.contains("safari")
            || p_lower.contains("browser");

        if !is_browser {
            return false;
        }

        let t_lower = title.to_lowercase();

        // Safe overrides for developer & work research
        let safe_titles = [
            "visual studio", "github", "gitlab", "stackoverflow",
            "reddit.com/r/rust", "crates.io", "docs.rs", "google search",
            "microsoft teams", "outlook", "slack", "zoom"
        ];
        for safe in &safe_titles {
            if t_lower.contains(safe) {
                return false;
            }
        }

        // Check for strong adult site names & explicit indicators in window title
        for kw in &self.strong_keywords {
            if t_lower.contains(kw) {
                return true;
            }
        }

        for tld in &self.adult_tlds {
            if t_lower.contains(tld) {
                return true;
            }
        }

        false
    }
}

