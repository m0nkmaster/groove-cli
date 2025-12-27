//! Minimal, TUI-safe logging.
//!
//! Any stdout/stderr writes while the TUI is running will corrupt the screen.
//! This module provides a tiny publish/subscribe mechanism so background
//! threads can report warnings/errors without printing directly.

use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogMessage {
    pub level: Level,
    pub text: String,
}

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
static SUBSCRIBERS: Lazy<Mutex<Vec<(usize, Sender<LogMessage>)>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

/// A subscription to console log messages.
///
/// Dropping this value unsubscribes it.
pub struct Subscription {
    id: usize,
    rx: Receiver<LogMessage>,
}

impl Subscription {
    pub fn drain(&self) -> Vec<LogMessage> {
        self.rx.try_iter().collect()
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let mut subs = SUBSCRIBERS.lock().unwrap();
        subs.retain(|(id, _)| *id != self.id);
    }
}

pub fn subscribe() -> Subscription {
    let (tx, rx) = mpsc::channel();
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    SUBSCRIBERS.lock().unwrap().push((id, tx));
    Subscription { id, rx }
}

pub fn warn(msg: impl Into<String>) {
    publish(Level::Warn, msg.into());
}

pub fn error(msg: impl Into<String>) {
    publish(Level::Error, msg.into());
}

fn publish(level: Level, text: String) {
    let message = LogMessage { level, text };

    let mut subs = SUBSCRIBERS.lock().unwrap();
    if subs.is_empty() {
        // Outside the TUI, warnings/errors should still be visible.
        match message.level {
            Level::Warn | Level::Error => eprintln!("{}", message.text),
            Level::Info => {}
        }
        return;
    }

    // Broadcast to all subscribers; drop any that have gone away.
    subs.retain(|(_, tx)| tx.send(message.clone()).is_ok());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_receives_warn_messages() {
        let sub = subscribe();
        warn("hello");

        let msgs = sub.drain();
        assert!(msgs.iter().any(|m| m.level == Level::Warn && m.text == "hello"));
    }
}


