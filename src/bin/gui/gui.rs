use crate::draw::{draw_board_area, draw_info_area};

use eframe::{
    egui::{Color32, Vec2, ViewportBuilder},
    run_simple_native, Error, NativeOptions,
};

mod draw;
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

    run_simple_native(title, options, move |ctx, _frame| {
        // board width with 40 px margin
        let board_area_width = 880.0;
        let info_box_width = 1920.0 - 880.0;
        // I like this colour
        let bg_col = Color32::from_rgb(0x2e, 0x2e, 0x2e);
        draw_board_area(ctx, board_area_width, bg_col);
        draw_info_area(ctx, info_box_width, Color32::RED);
    })
}
