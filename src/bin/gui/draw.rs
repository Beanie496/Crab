use crate::util::points_to_pixels;

use eframe::{
    egui::{
        include_image, widgets::Button, Align, Color32, Context, Direction, Frame, Id, Image,
        Layout, Pos2, Rect, Rounding, Shape, SidePanel, Stroke, Ui, Vec2,
    },
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
            let rect = Rect {
                // the board to be drawn is 800x800 pixels and sits at the bottom
                // left with a margix of 40 pixels. You can figure out the rest :)
                min: Pos2::new(
                    points_to_pixels(ctx, 40.0 + 100.0 * file as f32),
                    // yeah idk why the available height is given in pixels to
                    // begin with
                    ui.available_height() - points_to_pixels(ctx, 840.0 - 100.0 * rank as f32),
                ),
                max: Pos2::new(
                    points_to_pixels(ctx, 140.0 + 100.0 * file as f32),
                    ui.available_height() - points_to_pixels(ctx, 740.0 - 100.0 * rank as f32),
                ),
            };

            let mut child = ui.child_ui_with_id_source(
                rect,
                Layout::centered_and_justified(Direction::LeftToRight),
                rank * 8 + file,
            );

            draw_square(&mut child, &rect, &col);
            draw_piece(&mut child);

            flip_colour(&mut col, &Color32::WHITE, &Color32::from_rgb(0xb8, 0x87, 0x62));
        }
        // when going onto a new rank, flip the square again because it needs
        // stay the same colour
        flip_colour(&mut col, &Color32::WHITE, &Color32::from_rgb(0xb8, 0x87, 0x62));
    }
}

fn draw_buttons(ctx: &Context, ui: &mut Ui) {
    // I need child UI's to lay out the buttons exactly where I want them
    let mut child = ui.child_ui(
        Rect {
            // this is REALLY fucked. The width of the child UI is the width of
            // two buttons, plus the spacing between. Ok. The HEIGHT is the
            // height of ONE button so `Align::Center` causes the button to
            // fill the whole vertical space, then overflow the UI to form a
            // nice 2x2 grid. Why am I doing this? So the text is in the centre
            // of the buttons. Because aligning the text within the buttons is
            // not a feature for SOME GOD DAMN REASON.
            min: Pos2::new(points_to_pixels(ctx, 240.0), points_to_pixels(ctx, 40.0)),
            max: Pos2::new(points_to_pixels(ctx, 640.0), points_to_pixels(ctx, 110.0)),
        },
        Layout::left_to_right(Align::Center).with_main_wrap(true),
    );
    child.spacing_mut().item_spacing =
        // due to floating-point imprecision, 190.0 * 2 + 20.0 > 400.0 if any
        // of the numbers are divided
        Vec2::new(points_to_pixels(ctx, 19.9), points_to_pixels(ctx, 20.0));

    let stop = Button::new("Stop").min_size(Vec2::new(
        points_to_pixels(ctx, 190.0),
        points_to_pixels(ctx, 70.0),
    ));
    let restart = Button::new("Restart").min_size(Vec2::new(
        points_to_pixels(ctx, 190.0),
        points_to_pixels(ctx, 70.0),
    ));
    let import_fen = Button::new("Import FEN").min_size(Vec2::new(
        points_to_pixels(ctx, 190.0),
        points_to_pixels(ctx, 70.0),
    ));
    let copy_fen = Button::new("Copy FEN").min_size(Vec2::new(
        points_to_pixels(ctx, 190.0),
        points_to_pixels(ctx, 70.0),
    ));
    child.add(stop);
    child.add(restart);
    child.add(import_fen);
    child.add(copy_fen);
}

fn draw_labels(_ctx: &Context, _ui: &mut Ui) {}

fn draw_piece(child: &mut Ui) {
    // TODO: add an actual piece instead of just a placeholder
    child.add(Image::new(include_image!("pieces/wk.png")));
}

fn draw_square(child: &mut Ui, rect: &Rect, col: &Color32) {
    child.painter().add(Shape::Rect(RectShape::new(
        *rect,
        Rounding::ZERO,
        *col,
        Stroke::default(),
    )));
}

fn flip_colour(col: &mut Color32, col1: &Color32, col2: &Color32) {
    *col = if *col == *col1 {
        *col2
    } else {
        *col1
    };
}
