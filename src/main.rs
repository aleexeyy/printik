mod ui;
mod watcher;
mod printer;
mod pdfwrap;
mod printer_wrapper;
fn main() {
    if cfg!(target_os = "macos") {
        println!("Running on MacOS");
    } else {
        println!("Running on Windows");
    }


    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([ui::INITIAL_WIDTH, ui::INITIAL_HEIGHT])
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "photoQT",
        options,
        Box::new(|_cc| {
            Ok(Box::new(ui::MyApp::new()))
        }),
    ).unwrap();

}
