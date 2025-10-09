use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::model::song::Song;

pub fn save(song: &Song, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    let data = match ext.as_str() {
        "yml" | "yaml" => serde_yaml::to_string(song)?,
        "toml" => toml::to_string_pretty(song)?,
        _ => serde_yaml::to_string(song)?,
    };
    fs::write(path, data).with_context(|| format!("writing {}", path.display()))
}

pub fn open(path: impl AsRef<Path>) -> Result<Song> {
    let path = path.as_ref();
    let data = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    let song: Song = match ext.as_str() {
        "yml" | "yaml" => serde_yaml::from_str(&data)?,
        "toml" => toml::from_str(&data)?,
        _ => {
            // Try YAML then TOML for unknown extensions
            match serde_yaml::from_str(&data) {
                Ok(s) => s,
                Err(_) => toml::from_str(&data)?,
            }
        }
    };
    Ok(song)
}
