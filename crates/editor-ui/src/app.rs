use std::path::PathBuf;
use std::sync::mpsc;

use eframe::egui;
use egui::{ColorImage, TextureHandle, Vec2};
use rfd::FileDialog;

use editor_core::project::Project;
use media_engine::{DecodedFrame, VideoDecoder, export_timeline, ExportConfig};
use project_store::ProjectDatabase;

#[derive(Default)]
pub struct VideoEditorApp {
    pub project: Option<Project>,
    pub project_path: Option<PathBuf>,
    pub show_timeline: bool,
    pub show_media_browser: bool,
    pub show_preview: bool,

    // Video playback
    pub media_path: String,
    pub decoder: Option<VideoDecoder>,
    pub current_frame: Option<DecodedFrame>,
    pub texture: Option<TextureHandle>,
    pub is_playing: bool,
    pub current_time: f64,
    pub duration: f64,
    pub load_error: Option<String>,

    // Export
    pub is_exporting: bool,
    pub export_progress: (u64, u64),
    pub export_error: Option<String>,
    export_rx: Option<mpsc::Receiver<ExportEvent>>,
}

enum ExportEvent {
    Progress(u64, u64),
    Done,
    Error(String),
}

impl VideoEditorApp {
    pub fn new() -> Self {
        VideoEditorApp {
            project: None,
            project_path: None,
            show_timeline: true,
            show_media_browser: true,
            show_preview: true,
            media_path: String::new(),
            decoder: None,
            current_frame: None,
            texture: None,
            is_playing: false,
            current_time: 0.0,
            duration: 0.0,
            load_error: None,
            is_exporting: false,
            export_progress: (0, 0),
            export_error: None,
            export_rx: None,
        }
    }

    pub fn new_project(&mut self, name: &str) {
        self.project = Some(Project::new(name));
        self.project_path = None;
    }

    pub fn load_media(&mut self, path: &str) {
        self.load_error = None;
        match VideoDecoder::new(path) {
            Ok(decoder) => {
                self.media_path = path.to_string();
                self.decoder = Some(decoder);
                self.current_time = 0.0;
                self.is_playing = false;
                self.current_frame = None;
                self.texture = None;
                self.duration = 0.0;
            }
            Err(e) => {
                self.load_error = Some(e.to_string());
                self.decoder = None;
                self.current_frame = None;
                self.texture = None;
            }
        }
        if let Some(decoder) = &self.decoder {
            self.duration = decoder.duration();
        }
    }

    fn save_current_project(&mut self) -> Result<(), String> {
        let project = self.project.as_ref().ok_or("No project open")?;
        let path = self
            .project_path
            .clone()
            .or_else(|| {
                FileDialog::new()
                    .add_filter("Project", &["rveproj"])
                    .set_file_name(format!("{}.rveproj", project.name))
                    .save_file()
            })
            .ok_or("Save cancelled")?;

        let db = ProjectDatabase::open(&path.to_string_lossy())
            .map_err(|e| format!("Cannot open database: {e}"))?;
        db.initialize_schema()
            .map_err(|e| format!("Schema init: {e}"))?;
        db.save_project(project)
            .map_err(|e| format!("Save failed: {e}"))?;

        self.project_path = Some(path);
        tracing::info!("Project saved");
        Ok(())
    }

    fn open_project_file(&mut self, path: PathBuf) -> Result<(), String> {
        let db = ProjectDatabase::open(&path.to_string_lossy())
            .map_err(|e| format!("Cannot open database: {e}"))?;
        db.initialize_schema()
            .map_err(|e| format!("Schema init: {e}"))?;

        let projects = db.list_projects().map_err(|e| format!("List: {e}"))?;
        let summary = projects
            .first()
            .ok_or("No projects in database")?
            .clone();

        let project = db.load_project(summary.id).map_err(|e| format!("Load: {e}"))?;
        self.project = Some(project);
        self.project_path = Some(path);
        Ok(())
    }

    fn advance_frame(&mut self) {
        let decoder = match &mut self.decoder {
            Some(d) => d,
            None => return,
        };

        match decoder.next_frame() {
            Ok(Some(frame)) => {
                self.current_time = frame.pts;
                self.current_frame = Some(frame);
            }
            Ok(None) => {
                self.is_playing = false;
                let _ = decoder.seek(0.0);
                self.current_time = 0.0;
                self.current_frame = None;
            }
            Err(_) => {
                self.is_playing = false;
            }
        }
    }

