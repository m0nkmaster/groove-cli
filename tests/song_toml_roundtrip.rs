use groove_cli::model::song::Song;
use groove_cli::model::track::Track;
use groove_cli::model::pattern::Pattern;

#[test]
fn roundtrip_song_toml() {
    let mut s = Song::default();
    let mut t = Track::new("Kick");
    t.pattern = Some(Pattern::visual("x... x... x... x..."));
    t.sample = Some("samples/909/kick.wav".into());
    s.tracks.push(t);

    let toml = toml::to_string_pretty(&s).expect("serialize");
    let out: Song = toml::from_str(&toml).expect("deserialize");

    assert_eq!(out.tracks.len(), 1);
    assert_eq!(out.tracks[0].name, "Kick");
}

