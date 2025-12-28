//! TUI (Terminal User Interface) for groove-cli.
//!
//! Tracker-style interface with live playhead and command input.

mod input;
mod widgets;

use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::audio;
use crate::console::{self, Level};
use crate::model::song::Song;
use crate::repl;

use input::InputLine;
use widgets::TrackerGrid;

struct Message {
    text: String,
    level: Level,
}

struct CompletionCycle {
    prefix: String,
    suffix: String,
    candidates: Vec<crate::repl::completer::Completion>,
    idx: usize,
}

pub struct TuiApp {
    song: Song,
    input: InputLine,
    messages: Vec<Message>,
    console: console::Subscription,
    should_quit: bool,
    completion_cycle: Option<CompletionCycle>,
}

impl TuiApp {
    pub fn new(song: Song) -> Self {
        Self {
            song,
            input: InputLine::new(),
            messages: Vec::new(),
            console: console::subscribe(),
            should_quit: false,
            completion_cycle: None,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let mut terminal = setup_terminal()?;
        let result = self.event_loop(&mut terminal);
        restore_terminal(&mut terminal)?;
        result
    }

    fn event_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        let tick = Duration::from_millis(33); // ~30fps
        let mut last = Instant::now();

        loop {
            for log in self.console.drain() {
                self.msg(&log.text, log.level);
            }

            terminal.draw(|f| self.render(f))?;

            let timeout = tick.saturating_sub(last.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key.code, key.modifiers);
                }
            }

            if last.elapsed() >= tick {
                last = Instant::now();
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) {
        if mods.contains(KeyModifiers::CONTROL) {
            match code {
                KeyCode::Char('c') | KeyCode::Char('d') => {
                    self.should_quit = true;
                    return;
                }
                _ => {}
            }
        }

        // Any non-Tab key cancels the current completion cycle.
        if !matches!(code, KeyCode::Tab) {
            self.completion_cycle = None;
        }

        match code {
            KeyCode::Enter => {
                let line = self.input.submit();
                if !line.is_empty() {
                    self.execute(&line);
                }
            }
            KeyCode::Char(c) => self.input.insert(c),
            KeyCode::Backspace => self.input.backspace(),
            KeyCode::Delete => self.input.delete(),
            KeyCode::Left => self.input.move_left(),
            KeyCode::Right => self.input.move_right(),
            KeyCode::Up => self.input.history_prev(),
            KeyCode::Down => self.input.history_next(),
            KeyCode::Home => self.input.move_home(),
            KeyCode::End => self.input.move_end(),
            KeyCode::Tab => self.tab_complete(),
            KeyCode::Esc => self.input.clear(),
            _ => {}
        }
    }

    fn execute(&mut self, line: &str) {
        if line == ":q" || line == ":quit" || line == "exit" || line == "quit" {
            self.should_quit = true;
            return;
        }

        if line.trim().starts_with("browse") {
            self.msg("Use Tab completion: track ~ query<Tab>", Level::Info);
            return;
        }

        if line.trim().starts_with("samples") {
            let q = line.trim().strip_prefix("samples").unwrap_or("").trim();
            self.search_samples(q);
            return;
        }

        match repl::handle_line_for_tui(&mut self.song, line) {
            Ok(Some(out)) => self.msg(&out, Level::Info),
            Ok(None) => {}
            Err(e) => self.msg(&e.to_string(), Level::Error),
        }
    }

    fn tab_complete(&mut self) {
        use crate::repl::completer::complete_for_tui;
        
        // If we already have a completion cycle, advance it without recomputing.
        if let Some(mut state) = self.completion_cycle.take() {
            if state.candidates.is_empty() {
                return;
            }
            state.idx = (state.idx + 1) % state.candidates.len();
            let c = &state.candidates[state.idx];
            let new_line = format!("{}{}{}", state.prefix, c.replacement, state.suffix);
            let msg = format!("▶ {} ({}/{})", c.display, state.idx + 1, state.candidates.len());
            self.input.set(&new_line);
            self.msg(&msg, Level::Info);
            self.completion_cycle = Some(state);
            return;
        }

        let line = self.input.value();
        let pos = self.input.cursor();
        let comps = complete_for_tui(line, pos);
        
        if comps.is_empty() {
            return;
        }
        
        if comps.len() == 1 {
            let c = &comps[0];
            let new = format!("{}{}{}", &line[..c.start], c.replacement, &line[pos..]);
            self.input.set(&new);
        } else {
            // Start a completion cycle:
            // - replace the span [start..pos] with each candidate's replacement
            // - Tab cycles through candidates
            let start = comps[0].start;
            let prefix = line[..start].to_string();
            let suffix = line[pos..].to_string();
            let first_display = comps[0].display.clone();
            let first_replacement = comps[0].replacement.clone();

            let new_line = format!("{}{}{}", prefix, first_replacement, suffix);
            self.input.set(&new_line);
            self.msg(
                &format!("▶ {} (1/{})  Tab to cycle", first_display, comps.len()),
                Level::Info,
            );

            self.completion_cycle = Some(CompletionCycle {
                prefix,
                suffix,
                candidates: comps,
                idx: 0,
            });
        }
    }

