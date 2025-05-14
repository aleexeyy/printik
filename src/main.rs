mod ui;
mod watcher;
mod print_job;

mod bindings {
    #![allow(warnings)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

fn main() -> eframe::Result<()> {


    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([ui::INITIAL_WIDTH, ui::INITIAL_HEIGHT]),
        ..Default::default()
    };
    eframe::run_native(
        "photoQT",
        options,
        Box::new(|_cc| Box::new(ui::MyApp::new())),

    )

}
