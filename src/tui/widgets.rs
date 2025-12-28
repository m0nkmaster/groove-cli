//! Custom TUI widgets for groove-cli.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Widget},
};

use crate::audio;
use crate::model::pattern::Pattern;
use crate::model::song::Song;
use crate::pattern::visual::{parse_visual_pattern, Step};

/// The tracker grid widget
pub struct TrackerGrid<'a> {
    song: &'a Song,
    block: Option<Block<'a>>,
    snapshot: Option<audio::LiveSnapshot>,
    selected_track: Option<String>,
}

impl<'a> TrackerGrid<'a> {
    pub fn new(song: &'a Song) -> Self {
        Self {
            song,
            block: None,
            snapshot: None,
            selected_track: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn snapshot(mut self, snapshot: Option<audio::LiveSnapshot>) -> Self {
        self.snapshot = snapshot;
        self
    }

    pub fn selected_track(mut self, track_name: Option<String>) -> Self {
        self.selected_track = track_name;
        self
    }

    fn parse_steps(track: &crate::model::track::Track) -> Vec<Step> {
        match track.active_pattern() {
            Some(Pattern::Visual(s)) => parse_visual_pattern(s).unwrap_or_default(),
            None => Vec::new(),
        }
    }
}

impl Widget for TrackerGrid<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let TrackerGrid {
            song,
            block,
            snapshot,
            selected_track,
        } = self;
        let snapshot = snapshot.or_else(|| audio::snapshot_live_state());
        let mut area = area;
        if let Some(block) = block {
            let inner = block.inner(area);
            block.render(area, buf);
            area = inner;
        }

        if area.height < 1 {
            return;
        }

        // Column widths
        let col_num = 3usize;
        let col_name = 10usize;
        let col_sample = 14usize;
        let col_note = 10usize;
        let col_vars = 18usize;
        let col_len = 4usize;
        let prefix_width =
            col_num + 1 + col_name + 1 + col_sample + 1 + col_note + 1 + col_vars + 1 + col_len + 2;
        if area.width as usize <= prefix_width + 4 {
            // Too narrow to show the grid cleanly.
            let y = area.y;
            for x in area.x..area.x + area.width {
                buf[(x, y)].set_char(' ').set_style(Style::default());
            }
            buf.set_string(
                area.x,
                y,
                "terminal too narrow — widen to view tracks",
                Style::default().fg(Color::DarkGray),
            );
            return;
        }
        
        let grid_start_x = area.x + prefix_width as u16;
        let grid_width = (area.width as usize).saturating_sub(prefix_width + 1);
        
        let dim = Style::default().fg(Color::DarkGray);
        let header_style = Style::default().fg(Color::Rgb(100, 100, 100));

        // === HEADER ROW ===
        let y = area.y;
        // Clear header row
        for x in area.x..area.x + area.width {
            buf[(x, y)].set_char(' ').set_style(Style::default());
        }
        
        buf.set_string(area.x, y, &format!("{:>w$}", "#", w = col_num), header_style);
        buf.set_string(area.x + (col_num + 1) as u16, y, &format!("{:<w$}", "TRACK", w = col_name), header_style);
        buf.set_string(area.x + (col_num + 1 + col_name + 1) as u16, y, &format!("{:<w$}", "SAMPLE", w = col_sample), header_style);
        buf.set_string(area.x + (col_num + 1 + col_name + 1 + col_sample + 1) as u16, y, &format!("{:<w$}", "NOTE", w = col_note), header_style);
        buf.set_string(
            area.x + (col_num + 1 + col_name + 1 + col_sample + 1 + col_note + 1) as u16,
            y,
            &format!("{:<w$}", "VARS", w = col_vars),
            header_style,
        );
        buf.set_string(
            area.x + (col_num + 1 + col_name + 1 + col_sample + 1 + col_note + 1 + col_vars + 1) as u16,
            y,
            &format!("{:>w$}", "LEN", w = col_len),
            header_style,
        );
        
        // Beat markers - use song.steps as reference
        let max_display_steps = grid_width.min(song.steps as usize);
        for step in 0..max_display_steps {
            let x = grid_start_x + step as u16;
            if x >= area.x + area.width {
                break;
            }
            if step % 4 == 0 {
                buf.set_string(x, y, &format!("{}", step + 1), dim);
            }
        }

        // === EMPTY STATE ===
        if song.tracks.is_empty() {
            let y = area.y + 1;
            if y < area.y + area.height {
                for x in area.x..area.x + area.width {
                    buf[(x, y)].set_char(' ').set_style(Style::default());
                }
                buf.set_string(area.x + 2, y, "No tracks. Type: + trackname", dim);
            }
            return;
        }

        // === TRACK ROWS ===
        for (i, track) in song.tracks.iter().enumerate() {
            let y = area.y + 1 + i as u16;
            if y >= area.y + area.height {
                break;
            }
            let is_selected = selected_track
                .as_deref()
                .map(|s| s.eq_ignore_ascii_case(&track.name))
                .unwrap_or(false);

            // Clear entire row first
            for x in area.x..area.x + area.width {
                buf[(x, y)].set_char(' ').set_style(Style::default());
            }

            // Track number
            let num_style = if track.mute {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::Rgb(80, 80, 80))
            };
            buf.set_string(area.x, y, &format!("{:>w$}", i + 1, w = col_num), num_style);

            // Track name
            let name_style = if track.mute {
                Style::default().fg(Color::DarkGray)
            } else if track.solo {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let name_style = if is_selected {
                name_style.add_modifier(Modifier::REVERSED)
            } else {
                name_style
            };
            let name = if track.name.len() > col_name { &track.name[..col_name] } else { &track.name };
            buf.set_string(area.x + (col_num + 1) as u16, y, &format!("{:<w$}", name, w = col_name), name_style);

            // Sample name
            let sample = track.sample.as_ref()
                .and_then(|s| std::path::Path::new(s).file_stem())
                .and_then(|f| f.to_str())
                .unwrap_or("—");
            let sample_disp = if sample.len() > col_sample { &sample[..col_sample] } else { sample };
            let sample_style = if track.sample.is_some() {
                Style::default().fg(Color::Rgb(70, 130, 180))
            } else {
                dim
            };
            buf.set_string(area.x + (col_num + 1 + col_name + 1) as u16, y, &format!("{:<w$}", sample_disp, w = col_sample), sample_style);

            // Root note (detected or manually set)
            let note = track
                .sample_root
                .map(|r| crate::audio::pitch::midi_note_to_display(r.midi_note))
                .unwrap_or_else(|| "—".to_string());
            let note_disp = if note.len() > col_note { &note[..col_note] } else { &note };
            let note_style = if track.sample_root.is_some() {
                Style::default().fg(Color::Rgb(140, 200, 140))
            } else {
                dim
            };
            buf.set_string(
                area.x + (col_num + 1 + col_name + 1 + col_sample + 1) as u16,
                y,
                &format!("{:<w$}", note_disp, w = col_note),
                note_style,
            );

            // Variations
            let current = track.current_variation.as_deref().unwrap_or("main");
            let mut keys: Vec<String> = track.variations.keys().cloned().collect();
            keys.sort();
            let vars_disp = std::iter::once("main".to_string())
                .chain(keys.into_iter())
                .map(|v| {
                    let up = v.to_uppercase();
                    if v == current {
                        format!("[{}]", up)
                    } else {
                        up
                    }
                })
                .collect::<Vec<_>>()
                .join("|");
            let vars_disp = if vars_disp.len() > col_vars {
                &vars_disp[..col_vars]
            } else {
                &vars_disp
            };
            let vars_style = if track.current_variation.is_some() {
                Style::default().fg(Color::Rgb(200, 160, 220))
            } else {
                dim
            };
            buf.set_string(
                area.x + (col_num + 1 + col_name + 1 + col_sample + 1 + col_note + 1) as u16,
                y,
                &format!("{:<w$}", vars_disp, w = col_vars),
                vars_style,
            );

            // Pattern length
            let steps = Self::parse_steps(track);
            let step_count = steps.len();
            let len_str = if step_count > 0 { format!("{}", step_count) } else { "—".to_string() };
            buf.set_string(
                area.x + (col_num + 1 + col_name + 1 + col_sample + 1 + col_note + 1 + col_vars + 1) as u16,
                y,
                &format!("{:>w$}", len_str, w = col_len),
                dim,
            );

            // Pattern grid
            if step_count == 0 {
                continue;
            }

            // Use per-track playhead (token index) from the audio engine so `div` is reflected.
            let track_playhead = snapshot
                .as_ref()
                .and_then(|s| {
                    s.tracks
                        .iter()
                        .find(|t| t.name.eq_ignore_ascii_case(&track.name))
                        .map(|t| t.token_index)
                })
                .map(|p| p % step_count);
            let steps_to_show = step_count.min(grid_width);

            for step_idx in 0..steps_to_show {
                let x = grid_start_x + step_idx as u16;
                if x >= area.x + area.width {
                    break;
                }

                let is_playhead = track_playhead == Some(step_idx);
                
                let (ch, is_hit) = match &steps[step_idx] {
                    Step::Hit(_) | Step::Chord(_) => ('●', true),
                    Step::Rest => ('·', false),
                    Step::Tie => ('─', false),
                };

                let style = if is_playhead {
                    if is_hit {
                        Style::default().fg(Color::Black).bg(Color::Rgb(100, 180, 100)).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray).bg(Color::Rgb(60, 100, 60))
                    }
                } else if track.mute {
                    Style::default().fg(Color::Rgb(50, 50, 50))
                } else if is_hit {
                    Style::default().fg(Color::Rgb(80, 160, 220))
                } else {
                    Style::default().fg(Color::Rgb(50, 50, 50))
                };

                buf.set_string(x, y, &ch.to_string(), style);
            }

            // Overflow indicator
            if step_count > steps_to_show {
                let x = grid_start_x + steps_to_show as u16;
                if x < area.x + area.width {
                    buf.set_string(x, y, "…", dim);
                }
            }

            // Status flags
            let flag_x = area.x + area.width - 4;
            if flag_x > grid_start_x {
                let mut flags = String::new();
                if track.mute { flags.push('M'); }
                if track.solo { flags.push('S'); }
                if track.delay.on { flags.push('D'); }
                if !flags.is_empty() {
                    buf.set_string(flag_x, y, &flags, Style::default().fg(Color::Rgb(70, 70, 70)));
                }
            }
        }

