use editor_core::Timeline;

use crate::compositor::FrameCompositor;
use crate::encode::{EncodeError, ExportConfig, VideoEncoder};

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("Encode: {0}")]
    Encode(#[from] EncodeError),
    #[error("Composite: {0}")]
    Composite(String),
    #[error("No frames in timeline")]
    NoFrames,
}

pub fn export_timeline(
    timeline: &Timeline,
    config: &ExportConfig,
    progress: impl Fn(u64, u64),
) -> Result<(), ExportError> {
    if timeline.duration <= 0.0 {
        return Err(ExportError::NoFrames);
    }

    let mut encoder = VideoEncoder::new(config)?;
    let mut compositor = FrameCompositor::new();

    let fps = config.frame_rate;
    let total_frames = (timeline.duration * fps).ceil() as u64;

    for frame_idx in 0..total_frames {
        let time = frame_idx as f64 / fps;
        let frame = compositor
            .composite_frame(timeline, time, config.width, config.height)
            .map_err(ExportError::Composite)?;
        encoder.write_frame(&frame)?;

        if frame_idx % 10 == 0 || frame_idx == total_frames - 1 {
            progress(frame_idx + 1, total_frames);
        }
    }

    encoder.finalize()?;
    Ok(())
}
