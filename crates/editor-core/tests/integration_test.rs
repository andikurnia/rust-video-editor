use editor_core::{Clip, Project, Timeline};

#[test]
fn test_project_serde_roundtrip() {
    let mut tl = Timeline::new();
    tl.video_tracks[0].clips.push(Clip {
        id: 1,
        source_path: "/data/video.mp4".into(),
        name: "Test Clip".into(),
        start: 0.0,
        duration: 10.0,
        source_in: 5.0,
        track_index: 0,
    });
    tl.recalculate_duration();

    let project = Project::new("Integration Test");
    let json = serde_json::to_string(&project).unwrap();
    let restored: Project = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.name, "Integration Test");
    assert_eq!(restored.timeline.video_tracks.len(), 1);
    assert_eq!(restored.timeline.audio_tracks.len(), 1);
}

#[test]
fn test_timeline_add_remove_clips_integration() {
    use editor_core::commands::{AddClip, RemoveClip, TimelineCommand};

    let mut tl = Timeline::new();
    let clip = Clip {
        id: 1,
        source_path: "/data/clip_a.mp4".into(),
        name: "Clip A".into(),
        start: 0.0,
        duration: 5.0,
        source_in: 0.0,
        track_index: 0,
    };

    let add = AddClip { clip };
    add.execute(&mut tl).unwrap();
    assert_eq!(tl.video_tracks[0].clips.len(), 1);
    assert!((tl.duration - 5.0).abs() < 1e-9);

    let remove = RemoveClip::new(1);
    remove.execute(&mut tl).unwrap();
    assert!(tl.video_tracks[0].clips.is_empty());

    remove.undo(&mut tl).unwrap();
    assert_eq!(tl.video_tracks[0].clips.len(), 1);
}

#[test]
fn test_track_duration_with_gap() {
    let mut tl = Timeline::new();
    tl.video_tracks[0].clips.push(Clip {
        id: 1,
        source_path: "/data/a.mp4".into(),
        name: "A".into(),
        start: 0.0,
        duration: 5.0,
        source_in: 0.0,
        track_index: 0,
    });
    tl.video_tracks[0].clips.push(Clip {
        id: 2,
        source_path: "/data/b.mp4".into(),
        name: "B".into(),
        start: 10.0,
        duration: 3.0,
        source_in: 0.0,
        track_index: 0,
    });
    tl.recalculate_duration();
    assert!((tl.duration - 13.0).abs() < 1e-9);
}

#[test]
fn test_multi_track_overlap() {
    let mut tl = Timeline::new();
    tl.add_video_track();

    tl.video_tracks[0].clips.push(Clip {
        id: 1,
        source_path: "/data/bg.mp4".into(),
        name: "BG".into(),
        start: 0.0,
        duration: 20.0,
        source_in: 0.0,
        track_index: 0,
    });
    tl.video_tracks[1].clips.push(Clip {
        id: 2,
        source_path: "/data/overlay.mp4".into(),
        name: "Overlay".into(),
        start: 5.0,
        duration: 10.0,
        source_in: 2.0,
        track_index: 1,
    });
    tl.recalculate_duration();
    assert!((tl.duration - 20.0).abs() < 1e-9);
}
