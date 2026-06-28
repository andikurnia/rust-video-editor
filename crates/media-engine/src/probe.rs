use serde::{Deserialize, Serialize};

use crate::ffmpeg_init;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    pub path: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
    pub video_codec: String,
    pub audio_codec: Option<String>,
    pub has_audio: bool,
    pub has_video: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("Probe failed: {0}")]
    Failed(String),
}

pub struct MediaProbe;

impl MediaProbe {
    pub fn new() -> Self {
        MediaProbe
    }

    pub fn probe(&self, path: &str) -> Result<MediaInfo, ProbeError> {
        if !std::path::Path::new(path).exists() {
            return Err(ProbeError::FileNotFound(path.into()));
        }
        ffmpeg_init();
        let ictx = ffmpeg::format::input(&path)
            .map_err(|e| ProbeError::Failed(format!("Cannot open file: {e}")))?;

        let duration = ictx.duration() as f64 / ffmpeg::ffi::AV_TIME_BASE as f64;

        let mut info = MediaInfo {
            path: path.to_string(),
            duration,
            width: 0,
            height: 0,
            frame_rate: 0.0,
            video_codec: String::new(),
            audio_codec: None,
            has_audio: false,
            has_video: false,
        };

        for stream in ictx.streams() {
            let params = stream.parameters();
            match params.medium() {
                ffmpeg::media::Type::Video => {
                    info.has_video = true;
                    if let Ok(codec) = ffmpeg::codec::context::Context::from_parameters(params) {
                        if let Ok(decoder) = codec.decoder().video() {
                            info.width = decoder.width();
                            info.height = decoder.height();
                            info.video_codec = decoder
                                .codec()
                                .map(|c| c.name().to_string())
                                .unwrap_or_default();
                        }
                    }
                    let num = stream.rate().0;
                    let den = stream.rate().1;
                    if den > 0 {
                        info.frame_rate = num as f64 / den as f64;
                    }
                }
                ffmpeg::media::Type::Audio => {
                    info.has_audio = true;
                    if let Ok(codec) = ffmpeg::codec::context::Context::from_parameters(params) {
                        if let Ok(decoder) = codec.decoder().audio() {
                            info.audio_codec = Some(
                                decoder
                                    .codec()
                                    .map(|c| c.name().to_string())
                                    .unwrap_or_default(),
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        if !info.has_video && !info.has_audio {
            return Err(ProbeError::UnsupportedFormat(
                "No video or audio streams found".into(),
            ));
        }

        Ok(info)
    }
}

impl Default for MediaProbe {
    fn default() -> Self {
        Self::new()
    }
}
