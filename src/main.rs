mod app;
mod db;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "GBilling RS",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
