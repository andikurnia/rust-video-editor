use serde::{Deserialize, Serialize};

pub type Time = f64;
pub type ClipId = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrackKind {
    Video,
    Audio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    pub id: ClipId,
    pub source_path: String,
    pub name: String,
    pub start: Time,
    pub duration: Time,
    pub source_in: Time,
    pub track_index: usize,
}

impl Clip {
    pub fn end(&self) -> Time {
        self.start + self.duration
    }

    pub fn source_end(&self) -> Time {
        self.source_in + self.duration
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub kind: TrackKind,
    pub name: String,
    pub clips: Vec<Clip>,
}

impl Track {
    pub fn new(kind: TrackKind, name: String) -> Self {
        Track {
            kind,
            name,
            clips: Vec::new(),
        }
    }

    pub fn duration(&self) -> Time {
        self.clips.iter().map(|c| c.end()).fold(0.0, f64::max)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    pub video_tracks: Vec<Track>,
    pub audio_tracks: Vec<Track>,
    pub duration: Time,
}

impl Timeline {
    pub fn new() -> Self {
        Timeline {
            video_tracks: vec![Track::new(TrackKind::Video, "V1".into())],
            audio_tracks: vec![Track::new(TrackKind::Audio, "A1".into())],
            duration: 0.0,
        }
    }

    pub fn all_tracks(&self) -> impl Iterator<Item = &Track> {
        self.video_tracks.iter().chain(self.audio_tracks.iter())
    }

    pub fn add_video_track(&mut self) -> usize {
        let idx = self.video_tracks.len() + 1;
        self.video_tracks
            .push(Track::new(TrackKind::Video, format!("V{idx}")));
        self.video_tracks.len() - 1
    }

    pub fn add_audio_track(&mut self) -> usize {
        let idx = self.audio_tracks.len() + 1;
        self.audio_tracks
            .push(Track::new(TrackKind::Audio, format!("A{idx}")));
        self.audio_tracks.len() - 1
    }

    pub fn recalculate_duration(&mut self) {
        self.duration = self
            .all_tracks()
            .map(|t| t.duration())
            .fold(0.0, f64::max);
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_clip(id: u64, start: Time, duration: Time, track: usize) -> Clip {
        Clip {
            id,
            source_path: "/test.mp4".into(),
            name: format!("clip_{id}"),
            start,
            duration,
            source_in: 0.0,
            track_index: track,
        }
    }

    #[test]
    fn test_clip_end() {
        let c = make_clip(1, 10.0, 5.0, 0);
        assert!((c.end() - 15.0).abs() < 1e-9);
    }

    #[test]
    fn test_clip_source_end() {
        let c = Clip {
            source_in: 2.0,
            duration: 5.0,
            ..make_clip(1, 10.0, 5.0, 0)
        };
        assert!((c.source_end() - 7.0).abs() < 1e-9);
    }

    #[test]
    fn test_track_new() {
        let t = Track::new(TrackKind::Video, "V1".into());
        assert!(matches!(t.kind, TrackKind::Video));
        assert_eq!(t.name, "V1");
        assert!(t.clips.is_empty());
    }

    #[test]
    fn test_track_duration_empty() {
        let t = Track::new(TrackKind::Video, "V1".into());
        assert!((t.duration() - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_track_duration_with_clips() {
        let mut t = Track::new(TrackKind::Video, "V1".into());
        t.clips.push(make_clip(1, 0.0, 10.0, 0));
        t.clips.push(make_clip(2, 15.0, 5.0, 0));
        assert!((t.duration() - 20.0).abs() < 1e-9);
    }

    #[test]
    fn test_timeline_new() {
        let tl = Timeline::new();
        assert_eq!(tl.video_tracks.len(), 1);
        assert_eq!(tl.audio_tracks.len(), 1);
        assert!((tl.duration - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_timeline_default() {
        let tl = Timeline::default();
        assert_eq!(tl.video_tracks.len(), 1);
        assert_eq!(tl.audio_tracks.len(), 1);
    }

    #[test]
    fn test_all_tracks_yields_all() {
        let tl = Timeline::new();
        let count = tl.all_tracks().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_add_video_track() {
        let mut tl = Timeline::new();
        let idx = tl.add_video_track();
        assert_eq!(idx, 1);
        assert_eq!(tl.video_tracks.len(), 2);
        assert_eq!(tl.video_tracks[1].name, "V2");
    }

    #[test]
    fn test_add_audio_track() {
        let mut tl = Timeline::new();
        let idx = tl.add_audio_track();
        assert_eq!(idx, 1);
        assert_eq!(tl.audio_tracks.len(), 2);
        assert_eq!(tl.audio_tracks[1].name, "A2");
    }

    #[test]
    fn test_recalculate_duration_no_clips() {
        let mut tl = Timeline::new();
        tl.recalculate_duration();
        assert!((tl.duration - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_recalculate_duration_single_track() {
        let mut tl = Timeline::new();
        tl.video_tracks[0].clips.push(make_clip(1, 0.0, 10.0, 0));
        tl.recalculate_duration();
        assert!((tl.duration - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_recalculate_duration_multi_track() {
        let mut tl = Timeline::new();
        tl.video_tracks[0].clips.push(make_clip(1, 0.0, 10.0, 0));
        tl.audio_tracks[0].clips.push(make_clip(2, 20.0, 15.0, 1));
        tl.recalculate_duration();
        assert!((tl.duration - 35.0).abs() < 1e-9);
    }

    #[test]
    fn test_clip_serde_roundtrip() {
        let c = make_clip(42, 1.5, 3.0, 0);
        let json = serde_json::to_string(&c).unwrap();
        let deserialized: Clip = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, 42);
        assert!((deserialized.start - 1.5).abs() < 1e-9);
    }

    #[test]
    fn test_timeline_serde_roundtrip() {
        let mut tl = Timeline::new();
        tl.video_tracks[0].clips.push(make_clip(1, 0.0, 10.0, 0));
        tl.recalculate_duration();
        let json = serde_json::to_string(&tl).unwrap();
        let deserialized: Timeline = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.video_tracks[0].clips.len(), 1);
        assert!((deserialized.duration - 10.0).abs() < 1e-9);
    }
}
