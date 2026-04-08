use eframe::egui;
use egui::ColorImage;
use image::{imageops::FilterType, DynamicImage, ImageReader, RgbaImage};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::{elevation, i18n::Language, registry, wallpaper_style::WallpaperStyle};

/// Dimensions of the wallpaper preview rendered inside the monitor mockup.
const PREVIEW_W: u32 = 316;
const PREVIEW_H: u32 = 198;

struct PreviewResult {
    request_id: u64,
    rgba: Result<Vec<u8>, String>,
}

pub struct WallpaperApp {
    /// Current UI language (derived from user locale at startup).
    lang: Language,
    /// Path selected by the user (or loaded from the registry on startup).
    wallpaper_path: Option<PathBuf>,
    /// Currently selected display style.
    style: WallpaperStyle,
    /// egui texture used for the monitor preview.
    preview_texture: Option<egui::TextureHandle>,
    /// Monotonic id for the latest preview request.
    active_preview_request: u64,
    /// Channel endpoints used by background preview workers.
    preview_tx: Sender<PreviewResult>,
    preview_rx: Receiver<PreviewResult>,
    /// Status message shown at the bottom of the window.
    status: Option<(String, bool)>, // (text, is_error)
    /// Set to true when the user clicks "Browse…"; consumed at the next frame.
    pending_file_dialog: bool,
}

impl WallpaperApp {
    pub fn new(lang: Language) -> Self {
        // Pre-populate from whatever is already in the registry.
        let (wallpaper_str, style) = registry::get_current_wallpaper().unwrap_or((None, None));
        let wallpaper_path = wallpaper_str.map(PathBuf::from).filter(|p| p.is_file());
        let style = style.unwrap_or_default();
        let (preview_tx, preview_rx) = mpsc::channel();

        let mut app = Self {
            lang,
            wallpaper_path,
            style,
            preview_texture: None,
            active_preview_request: 0,
            preview_tx,
            preview_rx,
            status: None,
            pending_file_dialog: false,
        };

        if app.wallpaper_path.is_some() {
            app.request_preview();
        }

        app
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    fn request_preview(&mut self) {
        let Some(path) = self.wallpaper_path.clone() else {
            self.preview_texture = None;
            return;
        };

        self.active_preview_request = self.active_preview_request.wrapping_add(1);
        let request_id = self.active_preview_request;
        let style = self.style;
        let tx = self.preview_tx.clone();

        thread::spawn(move || {
            let rgba = load_preview_rgba(&path, style).map_err(|e| e.to_string());
            let _ = tx.send(PreviewResult { request_id, rgba });
        });
    }

    fn apply_preview_results(&mut self, ctx: &egui::Context) {
        let mut updated = false;

        while let Ok(result) = self.preview_rx.try_recv() {
            if result.request_id != self.active_preview_request {
                continue;
            }

            match result.rgba {
                Ok(rgba) => {
                    let color_image = ColorImage::from_rgba_unmultiplied(
                        [PREVIEW_W as usize, PREVIEW_H as usize],
                        &rgba,
                    );
                    self.preview_texture = Some(ctx.load_texture(
                        "wallpaper-preview",
                        color_image,
                        egui::TextureOptions::LINEAR,
                    ));
                }
                Err(err) => {
                    self.status = Some((self.lang.failed_load_image(err), true));
                }
            }
            updated = true;
        }

        if updated {
            ctx.request_repaint();
        }
    }

    fn handle_file_dialog(&mut self, ctx: &egui::Context) {
        if !self.pending_file_dialog {
            return;
        }
        self.pending_file_dialog = false;

        if let Some(path) = rfd::FileDialog::new()
            .add_filter(self.lang.images_filter(), &["jpg", "jpeg", "png", "bmp"])
            .pick_file()
        {
            self.wallpaper_path = Some(path);
            self.request_preview();
            self.status = None;
        }
        ctx.request_repaint();
    }

    fn apply(&mut self) {
        let Some(path) = self.wallpaper_path.clone() else {
            self.status = Some((self.lang.no_wallpaper_selected().into(), true));
            return;
        };
        if !path.is_file() {
            self.status = Some((self.lang.file_no_longer_exists().into(), true));
            return;
        }

        let sid = match elevation::current_user_sid() {
            Ok(s) => s,
            Err(e) => {
                self.status = Some((self.lang.failed_resolve_sid(e), true));
                return;
            }
        };

        if elevation::is_elevated() {
            match registry::set_wallpaper_for_sid(&sid, &path, self.style) {
                Ok(()) => {
                    // Best-effort: refresh the current session's desktop.
                    let _ = registry::refresh_wallpaper_session(&path);
                    self.status = Some((self.lang.wallpaper_applied().into(), false));
                }
                Err(e) => {
                    self.status = Some((self.lang.failed_to_apply(e), true));
                }
            }
            return;
        }

        let broker_args = vec![
            "--target-sid".to_owned(),
            sid,
            "--wallpaper".to_owned(),
            path.to_string_lossy().into_owned(),
            "--style".to_owned(),
            self.style.code().to_owned(),
        ];

        match elevation::run_elevated_with_args(&broker_args) {
            Ok(0) => {
                // Best-effort: refresh the current session's desktop.
                let _ = registry::refresh_wallpaper_session(&path);
                self.status = Some((self.lang.wallpaper_applied().into(), false));
            }
            Ok(code) => {
                self.status = Some((self.lang.elevated_broker_failed(code), true));
            }
            Err(e) => {
                self.status = Some((self.lang.elevation_failed(e), true));
            }
        }
    }
}

impl eframe::App for WallpaperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle deferred actions before painting.
        self.handle_file_dialog(ctx);
        self.apply_preview_results(ctx);

