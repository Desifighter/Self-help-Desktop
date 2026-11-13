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
                "porn", "pornhub", "xxx", "xvideo", "xvideos", "xnxx", "xhamster", "redtube",
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
}
