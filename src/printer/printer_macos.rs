#![cfg(target_os = "macos")]
mod bindings {
    #![allow(warnings)]
    include!(concat!(env!("OUT_DIR"), "/bindings_cups.rs"));
}
use std::path::PathBuf;

#[cfg(target_os = "macos")]
pub fn print_document_macos(save_doc_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let temp_path = save_doc_path.as_path();
        unsafe {
            let mut dests: *mut bindings::cups_dest_t = std::ptr::null_mut();
            let num_dests = bindings::cupsGetDests(&mut dests);
            if num_dests == 0 {
                eprintln!("No printers found.");
                return Err("No printers found.".into());
            }
    
            let dest = bindings::cupsGetDest(std::ptr::null(), std::ptr::null(), num_dests, dests);
            if dest.is_null() {
                eprintln!("Default printer not found.");
                bindings::cupsFreeDests(num_dests, dests);
                return Err("Default printer not found.".into());
            }
            let file_path = std::ffi::CString::new(temp_path.to_str().unwrap()).unwrap();
            let job_name = std::ffi::CString::new("In-Memory PDF Print Job").unwrap();
    
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
    
        

        Ok(())
    }
