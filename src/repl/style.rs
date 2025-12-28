//! Output styling for the groove-cli REPL.
//! 
//! Provides emoji, colors, and unicode formatting for beautiful output.

// Emoji constants
pub const EMOJI_PLAY: &str = "▶";
pub const EMOJI_STOP: &str = "⏹";
pub const EMOJI_MUTE: &str = "🔇";
pub const EMOJI_UNMUTE: &str = "🔊";
pub const EMOJI_SOLO: &str = "🎤";
pub const EMOJI_GAIN: &str = "🎚";
pub const EMOJI_DELAY: &str = "🔁";
pub const EMOJI_DICE: &str = "🎲";
pub const EMOJI_SAVE: &str = "💾";
pub const EMOJI_OPEN: &str = "📂";
pub const EMOJI_CHECK: &str = "✓";
pub const EMOJI_THINK: &str = "🤔";
pub const EMOJI_SEARCH: &str = "🔍";
pub const EMOJI_WAVE: &str = "👋";
pub const EMOJI_EYE: &str = "👁";
pub const EMOJI_NOTE: &str = "♪";
pub const EMOJI_ARROW: &str = "→";

// Pattern display characters
pub const PATTERN_HIT: char = '●';
pub const PATTERN_REST: char = '·';
pub const PATTERN_ACCENT: char = '◉';

/// Convert a visual pattern (x...) to pretty display (●···)
pub fn prettify_pattern(pattern: &str) -> String {
    // Keep formatting (whitespace, separators, modifiers) but replace hit/rest glyphs
    // with prettier symbols for display.
    pattern
        .chars()
        .map(|c| match c {
            'x' | '1' | '*' => PATTERN_HIT,
            'X' => PATTERN_ACCENT,
            '.' => PATTERN_REST,
            '_' => '─',
            other => other,
        })
        .collect()
}

/// Format the REPL prompt with tempo and transport state
pub fn format_prompt(bpm: u32, playing: bool) -> String {
    let transport = if playing { EMOJI_PLAY } else { EMOJI_STOP };
    format!("{} {} {} › ", EMOJI_NOTE, bpm, transport)
}

/// Format a success message
pub fn success(msg: &str) -> String {
    format!("  {} {}", EMOJI_CHECK, msg)
}

/// Format a track pattern output (shows raw pattern and visual)
pub fn track_pattern(name: &str, pattern: &str) -> String {
    format!("  {}  {}  {}", name, pattern, prettify_pattern(pattern))
}

/// Format a track sample output
pub fn track_sample(name: &str, path: &str, root: Option<crate::model::track::SampleRoot>) -> String {
    let mut msg = format!("  {}  {} {}", name, EMOJI_UNMUTE, path);
    if let Some(r) = root {
        let note = crate::audio::pitch::midi_note_to_display(r.midi_note);
        msg.push_str(&format!(
            "  (root: {} {:.1}Hz {:+.1}c conf:{:.2})",
            note, r.freq_hz, r.cents, r.confidence
        ));
    }
    msg
}

/// Format a track mute output
pub fn track_muted(name: &str) -> String {
    format!("  {}  {} muted", name, EMOJI_MUTE)
}

/// Format a track unmute output  
pub fn track_unmuted(name: &str) -> String {
    format!("  {}  {} unmuted", name, EMOJI_UNMUTE)
}

/// Format a track solo output
pub fn track_solo(name: &str, on: bool) -> String {
    if on {
        format!("  {}  {} solo", name, EMOJI_SOLO)
    } else {
        format!("  {}  solo off", name)
    }
}

/// Format a track gain output
pub fn track_gain(name: &str, db: f32) -> String {
    format!("  {}  {} {:+.1}db", name, EMOJI_GAIN, db)
}

/// Format a track delay output
pub fn track_delay(name: &str, on: bool, time: Option<&str>, feedback: Option<f32>, mix: Option<f32>) -> String {
    if !on {
        return format!("  {}  {} delay off", name, EMOJI_DELAY);
    }
    match (time, feedback, mix) {
        (Some(t), Some(fb), Some(m)) => {
            format!("  {}  {} delay {} fb:{:.2} mix:{:.2}", name, EMOJI_DELAY, t, fb, m)
        }
        _ => format!("  {}  {} delay on", name, EMOJI_DELAY),
    }
}

/// Format a track variation switch
pub fn track_variation(name: &str, var: &str) -> String {
    format!("  {}  {} {}", name, EMOJI_ARROW, var)
}

/// Format a generated pattern
pub fn track_generated(name: &str, pattern: &str) -> String {
    format!("  {}  {} {}", name, EMOJI_DICE, prettify_pattern(pattern))
}

/// Format play message
pub fn playing() -> String {
    format!("  {} playing", EMOJI_PLAY)
}

/// Format stop message
pub fn stopped() -> String {
    format!("  {} stopped", EMOJI_STOP)
}

/// Format tempo change
pub fn tempo(bpm: u32) -> String {
    format!("  {} {}", EMOJI_NOTE, bpm)
}