    fn seek_to(&mut self, time: f64) {
        let decoder = match &mut self.decoder {
            Some(d) => d,
            None => return,
        };
        if let Ok(Some(frame)) = decoder.seek(time) {
            self.current_time = frame.pts;
            self.current_frame = Some(frame);
        }
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        let frame = match &self.current_frame {
            Some(f) => f,
            None => return,
        };

        let size = [frame.width as usize, frame.height as usize];
        let color_image = ColorImage::from_rgba_unmultiplied(size, &frame.data);

        self.texture = Some(ctx.load_texture(
            "preview",
            color_image,
            egui::TextureOptions::default(),
        ));
    }

    fn start_export(&mut self, config: ExportConfig) {
        let project = match &self.project {
            Some(p) => p.clone(),
            None => return,
        };
        let (tx, rx) = mpsc::channel();
        self.export_rx = Some(rx);
        self.is_exporting = true;
        self.export_progress = (0, 0);
        self.export_error = None;

        std::thread::spawn(move || {
            let result = export_timeline(&project.timeline, &config, |current, total| {
                let _ = tx.send(ExportEvent::Progress(current, total));
            });
            match result {
                Ok(()) => {
                    let _ = tx.send(ExportEvent::Done);
                }
                Err(e) => {
                    let _ = tx.send(ExportEvent::Error(e.to_string()));
                }
            }
        });
    }
}

