use std::path::Path;
#[cfg(target_os = "windows")]
pub use winprint::printer::{PdfiumPrinter, PrinterDevice, FilePrinter};

#[cfg(target_os = "macos")]
mod bindings {
    #![allow(warnings)]
    include!(concat!(env!("OUT_DIR"), "/bindings_cups.rs"));
}
#[cfg(target_os = "macos")]
use std::ffi::CString;


pub trait Printer: Send  + 'static   {
    fn new(printer_name: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> where Self : Sized;

    fn print(&self, file_to_print: &Path) -> Result<(), Box<dyn std::error::Error>>;
}

#[cfg(target_os = "windows")]
pub struct WindowsPrinter {
    printer: PdfiumPrinter,
}
#[cfg(target_os = "windows")]
impl Printer for WindowsPrinter {
    fn new(printer_name: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let devices = PrinterDevice::all()?; 
        let dev = printer_name
            .and_then(|name| devices.iter().find(|d| d.name() == name).cloned())
            .or_else(|| devices.into_iter().next())
            .ok_or("No printers available")?;

        Ok(WindowsPrinter { printer: PdfiumPrinter::new(dev) })
    }

    fn print(&self, file_to_print: &Path) -> Result<(), Box<dyn std::error::Error>> {
        self.printer
            .print(file_to_print, Default::default())
            .map_err(|e| e.into())
    }
}



#[cfg(target_os = "macos")]
pub struct MacosPrinter {
    dest: *mut bindings::cups_dest_t,
    job_name: CString,
}


#[cfg(target_os = "macos")]
unsafe impl Send for MacosPrinter {}


#[cfg(target_os = "macos")]
impl Printer for MacosPrinter {
    fn new(printer_name: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        if let Some(name) = printer_name {
            println!("Given printer name: {:?}", name);
        }
        
        unsafe {

            let dest = if let Some(name) = printer_name {
                let c_name = CString::new(name)?;
                let named = bindings::cupsGetNamedDest(
                    std::ptr::null_mut(),
                    c_name.as_ptr(),
                    std::ptr::null(),
                );

                if !named.is_null() {
                    named
                } else {
                    // Fallback to default
                    bindings::cupsGetNamedDest(
                        std::ptr::null_mut(),
                        std::ptr::null(),
                        std::ptr::null(),
                    )
                }
            } else {
                bindings::cupsGetNamedDest(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                )
            };

            if dest.is_null() {
                return Err("Requested printer not found".into());
            }

            Ok(MacosPrinter {   
                dest,
                job_name: CString::new("Cross-Platform Print Job")?,
            })
        }
    }

    fn print(&self, file_to_print: &Path) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let file_path = CString::new(file_to_print.to_string_lossy().as_ref())?;
            let job_id = bindings::cupsPrintFile(
                (*self.dest).name,
                file_path.as_ptr(),
                self.job_name.as_ptr(),
                (*self.dest).num_options,
                (*self.dest).options,
            );
            if job_id == 0 {
                return Err("Failed to print file".into());
            } else {
                println!("Print job submitted with ID: {}", job_id);
            }
        }
        Ok(())
    }
}
#[cfg(target_os = "macos")]
impl Drop for MacosPrinter {
    fn drop(&mut self) {
        unsafe {
            bindings::cupsFreeDests(1, self.dest);
        }
    }
}


pub fn make_printer(printer_name: Option<&str>) -> Result<Box<dyn Printer>, Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    { Ok(Box::new(MacosPrinter::new(printer_name)?)) }

    #[cfg(target_os = "windows")]
    { Ok(Box::new(WindowsPrinter::new(printer_name)?)) }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    { Err("Unsupported platform".into()) }
}
