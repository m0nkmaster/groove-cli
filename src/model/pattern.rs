use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Pattern {
    Visual(String),
    // Future: coded patterns
}

impl Pattern {
    pub fn visual(src: impl Into<String>) -> Self {
        Pattern::Visual(src.into())
    }
}

