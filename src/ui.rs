use eframe::egui;
use image::{RgbaImage, ImageReader, DynamicImage};
use pdfium_render::prelude::*;
use egui::{pos2, Color32, ColorImage, Rect, Vec2};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::Receiver;
use crate::watcher::FolderWatcher;
use std::sync::mpsc::{self};
use std::path::PathBuf;
const MAX_CACHE_SIZE: usize = 13;
pub const INITIAL_WIDTH: f32 = 900.0;
pub const INITIAL_HEIGHT: f32 = 600.0;
pub struct MyApp {
    new_images_rx: Receiver<String>,
    image_list: Vec<String>,
    template_image: Option<egui::TextureHandle>,
    current_image_texture: Option<egui::TextureHandle>,
    current_image_path: Option<String>,
    x_coordinate: String,
    y_coordinate: String,
    image_width: String,
    image_height: String,
    texture_cache: HashMap<String, egui::TextureHandle>,
    cache_order: VecDeque<String>,
    current_index: usize,
    folder_watcher: FolderWatcher,
    is_testing: bool,
    is_auto_work: bool,
}

impl MyApp {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let folder_watcher = FolderWatcher::new(tx);
        Self {
            new_images_rx: rx,
            image_list: Vec::new(),
            template_image: None,
            current_image_texture: None,
            current_image_path: None,
            x_coordinate: "150.0".to_string(),
            y_coordinate: "180.0".to_string(),
            image_width: "320.0".to_string(),
            image_height: "220.0".to_string(),
            texture_cache: HashMap::new(),
            cache_order: VecDeque::new(),
            current_index: 0,
            folder_watcher: folder_watcher,
            is_testing: true,
            is_auto_work: false,
        }
    }

    fn load_template_pdf(&mut self, ctx: &egui::Context, pdf_path: &str) {
        if let Some(image) = render_pdf_page_to_image(pdf_path, 0) {
            let size = [image.width() as usize, image.height() as usize];
            let pixels = image.to_vec();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
            self.template_image = Some(ctx.load_texture("template_pdf", color_image, egui::TextureOptions::default()));
        }
    }

    fn update_cache(&mut self, ctx: &egui::Context) {
        let len = self.image_list.len();
        let start = if len >= MAX_CACHE_SIZE { len - MAX_CACHE_SIZE } else { 0 };
        let recent_paths: Vec<String> = self.image_list[start..].to_vec();

        for path in &recent_paths {
            if !self.texture_cache.contains_key(path) {
                if let Ok(texture) = load_image_from_path(path, ctx) {
                    self.texture_cache.insert(path.clone(), texture);
                    self.cache_order.push_back(path.clone());
                    if self.cache_order.len() > MAX_CACHE_SIZE {
                        if let Some(old_path) = self.cache_order.pop_front() {
                            self.texture_cache.remove(&old_path);
                        }
                    }
                }
            }
        }
    }

    fn get_texture(&self, path: &str) -> Option<&egui::TextureHandle> {
        self.texture_cache.get(path)
    }

    
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut should_repaint = false;
        if self.is_testing {
            self.load_template_pdf(ctx, "./Берлін 1.pdf");
            if let Err(e) = self.folder_watcher.spawn_watcher(PathBuf::from("./folder_to_monitor")) {
                eprintln!("Failed to spawn watcher: {:?}", e);
            }
            self.image_list.push("/Users/alex/Developer/photo_qt/folder_to_monitor/Screenshot 2025-05-10 at 12.15.44.png".to_string());
            self.update_cache(ctx);
            self.is_testing = false;
            should_repaint = true;
        }
        

        // Drain new image notifications
        while let Ok(path) = self.new_images_rx.try_recv() {
            eprintln!("New image path received: {}", path);
            self.image_list.push(path.clone());
            self.update_cache(ctx);
            self.current_index = self.image_list.len() - 1;
            should_repaint = true;
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Layout").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("PDF files", &["pdf"]).pick_file() {
                        self.load_template_pdf(ctx, &path.to_string_lossy());
                        should_repaint = true;
                    }
                }
                if ui.button("Folder").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        if let Err(e) = self.folder_watcher.spawn_watcher(path) {
                            eprintln!("Failed to watch folder: {:?}", e);
                        }
                    }
                }
                if ui.button("Print").clicked() {
                    
                }
            });
        });

        if let Some(path) = self.image_list.get(self.current_index).cloned() {
            if let Some(texture) = self.get_texture(&path) {
                if self.current_image_path.as_deref() != Some(&path) {
                    self.current_image_texture = Some(texture.clone());
                    self.current_image_path = Some(path.clone());
                    should_repaint = true;
                }
            } else if let Ok(img) = load_image_from_path(&path, ctx) {
                self.current_image_texture = Some(img);
                self.current_image_path = Some(path.clone());
                should_repaint = true;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let total_width = ui.available_width();
            let left_width = total_width * 0.5;
            let right_width = total_width * 0.5;
            let height = ui.available_height();

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(left_width);
                    ui.set_height(height);
                    if let Some(template) = &self.template_image {
                        
                        
                        draw_full_image(ui, template);
                    }
                    if let (Some(template), Some(current)) = (&self.template_image, &self.current_image_texture) {
                        // let x = self.x_coordinate.parse::<f32>().unwrap_or(190.0);
                        // let y = self.y_coordinate.parse::<f32>().unwrap_or(210.0);
                        // let w = self.image_width.parse::<f32>().unwrap_or(320.0);
                        // let h = self.image_height.parse::<f32>().unwrap_or(220.0);
                        

                        let aspect_ratio = template.size()[0] as f32 / template.size()[1] as f32;
                        
                        let template_w : f32 = f32::min(aspect_ratio * height, left_width);
                        let template_h : f32 = f32::min(left_width / aspect_ratio, height);

                        let top_left = ui.min_rect().left_top();
                        let x= top_left[0] + 0.360 * template_w;
                        let y = top_left[1] + 0.30 * template_h;
                        let w = template_w * 0.605;
                        let h = template_h * 0.25;

                        
                        
                        ui.painter().image(
                            current.id(),
                            Rect::from_min_size(pos2(x, y), Vec2::new(w, h)),
                            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                });
                ui.vertical(|ui| {
                    ui.set_width(right_width); 
                    ui.add_space(20.0);
                    ui.label("X coordinate");
                    should_repaint |= ui.text_edit_singleline(&mut self.x_coordinate).changed();
                    ui.label("Y coordinate");
                    should_repaint |= ui.text_edit_singleline(&mut self.y_coordinate).changed();
                    ui.label("Image width");
                    should_repaint |= ui.text_edit_singleline(&mut self.image_width).changed();
                    ui.label("Image height");
                    should_repaint |= ui.text_edit_singleline(&mut self.image_height).changed();

                    if ui.button("AutoWork").clicked() {
                        self.is_auto_work = true;
                    }

                    if ui.button("Stop").clicked() {
                        self.is_auto_work = false;
                    }

                    ui.horizontal(|ui| {
                        if ui.button("<-").clicked() && self.current_index > 0 {
                            self.current_index -= 1; should_repaint = true;
                        }
                        if ui.button("->").clicked() && self.current_index + 1 < self.image_list.len() {
                            self.current_index += 1; should_repaint = true;
                        }
                    });
                    ui.separator();
                    ui.label(format!("Index: {}", self.current_index));
                });
            });
        });

        if should_repaint {
            ctx.request_repaint();
        }
    }
}

