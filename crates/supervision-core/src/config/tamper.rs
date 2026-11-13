use chrono::Utc;
use std::time::Instant;

pub struct TamperGuard {
    unlock_requested_at: Option<Instant>,
    cooling_off_duration: std::time::Duration,
}

impl TamperGuard {
    pub fn new(cooling_off_secs: u64) -> Self {
        Self {
            unlock_requested_at: None,
            cooling_off_duration: std::time::Duration::from_secs(cooling_off_secs),
        }
    }

    pub fn request_unlock(&mut self) -> u64 {
        self.unlock_requested_at = Some(Instant::now());
        self.cooling_off_duration.as_secs()
    }

    pub fn can_unlock(&self) -> bool {
        match self.unlock_requested_at {
            Some(requested) => requested.elapsed() >= self.cooling_off_duration,
            None => false,
        }
    }

    pub fn remaining_cooling_off_secs(&self) -> u64 {
        match self.unlock_requested_at {
            Some(requested) => {
                let elapsed = requested.elapsed();
                if elapsed >= self.cooling_off_duration {
                    0
                } else {
                    (self.cooling_off_duration - elapsed).as_secs()
                }
            }
            None => self.cooling_off_duration.as_secs(),
        }
    }

    pub fn cancel_unlock_request(&mut self) {
        self.unlock_requested_at = None;
    }
}
