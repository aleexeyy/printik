#![cfg(target_os = "windows")]

pub use winprint::printer::{PdfiumPrinter, PrinterDevice, FilePrinter};
use std::path::PathBuf;

#[cfg(target_os = "windows")]
pub fn print_document_windows(save_doc_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {

    let path = save_doc_path.as_path();
    // 1. Enumerate all printers and pick the default
    let printers = PrinterDevice::all().expect("Failed to get printers");  
    let default_printer : PrinterDevice = printers
        .into_iter()
        .find(|dev| dev.name() == "EPSON ET-M1120 Series") 
        .expect("Failed to found printer with the given name");

    // 2. Wrap it in a PdfiumPrinter (requires the `pdfium` feature)
    let printer : PdfiumPrinter = PdfiumPrinter::new(default_printer);  

    // 3. Print the PDF file with default ticket/options
    printer.print(path, Default::default())? ;  
    Ok(())
}