    fn search_samples(&mut self, query: &str) {
        use crate::repl::completer::complete_for_tui;
        let fake = format!("x ~ {}", query);
        let comps = complete_for_tui(&fake, fake.len());
        
        if comps.is_empty() {
            self.msg(&format!("No samples match '{}'", query), Level::Info);
        } else {
            let list: Vec<&str> = comps.iter().take(6).map(|c| c.display.as_str()).collect();
            self.msg(&list.join("  "), Level::Info);
        }
    }

    fn msg(&mut self, text: &str, level: Level) {
        self.messages.push(Message { text: text.to_string(), level });
        if self.messages.len() > 50 {
            self.messages.remove(0);
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        
        // Clear entire frame first
        frame.render_widget(ratatui::widgets::Clear, area);

        let snap = audio::snapshot_live_state();
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Header
                Constraint::Min(6),     // Tracker
                Constraint::Length(7),  // Messages
                Constraint::Length(3),  // Input
            ])
            .split(area);

        self.render_header(frame, chunks[0], snap.as_ref());
        self.render_tracker(frame, chunks[1], snap);
        self.render_messages(frame, chunks[2]);
        self.render_input(frame, chunks[3]);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect, snap: Option<&audio::LiveSnapshot>) {
        let playing = audio::is_playing();
        let step = snap
            .map(|s| ((s.global_step % self.song.steps as usize) + 1).to_string())
            .unwrap_or_else(|| "—".to_string());
        
        let transport = if playing {
            Span::styled(" ▶ ", Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(" ■ ", Style::default().fg(Color::DarkGray).bg(Color::Rgb(40, 40, 40)))
        };

        let line = Line::from(vec![
            Span::styled(" GROOVE ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            transport,
            Span::raw("  "),
            Span::styled(format!("{} BPM", self.song.bpm), Style::default().fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled(format!("{} tracks", self.song.tracks.len()), Style::default().fg(Color::DarkGray)),
            Span::raw("  "),
            Span::styled(format!("swing {}", self.song.swing), Style::default().fg(Color::DarkGray)),
            Span::raw("  "),
            Span::styled(format!("step {}/{}", step, self.song.steps), Style::default().fg(Color::DarkGray)),
            Span::styled("  Enter: run  Tab: complete  Ctrl-C: quit", Style::default().fg(Color::DarkGray)),
        ]);

        frame.render_widget(Paragraph::new(line), area);
    }

    fn render_tracker(&self, frame: &mut Frame, area: Rect, snap: Option<audio::LiveSnapshot>) {
        let block = Block::default()
            .title(" Tracks ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(60, 60, 60)));
        frame.render_widget(TrackerGrid::new(&self.song).block(block).snapshot(snap), area);
    }

    fn render_messages(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Output ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(60, 60, 60)));
        let inner = block.inner(area);
        let max_lines = inner.height as usize;

        let mut lines: Vec<Line> = Vec::new();
        for msg in self.messages.iter().rev().take(max_lines) {
            let style = match msg.level {
                Level::Error => Style::default().fg(Color::Red),
                Level::Warn => Style::default().fg(Color::Yellow),
                Level::Info => Style::default().fg(Color::DarkGray),
            };
            lines.push(Line::styled(&msg.text, style));
        }
        lines.reverse();

        frame.render_widget(
            Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
            area,
        );
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Command ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(60, 60, 60)));
        let inner = block.inner(area);
        let prompt = "› ";
        let prompt_width = 2usize; // terminal cells (not bytes)
        let prompt_style = Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD);

        // Horizontal scroll to keep cursor visible.
        let buf = self.input.value();
        let cursor = self.input.cursor();
        let max_text_width = (inner.width as usize).saturating_sub(prompt_width);
        let start = if max_text_width == 0 {
            0
        } else if cursor > max_text_width {
            cursor - max_text_width
        } else {
            0
        };
        let end = (start + max_text_width).min(buf.len());
        let visible = buf.get(start..end).unwrap_or("");

        let line = Line::from(vec![
            Span::styled(prompt, prompt_style),
            Span::raw(visible),
        ]);

        frame.render_widget(
            Paragraph::new(line).block(block),
            area,
        );

        if inner.width > 0 && inner.height > 0 {
            let cursor_rel = cursor.saturating_sub(start) as u16;
            let cursor_x = inner.x + (prompt_width as u16) + cursor_rel;
            frame.set_cursor_position((cursor_x.min(inner.x + inner.width - 1), inner.y));
        }
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

pub fn run(song: Song) -> Result<()> {
    TuiApp::new(song).run()
}
