use eframe::egui;
use image::{Rgba, ImageReader, DynamicImage, ImageBuffer, RgbaImage};
use std::{
    path::Path,
    env,
    sync::mpsc::{self, Sender, Receiver},
};
use egui::{pos2, Color32, ColorImage, Rect, Vec2, Button, Stroke};
use std::collections::{HashMap, VecDeque};
use crate::watcher::FolderWatcher;
use crate::printer::PdfImageInserter;
use crate::pdfwrap::{Library, BitmapFormat, PageOrientation, rendering_flags};


const MAX_CACHE_SIZE: usize = 13;
pub const INITIAL_WIDTH: f32 = 900.0;
pub const INITIAL_HEIGHT: f32 = 600.0;
pub struct MyApp {
    new_images_rx: Receiver<String>,
    image_list: Vec<String>,
    template_path: Option<String>,
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
    image_inserter : Option<Sender<String>>,
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
            template_path: None,
            template_image: None,
            current_image_texture: None,
            current_image_path: None,
            x_coordinate: "215.0".to_string(),
            y_coordinate: "380.0".to_string(),
            image_width: "360.0".to_string(),
            image_height: "220.0".to_string(),
            texture_cache: HashMap::new(),
            cache_order: VecDeque::new(),
            current_index: 0,
            folder_watcher: folder_watcher,
            image_inserter : None,
            is_testing: true,
            is_auto_work: false,
        }
    }

    fn load_template_pdf(&mut self, ctx: &egui::Context, pdf_path: &str) {
        if let Some(image) = render_pdf_page_to_image(pdf_path) {
            self.template_path = Some(pdf_path.to_string());
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
        let is_focused = ctx.input(|i| i.raw.focused);
        let mut should_repaint = false;
        if self.is_testing {
            let current_dir = env::current_dir().unwrap();
            self.load_template_pdf(ctx, "Berlin.pdf");
            if let Err(e) = self.folder_watcher.spawn_watcher(current_dir.join("folder_to_monitor")) {
                eprintln!("Failed to spawn watcher: {:?}", e);
            }
            self.image_list.push(current_dir.join("folder_to_monitor").join("test_image.jpeg").to_string_lossy().to_string());
            self.update_cache(ctx);
            self.is_testing = false;
            should_repaint = true;
        }
        

        while let Ok(path) = self.new_images_rx.try_recv() {
            // eprintln!("New image path received: {}", path);
            self.image_list.push(path.clone());
            self.update_cache(ctx);
            self.current_index = self.image_list.len() - 1;

            println!("Image List: {:?}", self.image_list);

            if self.is_auto_work {

                if self.image_inserter.is_none() {
                    let x: f32 = self.x_coordinate.parse().unwrap_or(215.0);
                    let y: f32 = self.y_coordinate.parse().unwrap_or(380.0);
                    let w: f32 = self.image_width.parse().unwrap_or(360.0);
                    let h: f32 = self.image_height.parse().unwrap_or(220.0);
                    
                    let inserter_tx = PdfImageInserter::new_and_spawn(self.template_path.as_ref().expect("No Template is Selected").clone(), x, y, w, h);
                    self.image_inserter = Some(inserter_tx);
                }
                if let Err(e) = self.image_inserter.as_ref().unwrap().send(path.clone()) {
                    eprintln!("Failed to send print job: {}", e);
                }
            }
            should_repaint = true;
        }


        if is_focused {
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
                    if let (Some(temp), Some(img)) = (&self.template_path, &self.current_image_path) {

                        

                        if self.image_inserter.is_none() {
                            let x: f32 = self.x_coordinate.parse().unwrap_or(215.0);
                            let y: f32 = self.y_coordinate.parse().unwrap_or(380.0);
                            let w: f32 = self.image_width.parse().unwrap_or(360.0);
                            let h: f32 = self.image_height.parse().unwrap_or(220.0);

                            let inserter_tx = PdfImageInserter::new_and_spawn(temp.clone(), x, y, w, h);
                            self.image_inserter = Some(inserter_tx);
                        }

                        if let Err(e) = self.image_inserter.as_ref().unwrap().send(img.clone()) {
                            eprintln!("Failed to send print job: {}", e);
                        }
                        
                    }

                }
            });
        });

        if let Some(path) = self.image_list.get(self.current_index).cloned() {
            if self.current_image_path.as_deref() != Some(&path) {
                if let Some(texture) = self.get_texture(&path) {
                    self.current_image_texture = Some(texture.clone());
                    self.current_image_path = Some(path.clone());
                    should_repaint = true;
                }
                else if let Ok(img) = load_image_from_path(&path, ctx) {
                    self.current_image_texture = Some(img);
                    self.current_image_path = Some(path.clone());
                    should_repaint = true;
                }
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


    //                     /// Assign a new image to a subregion of the whole texture.
    // #[allow(clippy::needless_pass_by_ref_mut)] // Intentionally hide interiority of mutability
    // pub fn set_partial(
    //     &mut self,
    //     pos: [usize; 2],
    //     image: impl Into<ImageData>,
    //     options: TextureOptions,
    // ) {
    //     self.tex_mngr
    //         .write()
    //         .set(self.id, ImageDelta::partial(pos, image.into(), options));
    // }

                    if let Some(template) = &self.template_image {
                        //TODO:
                        //Each frame we create Image::new, can get rid of that
                        //Can implement ImageLoader for egui 
                        draw_full_image(ui, template);
                    }
                    if let (Some(template), Some(current)) = (&self.template_image, &self.current_image_texture) {
                        

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

                    let auto_work_button = if self.is_auto_work {
                        Button::new("AutoWork").stroke(Stroke::new(1.5, Color32::LIGHT_BLUE))
                    } else {
                        Button::new("AutoWork")
                    };

                    let stop_button = if !self.is_auto_work {
                        Button::new("Stop").stroke(Stroke::new(1.5, Color32::LIGHT_BLUE))
                    } else {
                        Button::new("Stop")
                    };
                    

                    if ui.add(auto_work_button).clicked() {
                        self.is_auto_work = true;
                    }

                    if ui.add(stop_button).clicked() {
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
    }
        if should_repaint {
            // println!("Repainting!");
            ctx.request_repaint();
        } else if !is_focused {
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
    }
}

fn load_image_from_path(path: &str, ctx: &egui::Context) -> Result<egui::TextureHandle, String> {
    let img = ImageReader::open(path)
        .map_err(|e| format!("Failed to open image: {}", e))?
        .decode()
        .map_err(|e| format!("Failed to decode image: {}", e))?;

    let max_dim = 200u32;
    let (orig_w, orig_h) = (img.width(), img.height());
    let (new_w, new_h) = if orig_w > orig_h {
        (max_dim, (orig_h * max_dim) / orig_w)
    } else {
        ((orig_w * max_dim) / orig_h, max_dim)
    };
    let resized = img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);

    let gray_dynamic: DynamicImage = resized.grayscale();
    let gray_buf = gray_dynamic.into_luma8();
    let (w, h) = gray_buf.dimensions();
    let size = [w as usize, h as usize];
    let luma_pixels = gray_buf.into_raw();
    let mut rgba_pixels = Vec::with_capacity(w as usize * h as usize * 4);
    for &l in &luma_pixels {
        rgba_pixels.extend_from_slice(&[l, l, l, 255]);
    }
    let color_image = ColorImage::from_rgba_unmultiplied(size, &rgba_pixels);
    Ok(ctx.load_texture(path, color_image, egui::TextureOptions::default()))
}


fn render_pdf_page_to_image(pdf_path: &str) -> Option<RgbaImage> {

    let library = Library::init_library()?;

    let path = Path::new(pdf_path);

    let document = library.load_document(&path, None).ok()?;
    let page = library.load_page(&document, 0).ok()?;

    let width = library.get_page_width(&page).round() as usize;
    let height = library.get_page_height(&page).round() as usize;

    let format = BitmapFormat::BGRA;
    let stride = width * format.bytes_per_pixel();
    let mut buffer = vec![0; height * stride]; // Initialize with zeros (black)
    
    // Create a bitmap from our buffer
    let mut bitmap = library
        .create_bitmap_from_buffer(width, height, format, &mut buffer, stride)
        .ok()?;
    let color :u64 = 0xFFFFFFFF;
    // Fill the bitmap with white background
    library.bitmap_fill_rect(&mut bitmap, 0, 0, width as i32, height as i32, color);
    
    // Render the page to our bitmap
    library.render_page_to_bitmap(
        &mut bitmap,
        &page,
        0,
        0,
        width as i32,
        height as i32,
        PageOrientation::Normal,
        rendering_flags::NORMAL,
    );
    
    let bgra_data = library.get_bitmap_buffer(&bitmap);
    

    let mut rgba_data = Vec::with_capacity(bgra_data.len());
    for chunk in bgra_data.chunks_exact(4) {
        rgba_data.push(chunk[2]); // R (was B)
        rgba_data.push(chunk[1]); // G (stays G)
        rgba_data.push(chunk[0]); // B (was R)
        rgba_data.push(chunk[3]); // A (stays A)
    }
    
    let image: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(width as u32, height as u32, rgba_data).unwrap();
    if image.is_empty() { eprintln!("Failed to render PDF page."); return None; }
    Some(image)
}

fn draw_full_image(ui: &mut egui::Ui, image: &egui::TextureHandle) {
    ui.add(egui::Image::new(image).fit_to_exact_size(ui.available_size()));
}
