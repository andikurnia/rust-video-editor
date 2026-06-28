use crate::ffmpeg_init;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
    pub video_bitrate: String,
    pub audio_bitrate: String,
    pub video_codec: String,
    pub audio_codec: String,
    pub container: String,
    pub output_path: String,
}

impl ExportConfig {
    pub fn youtube_1080p(output_path: &str) -> Self {
        ExportConfig {
            width: 1920,
            height: 1080,
            frame_rate: 30.0,
            video_bitrate: "16M".into(),
            audio_bitrate: "192k".into(),
            video_codec: "libx264".into(),
            audio_codec: "aac".into(),
            container: "mp4".into(),
            output_path: output_path.into(),
        }
    }

    pub fn tiktok(output_path: &str) -> Self {
        ExportConfig {
            width: 1080,
            height: 1920,
            frame_rate: 30.0,
            video_bitrate: "8M".into(),
            audio_bitrate: "128k".into(),
            video_codec: "libx264".into(),
            audio_codec: "aac".into(),
            container: "mp4".into(),
            output_path: output_path.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    #[error("Open output: {0}")]
    OpenFailed(String),
    #[error("Write frame: {0}")]
    WriteFailed(String),
    #[error("Finalize: {0}")]
    FinalizeFailed(String),
}

pub struct VideoEncoder {
    output: ffmpeg::format::context::Output,
    video_stream_index: usize,
    encoder: ffmpeg::codec::encoder::Video,
}

impl VideoEncoder {
    pub fn new(config: &ExportConfig) -> Result<Self, EncodeError> {
        ffmpeg_init();

        let mut output = ffmpeg::format::output(&config.output_path)
            .map_err(|e| EncodeError::OpenFailed(format!("Create output: {e}")))?;

        let mut video_enc = ffmpeg::codec::context::Context::new()
            .encoder()
            .video()
            .map_err(|e| EncodeError::OpenFailed(format!("Video encoder: {e}")))?;

        video_enc.set_width(config.width);
        video_enc.set_height(config.height);
        video_enc.set_format(ffmpeg::format::Pixel::YUV420P);
        video_enc.set_frame_rate(Some(ffmpeg::Rational(
            (config.frame_rate * 1000.0).round() as i32,
            1000,
        )));
        video_enc.set_bit_rate(parse_bitrate(&config.video_bitrate));

        let opened = video_enc
            .open_as(ffmpeg::codec::Id::H264)
            .map_err(|e| EncodeError::OpenFailed(format!("Open encoder: {e}")))?;

        let stream = output
            .add_stream_with(opened.as_ref())
            .map_err(|e| EncodeError::OpenFailed(format!("Add stream: {e}")))?;

        let video_stream_index = stream.index();

        output
            .write_header()
            .map_err(|e| EncodeError::OpenFailed(format!("Write header: {e}")))?;

        Ok(VideoEncoder {
            output,
            video_stream_index,
            encoder: opened,
        })
    }

    pub fn write_frame(&mut self, frame: &crate::DecodedFrame) -> Result<(), EncodeError> {
        let mut rgb = ffmpeg::frame::Video::new(ffmpeg::format::Pixel::RGBA, frame.width, frame.height);
        rgb.data_mut(0).copy_from_slice(&frame.data);
        rgb.set_pts(Some((frame.pts * 1000.0) as i64));

        let mut yuv = ffmpeg::frame::Video::new(
            ffmpeg::format::Pixel::YUV420P,
            frame.width,
            frame.height,
        );
        let mut scaler = ffmpeg::software::scaling::Context::get(
            ffmpeg::format::Pixel::RGBA,
            frame.width,
            frame.height,
            ffmpeg::format::Pixel::YUV420P,
            frame.width,
            frame.height,
            ffmpeg::software::scaling::Flags::BILINEAR,
        )
        .map_err(|e| EncodeError::WriteFailed(format!("Scaler: {e}")))?;

        scaler
            .run(&rgb, &mut yuv)
            .map_err(|e| EncodeError::WriteFailed(format!("Scale: {e}")))?;
        yuv.set_pts(Some((frame.pts * 1000.0) as i64));

        self.encoder
            .send_frame(&yuv)
            .map_err(|e| EncodeError::WriteFailed(format!("Send frame: {e}")))?;

        let mut packet = ffmpeg::Packet::empty();
        loop {
            match self.encoder.receive_packet(&mut packet) {
                Ok(()) => {
                    packet.set_stream(self.video_stream_index);
                    packet
                        .write_interleaved(&mut self.output)
                        .map_err(|e| EncodeError::WriteFailed(format!("Write packet: {e}")))?;
                }
                Err(ffmpeg::Error::Other { errno }) if errno == ffmpeg::error::EAGAIN => break,
                Err(_) => break,
            }
        }
        Ok(())
    }

    pub fn finalize(mut self) -> Result<(), EncodeError> {
        self.encoder
            .send_eof()
            .map_err(|e| EncodeError::FinalizeFailed(format!("Send eof: {e}")))?;
        let mut packet = ffmpeg::Packet::empty();
        loop {
            match self.encoder.receive_packet(&mut packet) {
                Ok(()) => {
                    packet.set_stream(self.video_stream_index);
                    packet
                        .write_interleaved(&mut self.output)
                        .map_err(|e| EncodeError::FinalizeFailed(format!("Write: {e}")))?;
                }
                Err(ffmpeg::Error::Other { errno }) if errno == ffmpeg::error::EAGAIN => break,
                Err(_) => break,
            }
        }
        self.output
            .write_trailer()
            .map_err(|e| EncodeError::FinalizeFailed(format!("Trailer: {e}")))?;
        Ok(())
    }
}

fn parse_bitrate(s: &str) -> usize {
    if let Some(stripped) = s.strip_suffix("M") {
        let num: f64 = stripped.parse().unwrap_or(1.0);
        (num * 1_000_000.0) as usize
    } else if let Some(stripped) = s.strip_suffix("k") {
        let num: f64 = stripped.parse().unwrap_or(1.0);
        (num * 1_000.0) as usize
    } else {
        s.parse().unwrap_or(1_000_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bitrate_numeric() {
        assert_eq!(parse_bitrate("5000000"), 5_000_000);
    }

    #[test]
    fn test_parse_bitrate_m_suffix() {
        assert_eq!(parse_bitrate("16M"), 16_000_000);
    }

    #[test]
    fn test_parse_bitrate_k_suffix() {
        assert_eq!(parse_bitrate("192k"), 192_000);
    }

    #[test]
    fn test_parse_bitrate_float_m() {
        assert_eq!(parse_bitrate("2.5M"), 2_500_000);
    }

    #[test]
    fn test_parse_bitrate_float_k() {
        assert_eq!(parse_bitrate("128k"), 128_000);
    }

    #[test]
    fn test_parse_bitrate_invalid_fallback() {
        assert_eq!(parse_bitrate("not_a_number"), 1_000_000);
    }

    #[test]
    fn test_parse_bitrate_empty_fallback() {
        assert_eq!(parse_bitrate(""), 1_000_000);
    }

    #[test]
    fn test_export_config_youtube_1080p() {
        let cfg = ExportConfig::youtube_1080p("/out.mp4");
        assert_eq!(cfg.width, 1920);
        assert_eq!(cfg.height, 1080);
        assert_eq!(cfg.video_bitrate, "16M");
        assert_eq!(cfg.audio_bitrate, "192k");
    }

    #[test]
    fn test_export_config_tiktok() {
        let cfg = ExportConfig::tiktok("/out.mp4");
        assert_eq!(cfg.width, 1080);
        assert_eq!(cfg.height, 1920);
        assert_eq!(cfg.video_bitrate, "8M");
    }
}
