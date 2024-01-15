use crate::util::points_to_pixels;

use eframe::{
    egui::{Color32, Context, Frame, Id, Pos2, Rect, Rounding, Shape, SidePanel, Stroke, Ui},
    epaint::RectShape,
};

pub fn draw_board_area(ctx: &Context, width: f32, col: Color32) {
    SidePanel::left(Id::new("board"))
        .resizable(false)
        .show_separator_line(false)
        .exact_width(points_to_pixels(ctx, width))
        .frame(Frame::none().fill(col))
        .show(ctx, |ui| {
            draw_board(ctx, ui);
            draw_pieces(ctx, ui);
            draw_buttons(ctx, ui);
            draw_labels(ctx, ui);
        });
}

pub fn draw_info_area(ctx: &Context, width: f32, col: Color32) {
    SidePanel::right(Id::new("info"))
        .resizable(false)
        .show_separator_line(false)
        .exact_width(points_to_pixels(ctx, width))
        .frame(Frame::none().fill(col))
        .show(ctx, |_ui| {});
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

fn draw_pieces(_ctx: &Context, _ui: &mut Ui) {}

fn draw_buttons(_ctx: &Context, _ui: &mut Ui) {}

fn draw_labels(_ctx: &Context, _ui: &mut Ui) {}
