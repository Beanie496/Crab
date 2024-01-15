use eframe::{
    egui::{
        self, Color32, Context, Frame, Id, Pos2, Rect, Rounding, Shape, Stroke, Vec2,
        ViewportBuilder, Ui
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
            draw_board(ctx, ui, width);
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

fn draw_board(ctx: &Context, ui: &mut Ui, width: f32) {
    let top_left = Pos2::new(
        points_to_pixels(ctx, 40.0),
        ui.available_height() - points_to_pixels(ctx, 840.0),
    );
    let bottom_right = Pos2::new(
        points_to_pixels(ctx, width - 40.0),
        ui.available_height() - points_to_pixels(ctx, 40.0),
    );
    let rect = Rect {
        min: top_left,
        max: bottom_right,
    };
    ui.set_clip_rect(rect);
    ui.painter().add(Shape::Rect(RectShape::new(
        Rect::EVERYTHING,
        Rounding::ZERO,
        Color32::WHITE,
        Stroke::default(),
    )));
}
