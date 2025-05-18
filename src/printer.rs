use lopdf::{Document, Object, Dictionary, Stream, Error as LopdfError};
use lopdf::content::{Content, Operation};
use image::{ImageReader, DynamicImage};
use std::{
    error::Error,
    sync::mpsc::{self, Sender, Receiver},
    thread,
    env,
    path::PathBuf
};


#[cfg(target_os = "windows")]
mod printer_windows;
#[cfg(target_os = "windows")]
pub use printer_windows::print_document_windows;

#[cfg(target_os = "macos")]
mod printer_macos;
#[cfg(target_os = "macos")]
pub use printer_macos::print_document_macos;


fn get_or_create_xobject(
    resources_dict: &mut Dictionary,
) -> Result<&mut Dictionary, LopdfError> {
    if !resources_dict.has(b"XObject") {
        resources_dict.set(b"XObject", Object::Dictionary(Dictionary::new()));
    }
    let xobj = resources_dict.get_mut(b"XObject")?;
    xobj.as_dict_mut()
}




pub struct PdfImageInserter {
    template_path: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl PdfImageInserter {

    fn new(template_path: String, x: f32, y: f32, width: f32, height: f32) -> Self {
            let inserter = PdfImageInserter {
                template_path: template_path.into(),
                x,
                y,
                width,
                height, 
            };
            inserter
        }

    pub fn save_pdf_path(&self) -> PathBuf {
        env::current_dir().unwrap().join("print_doc.pdf")
    }
    pub fn new_and_spawn(template_path: String, x: f32, y: f32, width: f32, height: f32) -> Sender<String> {
            let (tx, rx) : (Sender<String>, Receiver<String>) = mpsc::channel();
            let inserter = PdfImageInserter::new(template_path, x, y, width, height);

            thread::spawn(move || { 
                for img in rx {
                        let save_pdf_path = inserter.save_pdf_path();
                        println!("New PDF Path: {:?}", save_pdf_path);
                        println!("Image Path: {:?}", img);
                        if let Err(insert_err) = inserter.insert_image(&img, &save_pdf_path) {
                            eprintln!("Insert error: {}", insert_err);
                        } else {

                            let _ = inserter.print(&save_pdf_path);
                            std::fs::remove_file(save_pdf_path).unwrap();
                        }
                }
            });
            tx
        }
    

    

    fn insert_image(&self, image_path: &str, output_path: &PathBuf) -> Result<(), Box<dyn Error>> {

        let mut doc = Document::load(&self.template_path).expect("Can't open the template to print");

        let img = ImageReader::open(image_path).expect("Can't open the image to print")
        .decode().expect("Can't decode the image to print");
        let gray_dynamic: DynamicImage = img.grayscale();
        let gray_buf = gray_dynamic.to_luma8();
        let (w_px, h_px) = gray_buf.dimensions();

        let mut img_dict = Dictionary::new();
        img_dict.set("Type", Object::Name(b"XObject".to_vec()));
        img_dict.set("Subtype", Object::Name(b"Image".to_vec()));
        img_dict.set("Width", Object::Integer(w_px as i64));
        img_dict.set("Height", Object::Integer(h_px as i64));
        img_dict.set("ColorSpace", Object::Name(b"DeviceGray".to_vec()));
        img_dict.set("BitsPerComponent", Object::Integer(8));

        let img_stream = Stream::new(img_dict, gray_buf.into_raw());
        let img_obj_id = doc.add_object(img_stream);

        let pages = doc.get_pages();
        if pages.is_empty() {
            return Err("PDF has no pages".into());
        }
        let (&_, &page_id) = pages.iter().next().unwrap();

        {
            let page_dict = doc.get_dictionary_mut(page_id)?;
            let res_obj = page_dict.get_mut(b"Resources")?;
            let resources_dict = res_obj
                .as_dict_mut()
                .map_err(|_| LopdfError::Type)?;
            let xobj_dict = get_or_create_xobject(resources_dict)?;
            xobj_dict.set(b"Im1".to_vec(), Object::Reference(img_obj_id));
        }

        let mut content = Content { operations: vec![] };
        content.operations.push(Operation::new("q", vec![]));
        content.operations.push(Operation::new(
            "cm",
            vec![
                self.width.into(),
                0.into(),
                0.into(),
                self.height.into(),
                self.x.into(),
                self.y.into(),
            ],
        ));
        content
            .operations
            .push(Operation::new("Do", vec![Object::Name(b"Im1".to_vec())]));
        content.operations.push(Operation::new("Q", vec![]));

        let content_stream = Stream::new(Dictionary::new(), content.encode()?);
        let content_id = doc.add_object(content_stream);

        {
            let page_dict = doc.get_dictionary_mut(page_id)?;
            let existing = page_dict.get(b"Contents")?;
            let new_obj = match existing {
                Object::Reference(rid) => Object::Array(vec![
                    Object::Reference(*rid),
                    Object::Reference(content_id),
                ]),
                Object::Array(arr) => {
                    let mut arr = arr.clone();
                    arr.push(Object::Reference(content_id));
                    Object::Array(arr)
                }
                _ => Object::Reference(content_id),
            };
            page_dict.set("Contents", new_obj);
        }
        
        doc.save(&output_path).expect("Cant save the printed document to the output path");
        Ok(())
    }
    

    fn print(&self, save_doc_path: &PathBuf) -> Result<(), Box<dyn Error>> {
        println!("Sending printer the document to print: {:?}", save_doc_path);
        #[cfg(target_os = "windows")]
        if let Err(e) = print_document_windows(save_doc_path) {
            eprintln!("Error printing on Windows: {}", e);
            return Err(e);
        }

        #[cfg(target_os = "macos")]
        if let Err(e) = print_document_macos(save_doc_path) {
            eprintln!("Error printing on MacOS: {}", e);
            return Err(e);
        }

        Ok(())
    }
    
    

    
}
