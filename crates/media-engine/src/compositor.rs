use std::collections::HashMap;

use editor_core::Timeline;

use crate::decode::{DecodedFrame, VideoDecoder};

pub struct FrameCompositor {
    decoders: HashMap<String, VideoDecoder>,
}

impl FrameCompositor {
    pub fn new() -> Self {
        FrameCompositor {
            decoders: HashMap::new(),
        }
    }

    fn get_decoder(&mut self, path: &str) -> Result<&mut VideoDecoder, String> {
        if !self.decoders.contains_key(path) {
            let decoder = VideoDecoder::new(path)
                .map_err(|e| format!("Cannot open {}: {e}", path))?;
            self.decoders.insert(path.to_string(), decoder);
        }
        Ok(self.decoders.get_mut(path).unwrap())
    }

    pub fn composite_frame(
        &mut self,
        timeline: &Timeline,
        time: f64,
        width: u32,
        height: u32,
    ) -> Result<DecodedFrame, String> {
        let size = (width * height * 4) as usize;
        let mut canvas = vec![0u8; size];

        for track in &timeline.video_tracks {
            for clip in &track.clips {
                if time < clip.start || time >= clip.end() {
                    continue;
                }
                let source_time = clip.source_in + (time - clip.start);
                let decoder = self.get_decoder(&clip.source_path)?;

                if let Ok(Some(frame)) = decoder.seek(source_time) {
                    self.overlay(&mut canvas, width, height, &frame);
                }
            }
        }

        Ok(DecodedFrame {
            data: canvas,
            width,
            height,
            pts: time,
        })
    }

    fn overlay(
        &self,
        canvas: &mut [u8],
        out_w: u32,
        out_h: u32,
        frame: &DecodedFrame,
    ) {
        for y in 0..out_h {
            for x in 0..out_w {
                let sx = (x * frame.width / out_w) as usize;
                let sy = (y * frame.height / out_h) as usize;
                let si = (sy * frame.width as usize + sx) * 4;
                let di = (y as usize * out_w as usize + x as usize) * 4;

                let a = frame.data[si + 3];
                if a == 255 {
                    canvas[di..di + 4].copy_from_slice(&frame.data[si..si + 4]);
                } else if a > 0 {
                    let inv = 255 - a;
                    for c in 0..3 {
                        canvas[di + c] = ((frame.data[si + c] as u16 * a as u16
                            + canvas[di + c] as u16 * inv as u16)
                            / 255) as u8;
                    }
                    canvas[di + 3] = 255;
                }
            }
        }
    }
}

impl Default for FrameCompositor {
    fn default() -> Self {
        Self::new()
    }
}