        // Clear any remaining rows (in case tracks were removed)
        let tracks_end_y = area.y + 1 + song.tracks.len() as u16;
        for y in tracks_end_y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf[(x, y)].set_char(' ').set_style(Style::default());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::widgets::{Borders};

    use crate::audio::{LiveSnapshot, LiveTrackSnapshot};
    use crate::model::pattern::Pattern;
    use crate::model::song::Song;
    use crate::model::track::Track;

    fn row(buf: &Buffer, y: u16) -> String {
        let mut out = String::new();
        for x in 0..buf.area.width {
            out.push_str(buf[(buf.area.x + x, buf.area.y + y)].symbol());
        }
        out
    }

    #[test]
    fn tracker_grid_respects_block_border() {
        let song = Song::default();
        let area = Rect { x: 0, y: 0, width: 48, height: 6 };
        let mut buf = Buffer::empty(area);

        let block = Block::default().borders(Borders::ALL).title(" Tracks ");
        TrackerGrid::new(&song).block(block).render(area, &mut buf);

        assert_eq!(buf[(0, 0)].symbol(), "┌");
        assert_eq!(buf[(area.width - 1, 0)].symbol(), "┐");
        assert_eq!(buf[(0, area.height - 1)].symbol(), "└");
        assert_eq!(buf[(area.width - 1, area.height - 1)].symbol(), "┘");
    }

