use eframe::{
    egui::{
        self, Color32, Context, Frame, Id, Pos2, Rect, Rounding, Shape, Stroke, Ui, Vec2,
        ViewportBuilder,
    },
    epaint::RectShape,
    run_simple_native, Error, NativeOptions,
};

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
        let board_area_width = 880.0;
        let info_box_width = 1920.0 - 880.0;
        let bg_col = Color32::from_rgb(0x2e, 0x2e, 0x2e);
        draw_board_area(ctx, board_area_width, bg_col);
        draw_info_area(ctx, info_box_width, Color32::RED);
    })
}

fn draw_board_area(ctx: &Context, width: f32, col: Color32) {
    egui::SidePanel::left(Id::new("board"))
        .resizable(false)
        .show_separator_line(false)
        .exact_width(points_to_pixels(ctx, width))
        .frame(Frame::none().fill(col))
        .show(ctx, |ui| {
            draw_board(ctx, ui);
        });
}

fn draw_info_area(ctx: &Context, width: f32, col: Color32) {
    egui::SidePanel::right(Id::new("info"))
        .resizable(false)
        .show_separator_line(false)
        .exact_width(points_to_pixels(ctx, width))
        .frame(Frame::none().fill(col))
        .show(ctx, |_ui| {});
}

fn points_to_pixels(ctx: &Context, points: f32) -> f32 {
    points / ctx.native_pixels_per_point().unwrap()
}

fn draw_board(ctx: &Context, ui: &mut Ui) {
    let mut col = Color32::WHITE;
    // draw the board, starting at the top left square; go left to right then
    // top to bottom
    for rank in 0..8 {
        for file in 0..8 {
            // the board to be drawn is 800x800 pixels and sits at the bottom
            // left with a margix of 40 pixels. You can figure out the rest :)
            let top_left = Pos2::new(
                points_to_pixels(ctx, 40.0 + 100.0 * file as f32),
                // yeah idk why the available height is given in pixels to
                // begin with
                ui.available_height() - points_to_pixels(ctx, 840.0 - 100.0 * rank as f32),
            );
            let bottom_right = Pos2::new(
                points_to_pixels(ctx, 140.0 + 100.0 * file as f32),
                ui.available_height() - points_to_pixels(ctx, 740.0 - 100.0 * rank as f32),
            );
            let rect = Rect {
                min: top_left,
                max: bottom_right,
            };
            ui.painter().add(Shape::Rect(RectShape::new(
                rect,
                Rounding::ZERO,
                col,
                Stroke::default(),
            )));
            // flip the square colour
            col = if col == Color32::WHITE {
                Color32::from_rgb(0xb8, 0x87, 0x62)
            } else {
                Color32::WHITE
            };
        }
        // when going onto a new rank, flip the square again because it needs
        // stay the same colour
        col = if col == Color32::WHITE {
            Color32::from_rgb(0xb8, 0x87, 0x62)
        } else {
            Color32::WHITE
        };
    }
}
