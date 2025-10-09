use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::model::song::Song;

pub fn save(song: &Song, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    // Currently only YAML is supported; keep structure simple and explicit.
    let _ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    let data = serde_yaml::to_string(song)?;
    fs::write(path, data).with_context(|| format!("writing {}", path.display()))
}

pub fn open(path: impl AsRef<Path>) -> Result<Song> {
    let path = path.as_ref();
    let data = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    // Currently only YAML is supported; attempt YAML parse.
    let _ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    let song: Song = serde_yaml::from_str(&data)?;
    Ok(song)
}