fn load_image_from_path(path: &str, ctx: &egui::Context) -> Result<egui::TextureHandle, String> {
    let img = ImageReader::open(path)
        .map_err(|e| format!("Failed to open image: {}", e))?
        .decode()
        .map_err(|e| format!("Failed to decode image: {}", e))?;
    let gray_dynamic: DynamicImage = img.grayscale();
    let gray_buf = gray_dynamic.into_luma8();
    let (w, h) = gray_buf.dimensions();
    let size = [w as usize, h as usize];
    let luma_pixels = gray_buf.into_raw();
    let mut rgba_pixels = Vec::with_capacity(w as usize * h as usize * 4);
    for &l in &luma_pixels { rgba_pixels.extend_from_slice(&[l, l, l, 255]); }
    let color_image = ColorImage::from_rgba_unmultiplied(size, &rgba_pixels);
    Ok(ctx.load_texture(path, color_image, egui::TextureOptions::default()))
}

fn render_pdf_page_to_image(pdf_path: &str, page_num: usize) -> Option<RgbaImage> {
    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./")).unwrap()
    );
    let document = pdfium.load_pdf_from_file(pdf_path, None).ok()?;
    let page = document.pages().get(page_num as u16).ok()?;
    let render_config = PdfRenderConfig::new()
        .set_target_width(2000)
        .set_maximum_height(2000);
    let image = page.render_with_config(&render_config).ok()?.as_image().into_rgba8();
    if image.is_empty() { eprintln!("Failed to render PDF page."); return None; }
    Some(image)
}

fn draw_full_image(ui: &mut egui::Ui, image: &egui::TextureHandle) {
    ui.add(egui::Image::new(image).fit_to_exact_size(ui.available_size()));
}
