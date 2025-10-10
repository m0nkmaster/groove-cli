use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Rest,
    Tie,
    Hit(StepEvent),
    Chord(Vec<StepEvent>),
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

#[derive(Debug, Clone, PartialEq)]
pub enum Nudge {
    Millis(f32),
    Percent(f32),
}

#[derive(Debug, Clone, PartialEq)]
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

pub fn parse_visual_pattern(src: &str) -> Result<Vec<Step>, ParseError> {
    Parser::new(src).parse_pattern()
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    fn parse_pattern(mut self) -> Result<Vec<Step>, ParseError> {
        let mut steps = Vec::new();
        loop {
            self.skip_ws_and_comments();
            match self.peek() {
                None => break,
                Some(')') => {
                    return Err(ParseError::UnexpectedChar {
                        position: self.pos,
                        found: ')',
                    });
                }
                Some('|') => {
                    self.bump();
                }
                Some('.') => {
                    self.bump();
                    steps.push(Step::Rest);
                }
                Some('_') => {
                    self.bump();
                    steps.push(Step::Tie);
                }
                Some('(') => {
                    let mut group = self.parse_parenthetical()?;
                    steps.append(&mut group);
                }
                Some(c) if is_hit_symbol(c) => {
                    let step = self.parse_hit()?;
                    steps.push(step);
                }
                Some('#') => {
                    self.skip_comment();
                }
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some(found) => {
                    return Err(ParseError::UnexpectedChar {
                        position: self.pos,
                        found,
                    });
                }
            }
        }
        Ok(steps)
    }

    fn parse_steps(&mut self) -> Result<Vec<Step>, ParseError> {
        let mut steps = Vec::new();
        loop {
            self.skip_ws_and_comments();
            match self.peek() {
                None => return Err(ParseError::UnexpectedEnd),
                Some(')') => break,
                Some('|') => {
                    self.bump();
                }
                Some('.') => {
                    self.bump();
                    steps.push(Step::Rest);
                }
                Some('_') => {
                    self.bump();
                    steps.push(Step::Tie);
                }
                Some('(') => {
                    let mut group = self.parse_parenthetical()?;
                    steps.append(&mut group);
                }
                Some(c) if is_hit_symbol(c) => {
                    let hit = self.parse_hit()?;
                    steps.push(hit);
                }
                Some('#') => {
                    self.skip_comment();
                }
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some(found) => {
                    return Err(ParseError::UnexpectedChar {
                        position: self.pos,
                        found,
                    });
                }
            }
        }
        Ok(steps)
    }

    fn parse_parenthetical(&mut self) -> Result<Vec<Step>, ParseError> {
        let start = self.pos;
        self.expect('(')?;
        let inner = self.parse_steps()?;
        self.expect(')')?;
        self.skip_ws_and_comments();
        if matches!(self.peek(), Some('*')) {
            self.bump();
            let repeat_pos = self.pos;
            let count = self.parse_unsigned_int()?;
            if count == 0 {
                return Err(ParseError::InvalidRepeat {
                    position: repeat_pos,
                });
            }
            let mut out = Vec::new();
            for _ in 0..count {
                out.extend(inner.clone());
            }
            Ok(out)
        } else if inner.is_empty() {
            Ok(Vec::new())
        } else if inner
            .iter()
            .all(|step| matches!(step, Step::Hit(_) | Step::Chord(_)))
        {
            let mut chord_events: Vec<StepEvent> = Vec::new();
            for step in inner {
                match step {
                    Step::Hit(event) => chord_events.push(event),
                    Step::Chord(mut events) => chord_events.append(&mut events),
                    Step::Rest | Step::Tie => {
                        return Err(ParseError::InvalidChord { position: start });
                    }
                }
            }
            Ok(vec![Step::Chord(chord_events)])
        } else {
            Ok(inner)
        }
    }

    fn parse_hit(&mut self) -> Result<Step, ParseError> {
        let symbol = self.bump().expect("hit symbol consumed");
        let mut base = StepEvent::default();
        if symbol == 'X' {
            base.note.accent = true;
        }
        let mut chord_offsets: Option<Vec<i32>> = None;
        loop {
            self.skip_inline_ws();
            match self.peek() {
                Some('+') | Some('-') => {
                    let sign_char = self.bump().unwrap();
                    if sign_char == '+' && matches!(self.peek(), Some('(')) {
                        self.bump();
                        let offsets = self.parse_chord_offsets()?;
                        chord_offsets = Some(offsets);
                    } else {
                        let start = self.pos;
                        let digits = self.take_digits();
                        if digits.is_empty() {
                            return Err(ParseError::ExpectedNumber { position: start });
                        }
                        let value: i32 = digits
                            .parse()
                            .map_err(|_| ParseError::InvalidNumber { position: start })?;
                        match sign_char {
                            '+' => base.note.pitch_offset += value,
                            '-' => base.note.pitch_offset -= value,
                            _ => {}
                        }
                    }
                }
                Some('?') => {
                    self.bump();
                    let start = self.pos;
                    let number = self.take_number();
                    if number.is_empty() {
                        return Err(ParseError::ExpectedNumber { position: start });
                    }
                    let mut value: f32 = number
                        .parse()
                        .map_err(|_| ParseError::InvalidNumber { position: start })?;
                    if matches!(self.peek(), Some('%')) {
                        self.bump();
                        value /= 100.0;
                    }
                    base.note.probability = Some(value);
                }
                Some('v') => {
                    self.bump();
                    let start = self.pos;
                    let digits = self.take_digits();
                    if digits.is_empty() {
                        return Err(ParseError::ExpectedNumber { position: start });
                    }
                    let value: u32 = digits
                        .parse()
                        .map_err(|_| ParseError::InvalidNumber { position: start })?;
                    if value > 127 {
                        return Err(ParseError::InvalidNumber { position: start });
                    }
                    base.note.velocity = Some(value as u8);
                }
                Some('{') => {
                    self.bump();
                    let start = self.pos;
                    let digits = self.take_digits();
                    if digits.is_empty() {
                        return Err(ParseError::ExpectedNumber { position: start });
                    }
                    let value: u32 = digits
                        .parse()
                        .map_err(|_| ParseError::InvalidNumber { position: start })?;
                    self.expect('}')?;
                    base.ratchet = Some(value);
                }
                Some('@') => {
                    self.bump();
                    let mark = self.pos;
                    if let Some(cond) = self.try_parse_cycle()? {
                        base.note.cycle = Some(cond);
                    } else {
                        self.reset(mark);
                        let nudge = self.parse_nudge()?;
                        base.nudge = Some(nudge);
                    }
                }
                Some('=') => {
                    self.bump();
                    let gate = self.parse_gate()?;
                    base.gate = Some(gate);
                }
                Some('[') => {
                    let locks = self.parse_param_locks()?;
                    base.note.param_locks.extend(locks);
                }
                Some('#') => {
                    self.skip_comment();
                    break;
                }
                Some(c) if is_step_terminator(c) => break,
                None => break,
                Some(other) => {
                    return Err(ParseError::UnexpectedChar {
                        position: self.pos,
                        found: other,
                    });
                }
            }
        }
        if let Some(offsets) = chord_offsets {
            let mut unique_offsets = offsets;
            if !unique_offsets.iter().any(|&o| o == 0) {
                unique_offsets.insert(0, 0);
            }
            unique_offsets.sort_unstable();
            unique_offsets.dedup();
            let base_offset = base.note.pitch_offset;
            let mut events = Vec::new();
            for offset in unique_offsets {
                let mut event = base.clone();
                event.note.pitch_offset = base_offset + offset;
                events.push(event);
            }
            Ok(Step::Chord(events))
        } else {
            Ok(Step::Hit(base))
        }
    }

    fn parse_chord_offsets(&mut self) -> Result<Vec<i32>, ParseError> {
        let mut offsets = Vec::new();
        loop {
            self.skip_ws_and_comments();
            let sign = match self.peek() {
                Some('+') => {
                    self.bump();
                    1
                }
                Some('-') => {
                    self.bump();
                    -1
                }
                _ => 1,
            };
            let start = self.pos;
            let digits = self.take_digits();
            if digits.is_empty() {
                return Err(ParseError::ExpectedNumber { position: start });
            }
            let value: i32 = digits
                .parse()
                .map_err(|_| ParseError::InvalidNumber { position: start })?;
            offsets.push(sign * value);
            self.skip_ws_and_comments();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some(')') => {
                    self.bump();
                    break;
                }
                None => return Err(ParseError::UnexpectedEnd),
                Some(other) => {
                    return Err(ParseError::UnexpectedChar {
                        position: self.pos,
                        found: other,
                    });
                }
            }
        }
        Ok(offsets)
    }

    fn parse_gate(&mut self) -> Result<Gate, ParseError> {
        let start = self.pos;
        let digits = self.take_digits();
        if digits.is_empty() {
            // Maybe a float starting with '.'
            if matches!(self.peek(), Some('.')) {
                self.bump();
                let frac_digits = self.take_digits();
                if frac_digits.is_empty() {
                    return Err(ParseError::ExpectedNumber { position: start });
                }
                let value_str = format!("0.{}", frac_digits);
                let value: f32 = value_str
                    .parse()
                    .map_err(|_| ParseError::InvalidNumber { position: start })?;
                return Ok(Gate::Float(value));
            }
            return Err(ParseError::ExpectedNumber { position: start });
        }
        if matches!(self.peek(), Some('/')) {
            self.bump();
            let denom_start = self.pos;
            let denom_digits = self.take_digits();
            if denom_digits.is_empty() {
                return Err(ParseError::ExpectedNumber {
                    position: denom_start,
                });
            }
            let numerator: u32 = digits
                .parse()
                .map_err(|_| ParseError::InvalidNumber { position: start })?;
            let denominator: u32 = denom_digits
                .parse()
                .map_err(|_| ParseError::InvalidNumber {
                    position: denom_start,
                })?;
            Ok(Gate::Fraction {
                numerator,
                denominator,
            })
        } else if matches!(self.peek(), Some('%')) {
            self.bump();
            let numerator: f32 = digits
                .parse()
                .map_err(|_| ParseError::InvalidNumber { position: start })?;
            Ok(Gate::Percent(numerator / 100.0))
        } else if matches!(self.peek(), Some('.')) {
            self.bump();
            let frac_digits = self.take_digits();
            let value_str = format!("{}.{frac}", digits, frac = frac_digits);
            let value: f32 = value_str
                .parse()
                .map_err(|_| ParseError::InvalidNumber { position: start })?;
            Ok(Gate::Float(value))
        } else {
            let numerator: f32 = digits
                .parse()
                .map_err(|_| ParseError::InvalidNumber { position: start })?;
            Ok(Gate::Float(numerator))
        }
    }

    fn parse_param_locks(&mut self) -> Result<Vec<ParamLock>, ParseError> {
        self.expect('[')?;
        let mut locks = Vec::new();
        loop {
            self.skip_ws_and_comments();
            if matches!(self.peek(), Some(']')) {
                self.bump();
                break;
            }
            let key_start = self.pos;
            while let Some(c) = self.peek() {
                if matches!(c, '=' | ',' | ']') {
                    break;
                }
                if c == '\n' {
                    break;
                }
                self.bump();
            }
            let key_raw = self.src[key_start..self.pos].trim();
            if key_raw.is_empty() {
                return Err(ParseError::UnexpectedChar {
                    position: key_start,
                    found: ']',
                });
            }
            let mut value: Option<String> = None;
            self.skip_ws_and_comments();
            if matches!(self.peek(), Some('=')) {
                self.bump();
                self.skip_ws_and_comments();
                let value_start = self.pos;
                while let Some(c) = self.peek() {
                    if matches!(c, ',' | ']') {
                        break;
                    }
                    self.bump();
                }
                let raw = self.src[value_start..self.pos].trim();
                if !raw.is_empty() {
                    value = Some(raw.to_string());
                }
            }
            locks.push(ParamLock {
                key: key_raw.to_string(),
                value,
            });
            self.skip_ws_and_comments();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some(']') => {
                    self.bump();
                    break;
                }
                None => return Err(ParseError::UnexpectedEnd),
                Some(other) => {
                    return Err(ParseError::UnexpectedChar {
                        position: self.pos,
                        found: other,
                    });
                }
            }
        }
        Ok(locks)
    }

    fn parse_unsigned_int(&mut self) -> Result<u32, ParseError> {
        let start = self.pos;
        let digits = self.take_digits();
        if digits.is_empty() {
            return Err(ParseError::ExpectedNumber { position: start });
        }
        digits
            .parse()
            .map_err(|_| ParseError::InvalidNumber { position: start })
    }

    fn try_parse_cycle(&mut self) -> Result<Option<CycleCondition>, ParseError> {
        let mark = self.pos;
        let digits = self.take_digits();
        if digits.is_empty() {
            self.reset(mark);
            return Ok(None);
        }
        if !matches!(self.peek(), Some('/')) {
            self.reset(mark);
            return Ok(None);
        }
        self.bump();
        let denom_start = self.pos;
        let denom_digits = self.take_digits();
        if denom_digits.is_empty() {
            self.reset(mark);
            return Ok(None);
        }
        let hit: u32 = digits
            .parse()
            .map_err(|_| ParseError::InvalidNumber { position: mark })?;
        let of: u32 = denom_digits
            .parse()
            .map_err(|_| ParseError::InvalidNumber {
                position: denom_start,
            })?;
        Ok(Some(CycleCondition { hit, of }))
    }

    fn parse_nudge(&mut self) -> Result<Nudge, ParseError> {
        let start = self.pos;
        let sign = match self.peek() {
            Some('+') => {
                self.bump();
                1.0
            }
            Some('-') => {
                self.bump();
                -1.0
            }
            _ => 1.0,
        };
        let number_start = self.pos;
        let number_str = self.take_number();
        if number_str.is_empty() {
            return Err(ParseError::ExpectedNumber {
                position: number_start,
            });
        }
        let mut value: f32 = number_str.parse().map_err(|_| ParseError::InvalidNumber {
            position: number_start,
        })?;
        value *= sign;
        if self.src[self.pos..].starts_with("ms") {
            self.pos += 2;
            Ok(Nudge::Millis(value))
        } else if matches!(self.peek(), Some('%')) {
            self.bump();
            Ok(Nudge::Percent(value))
        } else {
            Err(ParseError::UnexpectedChar {
                position: start,
                found: self.peek().unwrap_or('\0'),
            })
        }
    }

    fn take_digits(&mut self) -> &'a str {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.bump();
            } else {
                break;
            }
        }
        &self.src[start..self.pos]
    }

    fn take_number(&mut self) -> &'a str {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.bump();
            } else {
                break;
            }
        }
        if matches!(self.peek(), Some('.')) {
            self.bump();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        &self.src[start..self.pos]
    }

    fn skip_inline_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c == '\t') {
            self.bump();
        }
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while matches!(self.peek(), Some(c) if c.is_whitespace()) {
                self.bump();
            }
            if matches!(self.peek(), Some('#')) {
                self.skip_comment();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        while let Some(c) = self.bump() {
            if c == '\n' {
                break;
            }
        }
    }

    fn expect(&mut self, ch: char) -> Result<(), ParseError> {
        match self.bump() {
            Some(c) if c == ch => Ok(()),
            Some(found) => Err(ParseError::UnexpectedChar {
                position: self.pos - found.len_utf8(),
                found,
            }),
            None => Err(ParseError::UnexpectedEnd),
        }
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn reset(&mut self, pos: usize) {
        self.pos = pos;
    }
}

fn is_hit_symbol(c: char) -> bool {
    matches!(c, 'x' | 'X' | '1' | '*')
}

fn is_step_terminator(c: char) -> bool {
    matches!(
        c,
        ' ' | '\n' | '\r' | '\t' | '|' | '.' | '_' | '(' | ')' | 'x' | 'X' | '1' | '*'
    )
}
