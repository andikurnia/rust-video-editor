use std::cell::RefCell;

use crate::timeline::{Clip, ClipId, Timeline};

pub type CommandResult = Result<(), String>;

pub trait TimelineCommand {
    fn name(&self) -> &str;
    fn execute(&self, timeline: &mut Timeline) -> CommandResult;
    fn undo(&self, timeline: &mut Timeline) -> CommandResult;
}

fn track_mut(timeline: &mut Timeline, track_index: usize) -> Option<&mut crate::timeline::Track> {
    if track_index < timeline.video_tracks.len() {
        Some(&mut timeline.video_tracks[track_index])
    } else if track_index - timeline.video_tracks.len() < timeline.audio_tracks.len() {
        Some(&mut timeline.audio_tracks[track_index - timeline.video_tracks.len()])
    } else {
        None
    }
}

pub struct AddClip {
    pub clip: Clip,
}

impl TimelineCommand for AddClip {
    fn name(&self) -> &str {
        "Add Clip"
    }

    fn execute(&self, timeline: &mut Timeline) -> CommandResult {
        let track = track_mut(timeline, self.clip.track_index)
            .ok_or_else(|| "Track not found".to_string())?;
        track.clips.push(self.clip.clone());
        timeline.recalculate_duration();
        Ok(())
    }

    fn undo(&self, timeline: &mut Timeline) -> CommandResult {
        let track = track_mut(timeline, self.clip.track_index)
            .ok_or_else(|| "Track not found".to_string())?;
        track.clips.retain(|c| c.id != self.clip.id);
        timeline.recalculate_duration();
        Ok(())
    }
}

pub struct RemoveClip {
    pub clip_id: ClipId,
    removed_clip: RefCell<Option<Clip>>,
}

impl RemoveClip {
    pub fn new(clip_id: ClipId) -> Self {
        RemoveClip {
            clip_id,
            removed_clip: RefCell::new(None),
        }
    }
}

impl TimelineCommand for RemoveClip {
    fn name(&self) -> &str {
        "Remove Clip"
    }

    fn execute(&self, timeline: &mut Timeline) -> CommandResult {
        for track in timeline
            .video_tracks
            .iter_mut()
            .chain(timeline.audio_tracks.iter_mut())
        {
            if let Some(pos) = track.clips.iter().position(|c| c.id == self.clip_id) {
                let clip = track.clips.remove(pos);
                timeline.recalculate_duration();
                *self.removed_clip.borrow_mut() = Some(clip);
                return Ok(());
            }
        }
        Err("Clip not found".into())
    }

    fn undo(&self, timeline: &mut Timeline) -> CommandResult {
        let clip = self.removed_clip.borrow().clone()
            .ok_or_else(|| "No clip to restore".to_string())?;
        let track = track_mut(timeline, clip.track_index)
            .ok_or_else(|| "Track not found".to_string())?;
        track.clips.push(clip);
        timeline.recalculate_duration();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::{Clip, Timeline, TrackKind};

    fn make_clip(id: u64, start: f64, duration: f64, track: usize) -> Clip {
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
    fn test_add_clip_execute() {
        let mut tl = Timeline::new();
        let clip = make_clip(1, 0.0, 5.0, 0);
        let cmd = AddClip { clip };
        assert!(cmd.execute(&mut tl).is_ok());
        assert_eq!(tl.video_tracks[0].clips.len(), 1);
        assert!((tl.duration - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_add_clip_undo() {
        let mut tl = Timeline::new();
        let clip = make_clip(1, 0.0, 5.0, 0);
        let cmd = AddClip { clip };
        cmd.execute(&mut tl).unwrap();
        cmd.undo(&mut tl).unwrap();
        assert!(tl.video_tracks[0].clips.is_empty());
        assert!((tl.duration - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_add_clip_invalid_track() {
        let mut tl = Timeline::new();
        let clip = make_clip(1, 0.0, 5.0, 99); // invalid track index
        let cmd = AddClip { clip };
        assert!(cmd.execute(&mut tl).is_err());
    }

    #[test]
    fn test_add_clip_name() {
        let clip = make_clip(1, 0.0, 5.0, 0);
        let cmd = AddClip { clip };
        assert_eq!(cmd.name(), "Add Clip");
    }

    #[test]
    fn test_remove_clip_execute() {
        let mut tl = Timeline::new();
        let clip = make_clip(1, 0.0, 5.0, 0);
        tl.video_tracks[0].clips.push(clip);
        tl.recalculate_duration();
        let cmd = RemoveClip::new(1);
        assert!(cmd.execute(&mut tl).is_ok());
        assert!(tl.video_tracks[0].clips.is_empty());
        assert!((tl.duration - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_remove_clip_undo() {
        let mut tl = Timeline::new();
        let clip = make_clip(1, 0.0, 5.0, 0);
        tl.video_tracks[0].clips.push(clip);
        let cmd = RemoveClip::new(1);
        cmd.execute(&mut tl).unwrap();
        cmd.undo(&mut tl).unwrap();
        assert_eq!(tl.video_tracks[0].clips.len(), 1);
        assert!((tl.duration - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_remove_clip_not_found() {
        let mut tl = Timeline::new();
        let cmd = RemoveClip::new(999);
        assert!(cmd.execute(&mut tl).is_err());
    }

    #[test]
    fn test_remove_clip_undo_before_execute() {
        let mut tl = Timeline::new();
        let cmd = RemoveClip::new(1);
        assert!(cmd.undo(&mut tl).is_err());
    }

    #[test]
    fn test_remove_clip_name() {
        let cmd = RemoveClip::new(1);
        assert_eq!(cmd.name(), "Remove Clip");
    }

    #[test]
    fn test_track_mut_video() {
        let mut tl = Timeline::new();
        let track = track_mut(&mut tl, 0);
        assert!(track.is_some());
        assert!(matches!(track.unwrap().kind, TrackKind::Video));
    }

    #[test]
    fn test_track_mut_audio() {
        let mut tl = Timeline::new();
        let track = track_mut(&mut tl, 1);
        assert!(track.is_some());
        assert!(matches!(track.unwrap().kind, TrackKind::Audio));
    }

    #[test]
    fn test_track_mut_out_of_bounds() {
        let mut tl = Timeline::new();
        let track = track_mut(&mut tl, 99);
        assert!(track.is_none());
    }
}
