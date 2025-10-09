mod model;
mod repl;
mod storage;

use anyhow::Result;
use clap::{Arg, ArgAction, Command};

fn cli() -> Command {
    Command::new("groove-cli")
        .about("CLI groovebox REPL")
        .arg(
            Arg::new("open")
                .short('o')
                .long("open")
                .value_name("FILE")
                .help("Open a TOML song on start"),
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
            "CLI GROOVEBOX REPL — bpm: {} steps: {} swing: {}% (type :help)",
            song.bpm, song.steps, song.swing
        );
    }

    repl::run_repl(&mut song)?;

    Ok(())
}

