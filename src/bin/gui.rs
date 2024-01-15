use eframe::{egui, run_simple_native, Error, NativeOptions};

fn main() -> Result<(), Error> {
    let title = "Crab - A chess engine";

    let options = NativeOptions::default();
    run_simple_native(title, options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |_ui| {});
    })
}