/// Format save message
pub fn saved(path: &str) -> String {
    format!("  {} saved {}", EMOJI_SAVE, path)
}

/// Format open message
pub fn opened(path: &str) -> String {
    format!("  {} opened {}", EMOJI_OPEN, path)
}

/// Format goodbye message
pub fn goodbye() -> String {
    format!("{} bye", EMOJI_WAVE)
}

/// Format live view toggle
pub fn live_view(on: bool) -> String {
    if on {
        format!("  {} live view on", EMOJI_EYE)
    } else {
        format!("  {} live view off", EMOJI_EYE)
    }
}

/// Format a "not found" error with suggestions
#[allow(dead_code)]
pub fn not_found_track(name: &str, available: &[String]) -> String {
    let mut msg = format!("  {} no track \"{}\"\n", EMOJI_THINK, name);
    if !available.is_empty() {
        msg.push_str("\n     your tracks: ");
        msg.push_str(&available.join(", "));
    }
    msg.push_str("\n     or use: + ");
    msg.push_str(name);
    msg
}

/// Format a sample not found error with suggestions
pub fn not_found_sample(query: &str, suggestions: &[String]) -> String {
    let mut msg = format!("  {} couldn't find \"{}\"\n", EMOJI_SEARCH, query);
    if !suggestions.is_empty() {
        msg.push_str("\n     did you mean?");
        for s in suggestions.iter().take(5) {
            msg.push_str(&format!("\n     {} {}", EMOJI_ARROW, s));
        }
    }
    msg
}

/// Box drawing characters for help
pub const BOX_TL: &str = "╭";
pub const BOX_TR: &str = "╮";
pub const BOX_BL: &str = "╰";
pub const BOX_BR: &str = "╯";
pub const BOX_H: &str = "─";
pub const BOX_V: &str = "│";
pub const BOX_T: &str = "├";
pub const BOX_CROSS: &str = "┤";

/// Format the help text in a styled box
pub fn help_box() -> String {
    let width = 52;
    let h_line: String = BOX_H.repeat(width);
    
    format!(r#"
{tl}{h}{tr}
{v}  GROOVE                                            {v}
{t}{h}{c}
{v}                                                    {v}
{v}  TRANSPORT                                         {v}
{v}    go, play      play                              {v}
{v}    >             play                              {v}
{v}    . , stop      stop                              {v}
{v}    <             play                              {v}
{v}    120           set tempo                         {v}
{v}                                                    {v}
{v}  TRACKS                                            {v}
{v}    + a b c       add track(s)                      {v}
{v}    - name        remove track                      {v}
{v}    list, ls      show all                          {v}
{v}                                                    {v}
{v}  TRACK COMMANDS                                    {v}
{v}    kick x...     set pattern                       {v}
{v}    kick ~ path   set sample                        {v}
{v}    analyze kick  detect sample root note           {v}
{v}    root kick c4  set root note manually            {v}
{v}    * > chorus    wildcard variation switch         {v}
{v}    macro m "..." define macro                      {v}
{v}    show kick [var] show pattern                    {v}
{v}    kick x... ~q -3db  chain actions                {v}
{v}    kick -3db     gain                              {v}
{v}    kick mute     mute (unmute to undo)             {v}
{v}    kick solo     toggle solo                       {v}
{v}    kick.var x    set variation                     {v}
{v}    kick > var    switch variation                  {v}
{v}    kick gen expr generate (euclid, random, etc)    {v}
{v}    kick ai "..." AI pattern                        {v}
{v}                                                    {v}
{v}  PROGRESSIONS                                      {v}
{v}    prog kick "C Am F G"  chord pattern             {v}
{v}    notes kick    show resolved notes               {v}
{v}                                                    {v}
{v}  DELAY                                             {v}
{v}    kick delay on/off                               {v}
{v}    kick delay 1/8 0.4 0.3   (time, fb, mix)        {v}
{v}                                                    {v}
{v}  FILES                                             {v}
{v}    save file     save song                         {v}
{v}    open file     load song                         {v}
{v}                                                    {v}
{v}  :live on/off    playhead view                     {v}
{v}  :q              quit                              {v}
{bl}{h}{br}
"#,
        tl = BOX_TL, tr = BOX_TR, bl = BOX_BL, br = BOX_BR,
        v = BOX_V, t = BOX_T, c = BOX_CROSS, h = h_line
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prettify_pattern_converts_chars() {
        assert_eq!(prettify_pattern("x...x..."), "●···●···");
        assert_eq!(prettify_pattern("X.x."), "◉·●·");
        assert_eq!(prettify_pattern("x x"), "● ●");
    }

    #[test]
    fn format_prompt_shows_state() {
        let prompt = format_prompt(120, false);
        assert!(prompt.contains("120"));
        assert!(prompt.contains(EMOJI_STOP));
        
        let prompt_playing = format_prompt(85, true);
        assert!(prompt_playing.contains("85"));
        assert!(prompt_playing.contains(EMOJI_PLAY));
    }
}
