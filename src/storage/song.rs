use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::model::song::Song;

pub fn save(song: &Song, path: impl AsRef<Path>) -> Result<()> {
    let toml = toml::to_string_pretty(song)?;
    fs::write(path.as_ref(), toml).with_context(|| format!("writing {}", path.as_ref().display()))
}

pub fn open(path: impl AsRef<Path>) -> Result<Song> {
    let data = fs::read_to_string(path.as_ref()).with_context(|| format!("reading {}", path.as_ref().display()))?;
    let song: Song = toml::from_str(&data)?;
    Ok(song)
}

