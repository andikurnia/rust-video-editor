extern crate ffmpeg_next as ffmpeg;

pub mod probe;
pub mod decode;
pub mod encode;
pub mod compositor;
pub mod export;

pub use probe::MediaProbe;
pub use decode::{DecodedFrame, VideoDecoder};
pub use encode::{ExportConfig, VideoEncoder};
pub use compositor::FrameCompositor;
pub use export::{export_timeline, ExportError};

static FFMPEG_INIT: std::sync::Once = std::sync::Once::new();

pub(crate) fn ffmpeg_init() {
    FFMPEG_INIT.call_once(|| {
        ffmpeg::init().expect("Failed to initialize FFmpeg");
    });
}
