//! A GUI used to play chess against.

use gui::Gui;

use eframe::{
    egui::{Vec2, ViewportBuilder},
    run_native, Error, NativeOptions,
};

/// Defines what happens each frame.
mod gui;
/// Utility.
mod util;

fn main() -> Result<(), Error> {
    let title = "Crab - A chess engine";

    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title(title)
            .with_decorations(false)
            .with_inner_size(Vec2::new(1920.0, 1080.0)),
        ..Default::default()
    };

    run_native(title, options, Box::new(|cc| Box::new(Gui::new(cc))))
}