    #[test]
    fn tracker_grid_shows_narrow_width_message_instead_of_panicking() {
        let mut song = Song::default();
        let mut kick = Track::new("Kick");
        kick.pattern = Some(Pattern::visual("x...x...x...x..."));
        song.tracks.push(kick);

        let area = Rect { x: 0, y: 0, width: 20, height: 5 };
        let mut buf = Buffer::empty(area);
        let block = Block::default().borders(Borders::ALL).title(" Tracks ");

        TrackerGrid::new(&song).block(block).render(area, &mut buf);

        let line = row(&buf, 1);
        assert!(line.contains("terminal too narrow"), "got line: {line}");
    }

    #[test]
    fn playhead_uses_per_track_token_index_from_snapshot() {
        let mut song = Song::default();
        let mut tr = Track::new("Synth2");
        tr.pattern = Some(Pattern::visual("..x."));
        song.tracks.push(tr);

        // Intentionally mismatch global_step and token_index:
        // old behavior used global_step % len (=> 1), new behavior should use token_index (=> 2).
        let snap = LiveSnapshot {
            tracks: vec![LiveTrackSnapshot {
                name: "Synth2".into(),
                token_index: 2,
                pattern: vec![false, false, true, false],
            }],
            global_step: 1,
        };

        let area = Rect { x: 0, y: 0, width: 80, height: 3 };
        let mut buf = Buffer::empty(area);

        TrackerGrid::new(&song)
            .snapshot(Some(snap))
            .render(area, &mut buf);

        let y = 1u16; // first track row (header is y=0)
        let prefix_width = 3 + 1 + 10 + 1 + 14 + 1 + 10 + 1 + 18 + 1 + 4 + 2;
        let grid_start_x = prefix_width as u16;

        let cell_expected = &buf[(grid_start_x + 2, y)];
        let cell_wrong = &buf[(grid_start_x + 1, y)];

        assert_ne!(cell_expected.bg, Color::Reset, "expected playhead bg at token_index=2");
        assert_eq!(cell_wrong.bg, Color::Reset, "did not expect playhead bg at global_step=1");
    }