impl eframe::App for VideoEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll export channel
        if let Some(rx) = self.export_rx.take() {
            let mut keep = true;
            while let Ok(event) = rx.try_recv() {
                match event {
                    ExportEvent::Progress(current, total) => {
                        self.export_progress = (current, total);
                    }
                    ExportEvent::Done => {
                        keep = false;
                        self.is_exporting = false;
                        self.export_progress = (0, 0);
                    }
                    ExportEvent::Error(msg) => {
                        keep = false;
                        self.is_exporting = false;
                        self.export_progress = (0, 0);
                        self.export_error = Some(msg);
                    }
                }
            }
            if keep {
                self.export_rx = Some(rx);
            }
        }

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project").clicked() {
                        self.new_project("Untitled");
                        ui.close_menu();
                    }
                    if ui.button("Open Project").clicked() {
                        let file = FileDialog::new()
                            .add_filter("Project", &["rveproj"])
                            .pick_file();
                        if let Some(path) = file {
                            if let Err(e) = self.open_project_file(path) {
                                self.load_error = Some(e);
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        if let Err(e) = self.save_current_project() {
                            self.load_error = Some(e);
                        }
                        ui.close_menu();
                    }
                    if ui.button("Save As...").clicked() {
                        self.project_path = None;
                        if let Err(e) = self.save_current_project() {
                            self.load_error = Some(e);
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_timeline, "Timeline");
                    ui.checkbox(&mut self.show_media_browser, "Media Browser");
                    ui.checkbox(&mut self.show_preview, "Preview");
                });
                ui.menu_button("Export", |ui| {
                    if ui.button("YouTube 1080p").clicked() {
                        if self.project.is_some() && !self.is_exporting {
                            self.start_export(ExportConfig::youtube_1080p("export_youtube.mp4"));
                        }
                        ui.close_menu();
                    }
                    if ui.button("TikTok Vertical").clicked() {
                        if self.project.is_some() && !self.is_exporting {
                            self.start_export(ExportConfig::tiktok("export_tiktok.mp4"));
                        }
                        ui.close_menu();
                    }
                });
            });
        });

        egui::SidePanel::left("media_browser")
            .resizable(true)
            .default_width(200.0)
            .show_animated(ctx, self.show_media_browser, |ui| {
                ui.heading("Media");
                ui.separator();

                if let Some(project) = &self.project {
                    ui.label(format!("Project: {}", project.name));
                    if let Some(p) = &self.project_path {
                        ui.label(format!("File: {}", p.display()));
                    }
                    ui.separator();
                }

                if ui.button("📁 Open Video File").clicked() {
                    let file = FileDialog::new()
                        .add_filter("Video", &["mp4", "avi", "mov", "mkv", "webm", "m4v"])
                        .add_filter("All", &["*"])
                        .pick_file();
                    if let Some(path) = file {
                        self.load_media(&path.to_string_lossy());
                    }
                }

                ui.separator();
                ui.label("Or enter path manually:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.media_path);
                    if ui.button("Load").clicked() {
                        let path = self.media_path.clone();
                        if !path.is_empty() {
                            self.load_media(&path);
                        }
                    }
                });

                if let Some(err) = &self.load_error {
                    ui.colored_label(egui::Color32::RED, err);
                }

                if self.decoder.is_some() {
                    ui.label(format!("Duration: {:.1}s", self.duration));
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.show_preview {
                egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                    let available = ui.available_size();

                    // Playback controls above preview
                    if self.decoder.is_some() {
                        ui.horizontal(|ui| {
                            let play_label = if self.is_playing { "⏸ Pause" } else { "▶ Play" };
                            if ui.button(play_label).clicked() {
                                self.is_playing = !self.is_playing;
                                if self.is_playing && self.current_frame.is_none() {
                                    self.seek_to(0.0);
                                }
                            }
                            if ui.button("⏮").clicked() {
                                self.seek_to(0.0);
                            }
                        });
                    }

                    let (_id, rect) = ui.allocate_space(available);

                    // Decode frames if playing
                    if self.is_playing {
                        self.advance_frame();
                        if let Some(decoder) = &self.decoder {
                            let frame_rate = decoder.frame_rate();
                            if frame_rate > 0.0 {
                                let frame_dur = std::time::Duration::from_secs_f64(1.0 / frame_rate);
                                ctx.request_repaint_after(frame_dur);
                            }
                        }
                    }

                    // Update texture if we have a new frame
                    if self.current_frame.is_some() {
                        self.update_texture(ctx);
                    }

                    // Draw preview
                    if let Some(tex) = &self.texture {
                        let tex_size = tex.size_vec2();
                        let scale = (available.x / tex_size.x)
                            .min(available.y / tex_size.y)
                            .min(2.0);
                        let draw_size = tex_size * scale;
                        let image = egui::Image::from_texture(tex)
                            .fit_to_exact_size(Vec2::new(draw_size.x, draw_size.y));
                        ui.put(rect, image);
                    } else {
                        let painter = ui.painter_at(rect);
                        painter.text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            if self.decoder.is_some() {
                                "No frame"
                            } else {
                                "Preview"
                            },
                            egui::TextStyle::Heading.resolve(ui.style()),
                            egui::Color32::GRAY,
                        );
                    }
                });
            }

            // Export progress overlay
            if self.is_exporting {
                egui::Window::new("Export Progress")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        let (current, total) = self.export_progress;
                        ui.label(format!("Exporting... {}/{} frames", current, total));
                        if total > 0 {
                            let progress = current as f64 / total as f64;
                            ui.add(
                                egui::ProgressBar::new(progress as f32)
                                    .show_percentage(),
                            );
                        }
                        if ui.button("Cancel").clicked() {
                            self.is_exporting = false;
                            self.export_rx = None;
                        }
                        ctx.request_repaint_after(std::time::Duration::from_millis(100));
                    });
            }

            if let Some(err) = &self.export_error.clone() {
                egui::Window::new("Export Error")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.colored_label(egui::Color32::RED, err);
                        if ui.button("OK").clicked() {
                            self.export_error = None;
                        }
                    });
            }
        });

        if self.show_timeline {
            egui::TopBottomPanel::bottom("timeline")
                .resizable(true)
                .default_height(200.0)
                .min_height(100.0)
                .show(ctx, |ui| {
                    ui.heading("Timeline");
                    ui.separator();

                    // Timeline seek bar
                    if self.decoder.is_some() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{:.1}s", self.current_time));
                            let mut progress = if self.duration > 0.0 {
                                self.current_time / self.duration
                            } else {
                                0.0
                            };
                            ui.add(
                                egui::Slider::new(&mut progress, 0.0..=1.0)
                                    .text("position")
                                    .suffix("%"),
                            );
                            ui.label(format!("{:.1}s", self.duration));
                            let new_time = progress * self.duration;
                            if (new_time - self.current_time).abs() > 0.1 {
                                self.seek_to(new_time);
                            }
                        });
                    }

                    if let Some(project) = &self.project {
                        ui.label(format!("Project: {}", project.name));
                        ui.label(format!(
                            "Duration: {:.1}s",
                            project.timeline.duration
                        ));
                        ui.label(format!(
                            "Tracks: {} video, {} audio",
                            project.timeline.video_tracks.len(),
                            project.timeline.audio_tracks.len()
                        ));
                    } else {
                        ui.label("No project open. File → New Project to start.");
                    }
                });
        }
    }
}
