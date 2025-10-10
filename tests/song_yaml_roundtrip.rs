use groove_cli::model::pattern::Pattern;
use groove_cli::model::song::Song;
use groove_cli::model::track::{Track, TrackPlayback};

#[test]
fn roundtrip_song_yaml() {
    let mut s = Song::default();
    let mut t = Track::new("Kick");
    t.pattern = Some(Pattern::visual("x... x... x... x..."));
    t.sample = Some("samples/909/kick.wav".into());
    s.tracks.push(t);

    let yaml = serde_yaml::to_string(&s).expect("serialize");
    let out: Song = serde_yaml::from_str(&yaml).expect("deserialize");

    assert_eq!(out.tracks.len(), 1);
    assert_eq!(out.tracks[0].name, "Kick");
}

#[test]
fn track_playback_defaults_to_gate() {
    let t = Track::new("Snare");
    assert_eq!(t.playback, TrackPlayback::Gate);
}

#[test]
fn track_playback_roundtrips_through_yaml() {
    let mut t = Track::new("Hat");
    t.playback = TrackPlayback::Mono;

    let yaml = serde_yaml::to_string(&t).expect("serialize track");
    let out: Track = serde_yaml::from_str(&yaml).expect("deserialize track");

    assert_eq!(out.playback, TrackPlayback::Mono);
}