    #[test]
    fn tracker_grid_shows_variations_header() {
        let mut song = Song::default();
        let mut tr = Track::new("Synth");
        tr.variations.insert("chorus".into(), Pattern::visual("c d e"));
        song.tracks.push(tr);

        let area = Rect { x: 0, y: 0, width: 100, height: 4 };
        let mut buf = Buffer::empty(area);

        TrackerGrid::new(&song).render(area, &mut buf);

        let header = row(&buf, 0);
        assert!(header.contains("VARS"), "header row was: {header}");
    }

    #[test]
    fn tracker_grid_reverses_selected_track_name() {
        let mut song = Song::default();
        let mut a = Track::new("Kick");
        a.pattern = Some(Pattern::visual("x..."));
        let mut b = Track::new("Synth");
        b.pattern = Some(Pattern::visual("c d e"));
        song.tracks.push(a);
        song.tracks.push(b);

        let area = Rect { x: 0, y: 0, width: 120, height: 5 };
        let mut buf = Buffer::empty(area);
        TrackerGrid::new(&song)
            .selected_track(Some("Synth".into()))
            .render(area, &mut buf);

        // Second track row is y=2 (header y=0, first track y=1).
        let y = 2u16;
        let name_start_x = (3 + 1) as u16;
        let cell = &buf[(name_start_x, y)];
        assert!(
            cell.modifier.contains(Modifier::REVERSED),
            "expected selected track name to be reversed"
        );
    }
}
