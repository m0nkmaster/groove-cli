use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delay {
    pub on: bool,
    pub time: String,   // e.g. "1/4"
    pub feedback: f32,  // 0..1
    pub mix: f32,       // 0..1
}

impl Default for Delay {
    fn default() -> Self {
        Self { on: false, time: "1/4".into(), feedback: 0.35, mix: 0.25 }
    }
}

