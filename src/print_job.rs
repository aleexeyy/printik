
use lopdf::{Document, Object, Dictionary, Stream, Error as LopdfError};
use lopdf::content::{Content, Operation};
use image::{ImageReader, DynamicImage};

mod bindings {
    #![allow(warnings)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

use std::{
    error::Error,
    sync::mpsc::{self, Sender},
    thread,
    ptr,
    ffi::CString,
};


fn get_or_create_xobject(
    resources_dict: &mut Dictionary,
) -> Result<&mut Dictionary, LopdfError> {
    if !resources_dict.has(b"XObject") {
        resources_dict.set(b"XObject", Object::Dictionary(Dictionary::new()));
    }
    // get_mut returns Result<&mut Object, LopdfError>, then we downcast to Dictionary
    let xobj = resources_dict.get_mut(b"XObject")?;
    xobj.as_dict_mut()
}

pub enum InserterCommand {
    Insert { image_path: String, output_path: String },
    _Shutdown,
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
    pub fn new_and_spawn(template_path: String, x: f32, y: f32, width: f32, height: f32,) -> Sender<InserterCommand> {
            let (tx, rx) = mpsc::channel();
            let inserter = PdfImageInserter::new(template_path, x, y, width, height);

            // Spawn the single worker thread:
            thread::spawn(move || {
                for cmd in rx {
                    if let InserterCommand::Insert { image_path, output_path } = cmd {
                        if let Err(insert_err) = inserter.insert_image(&image_path, &output_path) {
                            eprintln!("Insert error: {}", insert_err);
                        } else {
                            if let Err(print_err) = inserter.print_document() {
                                eprintln!("Print error: {}", print_err);
                            }
                        }
                    } else {
                        break;
                    }
                }
            });
            tx
        }
    

    

    fn insert_image(&self, image_path: &str, _output_path: &str) -> Result<(), Box<dyn Error>> {

        let mut doc = Document::load(&self.template_path)?;

        let img = ImageReader::open(image_path)?
        .decode()?;
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
        content.operations.push(Operation::new("q", vec![])); // save state
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
            .push(Operation::new("Do", vec![Object::Name(b"Im1".to_vec())])); // draw
        content.operations.push(Operation::new("Q", vec![])); // restore state

        let content_stream = Stream::new(Dictionary::new(), content.encode()?);
        let content_id = doc.add_object(content_stream);

        {
            let page_dict = doc.get_dictionary_mut(page_id)?;
            let existing = page_dict.get(b"Contents")?; // &Object
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
        doc.save("/tmp/print_doc.pdf")?;
        Ok(())
    }

    fn print_document(&self) -> Result<(), Box<dyn Error>> {
        unsafe {
            // Get the list of destinations
            let mut dests: *mut bindings::cups_dest_t = ptr::null_mut();
            let num_dests = bindings::cupsGetDests(&mut dests);
            if num_dests == 0 {
                eprintln!("No printers found.");
                return Err("No printers found.".into());
            }
    
            // Get the default printer
            let dest = bindings::cupsGetDest(ptr::null(), ptr::null(), num_dests, dests);
            if dest.is_null() {
                eprintln!("Default printer not found.");
                bindings::cupsFreeDests(num_dests, dests);
                return Err("Default printer not found.".into());
            }
    
            // Prepare file path and job name
            let file_path = CString::new("/tmp/print_doc.pdf").unwrap();
            let job_name = CString::new("In-Memory PDF Print Job").unwrap();
    
            // Print the file
            let job_id = bindings::cupsPrintFile(
                (*dest).name,
                file_path.as_ptr(),
                job_name.as_ptr(),
                (*dest).num_options,
                (*dest).options,
            );
    
            if job_id == 0 {
                eprintln!("Failed to print the file.");
                return Err("Failed to print the file.".into());
            } else {
                println!("Print job submitted with ID: {}", job_id);
            }
    
            bindings::cupsFreeDests(num_dests, dests);
        }
    
        // Optionally, delete the temporary file after printing
        std::fs::remove_file("/tmp/print_doc.pdf").unwrap();

        Ok(())
    }
}
