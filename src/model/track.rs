use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use super::fx::Delay;
use super::pattern::Pattern;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TrackPlayback {
    #[serde(alias = "clip")]
    #[default]
    Gate,
    #[serde(alias = "replace")]
    Mono,
    OneShot,
}

impl TrackPlayback {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrackPlayback::Gate => "gate",
            TrackPlayback::Mono => "mono",
            TrackPlayback::OneShot => "one_shot",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
    pub sample: Option<String>,
    pub delay: Delay,
    pub pattern: Option<Pattern>,
    /// Named pattern variations (e.g., "a", "b", "fill")
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub variations: HashMap<String, Pattern>,
    /// Current active variation (if any)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_variation: Option<String>,
    pub mute: bool,
    pub solo: bool,
    #[serde(default)]
    pub playback: TrackPlayback,
    pub gain_db: f32,
    #[serde(default = "default_division")]
    pub div: u32, // tokens per beat (default 4 => 16th notes)
}

impl Track {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sample: None,
            delay: Delay::default(),
            pattern: None,
            variations: HashMap::new(),
            current_variation: None,
            mute: false,
            solo: false,
            playback: TrackPlayback::default(),
            gain_db: 0.0,
            div: default_division(),
        }
    }
    
    /// Get the currently active pattern (variation or main).
    pub fn active_pattern(&self) -> Option<&Pattern> {
        if let Some(ref var_name) = self.current_variation {
            self.variations.get(var_name)
        } else {
            self.pattern.as_ref()
        }
    }
}

fn default_division() -> u32 { 4 }
