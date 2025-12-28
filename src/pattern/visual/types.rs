use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Rest,
    Tie,
    Hit(StepEvent),
    Chord(Vec<StepEvent>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteToken {
    pub pitch_class: u8,    // 0..=11 (C..B)
    pub octave: Option<i8>, // MIDI octave number (C4 is octave 4)
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepEvent {
    pub note: StepNote,
    pub ratchet: Option<u32>,
    pub nudge: Option<Nudge>,
    pub gate: Option<Gate>,
}

impl Default for StepEvent {
    fn default() -> Self {
        Self {
            note: StepNote::default(),
            ratchet: None,
            nudge: None,
            gate: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepNote {
    pub base_note: Option<NoteToken>,
    pub pitch_offset: i32,
    pub velocity: Option<u8>,
    pub accent: bool,
    pub probability: Option<f32>,
    pub cycle: Option<CycleCondition>,
    pub param_locks: Vec<ParamLock>,
}

impl Default for StepNote {
    fn default() -> Self {
        Self {
            base_note: None,
            pitch_offset: 0,
            velocity: None,
            accent: false,
            probability: None,
            cycle: None,
            param_locks: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamLock {
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CycleCondition {
    pub hit: u32,
    pub of: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Nudge {
    Millis(f32),
    Percent(f32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gate {
    Fraction { numerator: u32, denominator: u32 },
    Percent(f32),
    Float(f32),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParseError {
    #[error("unexpected end of input")]
    UnexpectedEnd,
    #[error("unexpected character '{found}' at position {position}")]
    UnexpectedChar { position: usize, found: char },
    #[error("expected number at position {position}")]
    ExpectedNumber { position: usize },
    #[error("invalid number at position {position}")]
    InvalidNumber { position: usize },
    #[error("invalid chord contents at position {position}")]
    InvalidChord { position: usize },
    #[error("repeat count must be positive at position {position}")]
    InvalidRepeat { position: usize },
}

