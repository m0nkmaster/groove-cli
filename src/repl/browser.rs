//! Interactive sample browser with folder navigation.

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, ClearType},
    style::Stylize,
};
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};

/// Entry in the browser (file or directory).
#[derive(Clone)]
struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

/// Result of the browser interaction.
pub enum BrowserResult {
    Selected(String),
    Cancelled,
}

/// Run an interactive sample browser starting from the given directory.
/// Returns the selected sample path or None if cancelled.
pub fn browse_samples(start_dir: &str) -> std::io::Result<BrowserResult> {
    let mut current_dir = PathBuf::from(start_dir);
    if !current_dir.exists() {
        current_dir = PathBuf::from("samples");
    }
    if !current_dir.exists() {
        current_dir = PathBuf::from(".");
    }
    
    let mut selected_idx: usize = 0;
    let mut entries = list_entries(&current_dir);
    
    // Enter raw mode for keyboard input
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    
    // Hide cursor
    execute!(stdout, cursor::Hide)?;
    
    let result = run_browser_loop(&mut stdout, &mut current_dir, &mut entries, &mut selected_idx);
    
    // Restore terminal
    execute!(stdout, cursor::Show)?;
    terminal::disable_raw_mode()?;
    
    // Clear the browser output
    execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;
    
    result
}

fn run_browser_loop(
    stdout: &mut std::io::Stdout,
    current_dir: &mut PathBuf,
    entries: &mut Vec<Entry>,
    selected_idx: &mut usize,
) -> std::io::Result<BrowserResult> {
    loop {
        render_browser(stdout, current_dir, entries, *selected_idx)?;
        
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if *selected_idx > 0 {
                        *selected_idx -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if *selected_idx + 1 < entries.len() {
                        *selected_idx += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                    if let Some(entry) = entries.get(*selected_idx) {
                        if entry.is_dir {
                            // Enter directory
                            *current_dir = entry.path.clone();
                            *entries = list_entries(current_dir);
                            *selected_idx = 0;
                        } else {
                            // Select file
                            let path = entry.path.to_string_lossy().to_string();
                            return Ok(BrowserResult::Selected(path));
                        }
                    }
                }
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
                    // Go up one directory
                    if let Some(parent) = current_dir.parent() {
                        *current_dir = parent.to_path_buf();
                        *entries = list_entries(current_dir);
                        *selected_idx = 0;
                    }
                }
                KeyCode::Char(' ') => {
                    // Preview sample
                    if let Some(entry) = entries.get(*selected_idx) {
                        if !entry.is_dir {
                            let path = entry.path.to_string_lossy().to_string();
                            let _ = crate::audio::preview_sample(&path);
                        }
                    }
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    return Ok(BrowserResult::Cancelled);
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(BrowserResult::Cancelled);
                }
                _ => {}
            }
        }
    }
}

fn render_browser(
    stdout: &mut std::io::Stdout,
    current_dir: &PathBuf,
    entries: &[Entry],
    selected_idx: usize,
) -> std::io::Result<()> {
    // Move to start and clear
    execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(ClearType::FromCursorDown))?;
    
    // Header
    writeln!(stdout, "{}", "── Sample Browser ──".bold())?;
    writeln!(stdout, "{}", format!("📁 {}", current_dir.display()).dark_grey())?;
    writeln!(stdout)?;
    
    // Get terminal height for scrolling
    let (_, term_height) = terminal::size().unwrap_or((80, 24));
    let max_visible = (term_height as usize).saturating_sub(8);
    
    // Calculate scroll window
    let scroll_start = if selected_idx >= max_visible {
        selected_idx - max_visible + 1
    } else {
        0
    };
    
    // Entries
    for (i, entry) in entries.iter().enumerate().skip(scroll_start).take(max_visible) {
        let prefix = if i == selected_idx { "▶ " } else { "  " };
        let icon = if entry.is_dir { "📁" } else { "🎵" };
        
        let line = format!("{}{} {}", prefix, icon, entry.name);
        
        if i == selected_idx {
            writeln!(stdout, "{}", line.reverse())?;
        } else if entry.is_dir {
            writeln!(stdout, "{}", line.blue())?;
        } else {
            writeln!(stdout, "{}", line)?;
        }
    }
    
    if entries.is_empty() {
        writeln!(stdout, "  {}", "(empty)".dark_grey())?;
    }
    
    // Footer
    writeln!(stdout)?;
    writeln!(stdout, "{}", "─".repeat(40).dark_grey())?;
    writeln!(stdout, "{}", "↑↓ navigate  Enter select  Space preview  ← back  Esc cancel".dark_grey())?;
    
    stdout.flush()?;
    Ok(())
}

fn list_entries(dir: &Path) -> Vec<Entry> {
    let mut entries = Vec::new();
    
    // Add parent directory entry if not at root
    if dir.parent().is_some() && dir != Path::new(".") {
        entries.push(Entry {
            name: "..".to_string(),
            path: dir.parent().unwrap().to_path_buf(),
            is_dir: true,
        });
    }
    
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return entries;
    };
    
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    
    for entry in read_dir.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        
        // Skip hidden files
        if name.starts_with('.') {
            continue;
        }
        
        if path.is_dir() {
            dirs.push(Entry { name, path, is_dir: true });
        } else if is_audio_file(&path) {
            files.push(Entry { name, path, is_dir: false });
        }
    }
    
    // Sort alphabetically
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    
    entries.extend(dirs);
    entries.extend(files);
    entries
}

fn is_audio_file(path: &Path) -> bool {
    let Some(ext) = path.extension() else { return false };
    let ext = ext.to_string_lossy().to_lowercase();
    matches!(ext.as_str(), "wav" | "mp3" | "ogg" | "flac" | "aiff" | "aif")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn list_entries_includes_audio_files() {
        // Just test that it doesn't panic on various inputs
        let _ = list_entries(Path::new("samples"));
        let _ = list_entries(Path::new("."));
        let _ = list_entries(Path::new("/nonexistent"));
    }
}

