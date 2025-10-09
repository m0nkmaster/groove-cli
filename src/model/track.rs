use serde::{Deserialize, Serialize};

use super::fx::Delay;
use super::pattern::Pattern;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
    pub sample: Option<String>,
    pub delay: Delay,
    pub pattern: Option<Pattern>,
    pub mute: bool,
    pub solo: bool,
    pub gain_db: f32,
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
            gain_db: 0.0,
        }
    }
}

