use serde::{Deserialize, Serialize};

use super::pattern::Pattern;
use super::track::Track;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub bpm: u32,
    pub steps: u8,
    pub swing: u8, // percent 0..100
    pub tracks: Vec<Track>,
}

impl Default for Song {
    fn default() -> Self {
        Self {
            bpm: 120,
            steps: 16,
            swing: 0,
            tracks: Vec::new(),
        }
    }
}

impl Song {
    pub fn list(&self) -> String {
        if self.tracks.is_empty() {
            return "[no tracks]".to_string();
        }
        let mut out = String::new();
        for (i, t) in self.tracks.iter().enumerate() {
            let fx = if t.delay.on {
                format!(
                    "delay {} fb{:.2} mix{:.2}",
                    t.delay.time, t.delay.feedback, t.delay.mix
                )
            } else {
                "delay off".to_string()
            };
            let sample = t.sample.as_deref().unwrap_or("-");
            let pattern = match &t.pattern {
                Some(Pattern::Visual(p)) => format!("pattern: {}", p),
                None => "pattern: [unset]".to_string(),
            };
            let mute = if t.mute { "on" } else { "off" };
            let solo = if t.solo { "on" } else { "off" };
            out.push_str(&format!(
                "{:>2} {}  {}  sample: {}  {}  mute:{} solo:{} gain:{:+.1}dB\n",
                i + 1,
                t.name,
                fx,
                sample,
                pattern,
                mute,
                solo,
                t.gain_db,
            ));
        }
        out
    }
}
