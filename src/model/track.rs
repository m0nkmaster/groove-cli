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
            mute: false,
            solo: false,
            playback: TrackPlayback::default(),
            gain_db: 0.0,
            div: default_division(),
        }
    }
}

fn default_division() -> u32 { 4 }
