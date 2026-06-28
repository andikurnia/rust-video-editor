use serde::{Deserialize, Serialize};

use crate::timeline::Timeline;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: String,
    pub path: String,
    pub name: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPreset {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
    pub bitrate: String,
    pub codec: String,
    pub container: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub timeline: Timeline,
    pub media: Vec<MediaItem>,
}

impl Project {
    pub fn new(name: &str) -> Self {
        Project {
            name: name.to_string(),
            timeline: Timeline::new(),
            media: Vec::new(),
        }
    }
}
