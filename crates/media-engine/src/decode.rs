use crate::ffmpeg_init;

#[derive(Debug, Clone)]
pub struct DecodedFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub pts: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("No video stream found")]
    NoVideoStream,
    #[error("Decode failed: {0}")]
    Failed(String),
}

pub struct VideoDecoder {
    ictx: ffmpeg::format::context::Input,
    video_stream_index: usize,
    decoder: ffmpeg::codec::decoder::Video,
    scaler: ffmpeg::software::scaling::Context,
    time_base: f64,
}

impl VideoDecoder {
    pub fn new(path: &str) -> Result<Self, DecodeError> {
        if !std::path::Path::new(path).exists() {
            return Err(DecodeError::FileNotFound(path.into()));
        }
        ffmpeg_init();
        let ictx =
            ffmpeg::format::input(&path).map_err(|e| DecodeError::Failed(format!("Open: {e}")))?;

        let video_stream = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or(DecodeError::NoVideoStream)?;
        let video_stream_index = video_stream.index();
        let time_base = video_stream.time_base();

        let codec = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
            .map_err(|e| DecodeError::Failed(format!("Codec context: {e}")))?;
        let decoder = codec
            .decoder()
            .video()
            .map_err(|e| DecodeError::Failed(format!("Open decoder: {e}")))?;

        let scaler = ffmpeg::software::scaling::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg::format::Pixel::RGBA,
            decoder.width(),
            decoder.height(),
            ffmpeg::software::scaling::Flags::BILINEAR,
        )
        .map_err(|e| DecodeError::Failed(format!("Scaler init: {e}")))?;

        Ok(VideoDecoder {
            ictx,
            video_stream_index,
            decoder,
            scaler,
            time_base: time_base.0 as f64 / time_base.1 as f64,
        })
    }

    pub fn frame_rate(&self) -> f64 {
        if let Some(stream) = self
            .ictx
            .streams()
            .find(|s| s.index() == self.video_stream_index)
        {
            let r = stream.rate();
            if r.1 > 0 {
                return r.0 as f64 / r.1 as f64;
            }
        }
        30.0
    }

    pub fn duration(&self) -> f64 {
        self.ictx.duration() as f64 / ffmpeg::ffi::AV_TIME_BASE as f64
    }

    pub fn seek(&mut self, time: f64) -> Result<Option<DecodedFrame>, DecodeError> {
        let pts = (time / self.time_base) as i64;
        self.ictx
            .seek(pts, ..)
            .map_err(|e| DecodeError::Failed(format!("Seek: {e}")))?;
        self.decoder.flush();

        for (stream, packet) in self.ictx.packets() {
            if stream.index() == self.video_stream_index {
                self.decoder
                    .send_packet(&packet)
                    .map_err(|e| DecodeError::Failed(format!("Send packet: {e}")))?;
                let mut frame = ffmpeg::frame::Video::empty();
                loop {
                    match self.decoder.receive_frame(&mut frame) {
                        Ok(()) => {
                            let frame_pts = frame.pts().unwrap_or(0);
                            if frame_pts >= pts {
                                let rgb = self.scaler_to_rgba(&frame)?;
                                return Ok(Some(rgb));
                            }
                        }
                        Err(ffmpeg::Error::Other { errno }) if errno == ffmpeg::error::EAGAIN => {
                            break;
                        }
                        Err(_) => return Ok(None),
                    }
                }
            }
        }

        Ok(None)
    }

    pub fn next_frame(&mut self) -> Result<Option<DecodedFrame>, DecodeError> {
        for (stream, packet) in self.ictx.packets() {
            if stream.index() != self.video_stream_index {
                continue;
            }
            self.decoder
                .send_packet(&packet)
                .map_err(|e| DecodeError::Failed(format!("Send packet: {e}")))?;
            let mut frame = ffmpeg::frame::Video::empty();
            match self.decoder.receive_frame(&mut frame) {
                Ok(()) => {
                    let pts = frame.pts().unwrap_or(0) as f64 * self.time_base;
                    let mut rgb = self.scaler_to_rgba(&frame)?;
                    rgb.pts = pts;
                    return Ok(Some(rgb));
                }
                Err(ffmpeg::Error::Other { errno }) if errno == ffmpeg::error::EAGAIN => {}
                Err(_) => return Ok(None),
            }
        }
        Ok(None)
    }

    fn scaler_to_rgba(
        &mut self,
        frame: &ffmpeg::frame::Video,
    ) -> Result<DecodedFrame, DecodeError> {
        let mut rgb = ffmpeg::frame::Video::empty();
        self.scaler
            .run(frame, &mut rgb)
            .map_err(|e| DecodeError::Failed(format!("Scale: {e}")))?;

        let width = rgb.width();
        let height = rgb.height();
        let data = rgb.data(0).to_vec();

        Ok(DecodedFrame {
            data,
            width,
            height,
            pts: frame.pts().unwrap_or(0) as f64 * self.time_base,
        })
    }
}
