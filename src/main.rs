// mod ui;
// mod watcher;
// mod printer_control;
// fn main() -> eframe::Result<()> {


//     printer_control::control_printer();

//     let options = eframe::NativeOptions {
//         viewport: egui::ViewportBuilder::default()
//             .with_inner_size([ui::INITIAL_WIDTH, ui::INITIAL_HEIGHT]),
//         ..Default::default()
//     };
//     eframe::run_native(
//         "photoQT",
//         options,
//         Box::new(|_cc| Box::new(ui::MyApp::new())),

//     )

// }

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::mem;

fn main() {
    unsafe {
        let mut dests: *mut cups_dest_t = mem::zeroed();
        let num_dests = cupsGetDests(&mut dests as *mut _);
        println!("{:?}", std::slice::from_raw_parts(dests, num_dests as usize));
        cupsFreeDests(num_dests, dests);
    }
}
