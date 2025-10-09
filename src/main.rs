mod model;
mod repl;
mod storage;
mod audio;

use anyhow::Result;
use clap::{Arg, ArgAction, Command};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::env;
use std::fs;
use std::time::{Duration, SystemTime};

fn cli() -> Command {
    Command::new("groove-cli")
        .about("CLI groovebox REPL")
        .arg(
            Arg::new("open")
                .short('o')
                .long("open")
                .value_name("FILE")
                .help("Open a YAML song on start"),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .action(ArgAction::SetTrue)
                .help("Reduce startup banner output"),
        )
}

fn main() -> Result<()> {
    let matches = cli().get_matches();

    let mut song = if let Some(path) = matches.get_one::<String>("open") {
        match storage::song::open(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to open {}: {}\nStarting new song.", path, e);
                model::song::Song::default()
            }
        }
    } else {
        model::song::Song::default()
    };

    if !matches.get_flag("quiet") {
        println!(
            "CLI GROOVEBOX REPL — bpm: {} steps: {} swing: {}% repeat:{} (type :help)",
            song.bpm,
            song.steps,
            song.swing,
            if song.repeat_on() { "on" } else { "off" }
        );
    }

    // If a song file exists in CWD (song.yaml preferred) or was opened, watch it for changes
    let watch_path: Option<PathBuf> = if let Some(path) = matches.get_one::<String>("open") {
        Some(PathBuf::from(path))
    } else {
        let yaml = PathBuf::from("song.yaml");
        let yml = PathBuf::from("song.yml");
        if yaml.exists() { Some(yaml) }
        else if yml.exists() { Some(yml) }
        else { None }
    };
    if let Some(song_path) = watch_path {
        println!("watching: {}", song_path.display());
        start_watcher(song_path.clone());
        // Polling fallback for editors that use atomic rename or missed events
        start_polling(song_path);
    }

    repl::run_repl(&mut song)?;

    Ok(())
}

fn start_watcher(path: PathBuf) {
    // Resolve a reliable directory to watch (handle bare filenames and missing parents)
    let parent = match path.parent().filter(|p| !p.as_os_str().is_empty()) {
        Some(p) => p.to_path_buf(),
        None => env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };
    let file_name = path.file_name().map(|s| s.to_os_string());
    thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        let mut watcher: RecommendedWatcher = match Watcher::new(tx, notify::Config::default()) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("file watch disabled (create watcher failed): {}", e);
                return;
            }
        };
        if let Err(e) = watcher.watch(parent.as_path(), RecursiveMode::NonRecursive) {
            eprintln!(
                "file watch disabled (cannot watch parent dir '{}'): {}",
                parent.display(),
                e
            );
            return;
        }
        loop {
            match rx.recv() {
                Ok(event) => {
                    if let Ok(event) = event {
                        // Filter for our file
                        let relevant = event.paths.iter().any(|p| {
                            if let Some(fname) = &file_name { p.file_name() == Some(fname.as_ref()) } else { false }
                        });
                        if !relevant { continue; }
                        // On any relevant event, attempt reload (debounced)
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        if let Ok(new_song) = storage::song::open(&path) {
                            println!("reloaded: {} (event)", path.display());
                            crate::audio::reload_song(&new_song);
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });
}

fn start_polling(path: PathBuf) {
    thread::spawn(move || {
        let mut last_mod: Option<SystemTime> = None;
        let mut last_len: Option<u64> = None;
        loop {
            match fs::metadata(&path) {
                Ok(meta) => {
                    let mtime = meta.modified().ok();
                    let len = Some(meta.len());
                    let changed = match (last_mod, last_len, mtime, len) {
                        (Some(lm), Some(ll), Some(m), Some(l)) => m > lm || l != ll,
                        (None, _, Some(_), Some(_)) => true,
                        _ => false,
                    };
                    if changed {
                        // Debounce
                        thread::sleep(Duration::from_millis(50));
                        if let Ok(new_song) = storage::song::open(&path) {
                            println!("reloaded: {} (poll)", path.display());
                            crate::audio::reload_song(&new_song);
                            last_mod = mtime;
                            last_len = len;
                        }
                    }
                }
                Err(_) => {
                    // File missing; reset and keep watching
                    last_mod = None;
                    last_len = None;
                }
            }
            thread::sleep(Duration::from_millis(200));
        }
    });
}
