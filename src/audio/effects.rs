//! Audio effects for the groove-cli audio engine.

use rodio::Source;
use std::collections::VecDeque;
use std::time::Duration;

/// A simple tempo-synced delay effect that wraps a Source.
/// Uses a circular buffer to store and replay delayed samples.
pub struct DelayEffect<S: Source<Item = f32>> {
    source: S,
    buffer: VecDeque<f32>,
    delay_samples: usize,
    feedback: f32,
    mix: f32,
    sample_rate: u32,
    channels: u16,
}

impl<S: Source<Item = f32>> DelayEffect<S> {
    /// Create a new delay effect.
    /// - `source`: The audio source to apply delay to
    /// - `delay_time`: Delay time in seconds
    /// - `feedback`: Amount of delayed signal fed back (0.0 - 1.0)
    /// - `mix`: Wet/dry mix (0.0 = dry, 1.0 = wet only)
    pub fn new(source: S, delay_time: Duration, feedback: f32, mix: f32) -> Self {
        let sample_rate = source.sample_rate();
        let channels = source.channels();
        let delay_samples = (delay_time.as_secs_f64() * sample_rate as f64 * channels as f64) as usize;
        let delay_samples = delay_samples.max(1);
        
        Self {
            source,
            buffer: VecDeque::from(vec![0.0; delay_samples]),
            delay_samples,
            feedback: feedback.clamp(0.0, 0.95), // Limit feedback to avoid infinite buildup
            mix: mix.clamp(0.0, 1.0),
            sample_rate,
            channels,
        }
    }
}

impl<S: Source<Item = f32>> Iterator for DelayEffect<S> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let input = self.source.next()?;
        
        // Read delayed sample from buffer
        let delayed = self.buffer.front().copied().unwrap_or(0.0);
        
        // Calculate output with wet/dry mix
        let output = input * (1.0 - self.mix) + delayed * self.mix;
        
        // Write input + feedback to buffer
        let to_buffer = input + delayed * self.feedback;
        
        // Remove oldest sample and add new one
        self.buffer.pop_front();
        self.buffer.push_back(to_buffer);
        
        Some(output)
    }
}

impl<S: Source<Item = f32>> Source for DelayEffect<S> {
    fn current_frame_len(&self) -> Option<usize> {
        self.source.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        // Add tail time for delay to fade out
        self.source.total_duration().map(|d| {
            d + Duration::from_secs_f64(self.delay_samples as f64 / self.sample_rate as f64)
        })
    }
}

/// Parse delay time string (e.g., "1/4", "1/8", "100ms") to Duration at given BPM.
pub fn parse_delay_time(time_str: &str, bpm: u32) -> Duration {
    let time_str = time_str.trim();
    
    // Handle milliseconds format: "100ms"
    if time_str.ends_with("ms") {
        if let Ok(ms) = time_str.trim_end_matches("ms").parse::<f64>() {
            return Duration::from_secs_f64(ms / 1000.0);
        }
    }
    
    // Handle fraction format: "1/4", "1/8", "3/16"
    if let Some((num, denom)) = time_str.split_once('/') {
        if let (Ok(n), Ok(d)) = (num.trim().parse::<u32>(), denom.trim().parse::<u32>()) {
            if d > 0 && bpm > 0 {
                // One beat = 60/bpm seconds
                // Fraction n/d of a whole note (4 beats) = n/d * 4 beats
                let beats = (n as f64 / d as f64) * 4.0;
                let seconds = beats * 60.0 / bpm as f64;
                return Duration::from_secs_f64(seconds);
            }
        }
    }
    
    // Default: 1/4 note at given BPM
    Duration::from_secs_f64(60.0 / bpm as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_delay_time_quarter_note() {
        let delay = parse_delay_time("1/4", 120);
        // At 120 BPM, 1 beat = 0.5s, 1/4 note = 1 beat = 0.5s
        assert!((delay.as_secs_f64() - 0.5).abs() < 0.001);
    }

    #[test]
    fn parse_delay_time_eighth_note() {
        let delay = parse_delay_time("1/8", 120);
        // At 120 BPM, 1/8 note = 0.5 beat = 0.25s
        assert!((delay.as_secs_f64() - 0.25).abs() < 0.001);
    }

    #[test]
    fn parse_delay_time_milliseconds() {
        let delay = parse_delay_time("100ms", 120);
        assert!((delay.as_secs_f64() - 0.1).abs() < 0.001);
    }

    #[test]
    fn parse_delay_time_dotted_quarter() {
        let delay = parse_delay_time("3/8", 120);
        // 3/8 of whole note = 1.5 beats = 0.75s at 120 BPM
        assert!((delay.as_secs_f64() - 0.75).abs() < 0.001);
    }

    #[test]
    fn delay_effect_preserves_channels_and_sample_rate() {
        use rodio::source::SineWave;
        let source = SineWave::new(440.0);
        let delayed = DelayEffect::new(source, Duration::from_millis(100), 0.5, 0.5);
        assert_eq!(delayed.channels(), 1);
        assert_eq!(delayed.sample_rate(), 48000);
    }
}