        let has_image = self.wallpaper_path.is_some();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(16.0);

            // ── Title ──────────────────────────────────────────────────────
            ui.heading(self.lang.heading());
            ui.add_space(12.0);

            // ── File picker row ────────────────────────────────────────────
            ui.label(self.lang.choose_picture());
            ui.horizontal(|ui| {
                let path_display = self
                    .wallpaper_path
                    .as_deref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|| self.lang.empty_path().into());

                let mut buf = path_display.clone();
                ui.add(
                    egui::TextEdit::singleline(&mut buf)
                        .desired_width(320.0)
                        .interactive(false),
                );

                if ui.button(self.lang.browse_button()).clicked() {
                    self.pending_file_dialog = true;
                }
            });

            ui.add_space(12.0);

            // ── Monitor preview ────────────────────────────────────────────
            let monitor_size = egui::vec2(356.0, 232.0);
            let (_, monitor_rect) = ui.allocate_space(monitor_size);

            let painter = ui.painter();
            // Monitor bezel
            painter.rect_filled(monitor_rect, 6.0, egui::Color32::from_gray(55));
            // Screen area
            let screen = monitor_rect.shrink(12.0);
            painter.rect_filled(screen, 2.0, egui::Color32::from_gray(12));

            if let Some(tex) = &self.preview_texture {
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                painter.image(tex.id(), screen, uv, egui::Color32::WHITE);
            }

            ui.add_space(12.0);

            // ── Style selector ─────────────────────────────────────────────
            ui.label(self.lang.choose_fit());
            ui.add_enabled_ui(has_image, |ui| {
                let old_style = self.style;
                egui::ComboBox::from_id_salt("wallpaper-style")
                    .selected_text(self.style.label(self.lang))
                    .show_ui(ui, |ui| {
                        for &s in WallpaperStyle::all() {
                            ui.selectable_value(&mut self.style, s, s.label(self.lang));
                        }
                    });
                if self.style != old_style {
                    self.request_preview();
                }
            });

            ui.add_space(16.0);

            // ── Action buttons ─────────────────────────────────────────────
            ui.horizontal(|ui| {
                ui.add_enabled_ui(has_image, |ui| {
                    if ui.button(self.lang.apply_button()).clicked() {
                        self.apply();
                    }
                });
                if ui.button(self.lang.close_button()).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            // ── Status message ─────────────────────────────────────────────
            if let Some((msg, is_error)) = &self.status {
                ui.add_space(8.0);
                let color = if *is_error {
                    egui::Color32::from_rgb(220, 70, 70)
                } else {
                    egui::Color32::from_rgb(60, 200, 80)
                };
                ui.colored_label(color, msg);
            }
        });
    }
}

fn load_preview_rgba(path: &Path, style: WallpaperStyle) -> anyhow::Result<Vec<u8>> {
    let img = ImageReader::open(path)?.with_guessed_format()?.decode()?;
    Ok(render_preview(&img, style, PREVIEW_W, PREVIEW_H))
}

// ── Preview rendering ─────────────────────────────────────────────────────────

/// Render `img` into a `width × height` RGBA buffer according to `style`.
fn render_preview(img: &DynamicImage, style: WallpaperStyle, width: u32, height: u32) -> Vec<u8> {
    // Dark background, matching typical desktop defaults.
    let bg_pixel = image::Rgba([20u8, 20, 20, 255]);
    let mut canvas = RgbaImage::new(width, height);
    for p in canvas.pixels_mut() {
        *p = bg_pixel;
    }

    match style {
        WallpaperStyle::Stretch | WallpaperStyle::Span => {
            let resized = img.resize_exact(width, height, FilterType::Lanczos3);
            image::imageops::overlay(&mut canvas, &resized.to_rgba8(), 0, 0);
        }
        WallpaperStyle::Center => {
            let rgba = img.to_rgba8();
            let (iw, ih) = rgba.dimensions();
            let x = (width as i64 - iw as i64) / 2;
            let y = (height as i64 - ih as i64) / 2;
            image::imageops::overlay(&mut canvas, &rgba, x, y);
        }
        WallpaperStyle::Fit => {
            let resized = img.resize(width, height, FilterType::Lanczos3);
            let rgba = resized.to_rgba8();
            let (rw, rh) = rgba.dimensions();
            let x = (width as i64 - rw as i64) / 2;
            let y = (height as i64 - rh as i64) / 2;
            image::imageops::overlay(&mut canvas, &rgba, x, y);
        }
        WallpaperStyle::Fill => {
            let resized = img.resize_to_fill(width, height, FilterType::Lanczos3);
            image::imageops::overlay(&mut canvas, &resized.to_rgba8(), 0, 0);
        }
        WallpaperStyle::Tile => {
            let rgba = img.to_rgba8();
            let (iw, ih) = rgba.dimensions();
            if iw > 0 && ih > 0 {
                let mut ty: i64 = 0;
                while ty < height as i64 {
                    let mut tx: i64 = 0;
                    while tx < width as i64 {
                        image::imageops::overlay(&mut canvas, &rgba, tx, ty);
                        tx += iw as i64;
                    }
                    ty += ih as i64;
                }
            }
        }
    }

    canvas.into_raw()
}
