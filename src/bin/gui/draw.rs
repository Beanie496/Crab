use super::{Gui, SquareColour, SQUARE_COLOURS};
use crate::util::{flip_square_colour, points_to_pixels};

use backend::defs::{Nums, Piece, Side, Square};
use eframe::{
    egui::{
        widgets::Button, Align, Color32, Context, Direction, Frame, Id, Layout, Pos2, Rect,
        Rounding, Sense, Shape, SidePanel, Stroke, Ui, Vec2,
    },
    epaint::RectShape,
};

impl Gui {
    /// Creates a [`SidePanel`] and draws the chessboard, buttons and timers.
    ///
    /// Buttons currently do nothing and timers are not implemented yet.
    pub fn draw_board_area(&mut self, ctx: &Context, width: f32, col: Color32) {
        SidePanel::left(Id::new("board"))
            .resizable(false)
            .show_separator_line(false)
            .exact_width(points_to_pixels(ctx, width))
            .frame(Frame::none().fill(col))
            .show(ctx, |ui| {
                self.draw_board(ctx, ui);
                self.draw_buttons(ctx, ui);
                self.draw_labels(ctx, ui);
            });
    }

    /// Draws the area where all the information from the engine is displayed.
    ///
    /// Currently just paints the whole thing red.
    pub fn draw_info_area(&self, ctx: &Context, width: f32, col: Color32) {
        SidePanel::right(Id::new("info"))
            .resizable(false)
            .show_separator_line(false)
            .exact_width(points_to_pixels(ctx, width))
            .frame(Frame::none().fill(col))
            .show(ctx, |_ui| {});
    }
}

impl Gui {
    /// Draws the chessboard and all the pieces on it.
    fn draw_board(&mut self, ctx: &Context, ui: &mut Ui) {
        let mut colour = SQUARE_COLOURS.dark;
        // draw the board, starting at the bottom left square; go left to right then
        // bottom to top
        for rank in 0..Nums::RANKS {
            for file in 0..Nums::FILES {
                let square = Square::from(rank as u8 * 8 + file as u8);
                let square_corners = Rect {
                    // the board to be drawn is 800x800 pixels and sits at the
                    // bottom left with a margix of 40 pixels between it and
                    // the bottom and left. You can figure out the rest :)
                    min: Pos2::new(
                        points_to_pixels(ctx, 40.0 + 100.0 * file as f32),
                        // yeah idk why the available height is given in pixels to
                        // begin with
                        ui.available_height() - points_to_pixels(ctx, 140.0 + 100.0 * rank as f32),
                    ),
                    max: Pos2::new(
                        points_to_pixels(ctx, 140.0 + 100.0 * file as f32),
                        ui.available_height() - points_to_pixels(ctx, 40.0 + 100.0 * rank as f32),
                    ),
                };

                // create a new Ui for the square. This is done so that adding
                // the piece later adds it exactly where I want it.
                let mut child = ui.child_ui_with_id_source(
                    square_corners,
                    Layout::centered_and_justified(Direction::LeftToRight),
                    square.inner(),
                );

                // colour the square
                self.draw_square(&mut child, square_corners, square, colour);
                // add the piece
                self.draw_piece(&mut child, square);

                flip_square_colour(&mut colour);
            }
            // when going onto a new rank, flip the square again because it needs
            // stay the same colour
            flip_square_colour(&mut colour);
        }
    }

    /// Draws the buttons on the board [`SidePanel`].
    fn draw_buttons(&self, ctx: &Context, ui: &mut Ui) {
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

    /// Draws the labels on the board [`SidePanel`].
    ///
    /// Not implemented yet.
    fn draw_labels(&self, _ctx: &Context, _ui: &mut Ui) {}

    /// Adds the image of the piece that is on the given square. Adds nothing
    /// if there is no piece on the given square.
    fn draw_piece(&self, ui: &mut Ui, square: Square) {
        if self.piece_on(square) == Piece::NONE {
            return;
        }
        let image_path = match self.side_of(square) {
            Side::WHITE => match self.piece_on(square) {
                Piece::PAWN => "pieces/wp.png",
                Piece::KNIGHT => "pieces/wn.png",
                Piece::BISHOP => "pieces/wb.png",
                Piece::ROOK => "pieces/wr.png",
                Piece::QUEEN => "pieces/wq.png",
                Piece::KING => "pieces/wk.png",
                _ => unreachable!("There must be a piece on the square"),
            },
            Side::BLACK => match self.piece_on(square) {
                Piece::PAWN => "pieces/bp.png",
                Piece::KNIGHT => "pieces/bn.png",
                Piece::BISHOP => "pieces/bb.png",
                Piece::ROOK => "pieces/br.png",
                Piece::QUEEN => "pieces/bq.png",
                Piece::KING => "pieces/bk.png",
                _ => unreachable!("There must be a piece on the square"),
            },
            _ => unreachable!("If there is a piece, it must be White or Black"),
        };
        ui.image(image_path);
    }

    /// Draw a square on `ui` in area `rect` with the given square `square` and
    /// the the given square colour `colour`.
    ///
    /// If it's clicked, it will draw the square as selected and update the
    /// selected square of `self`.
    // TODO: I don't like how a function called 'draw_xyz' mutates some
    // unrelated thing. Fix when adding piece movement.
    fn draw_square(&mut self, ui: &mut Ui, rect: Rect, square: Square, colour: SquareColour) {
        if ui.interact(rect, ui.id(), Sense::click()).clicked() {
            if self.selected_square().is_none() {
                self.set_selected_square(Some(square));
            } else {
                self.set_selected_square(None);
            }
        }
        if self.selected_square() == Some(square) {
            self.paint_area_with_colour(ui, rect, colour.selected);
        } else {
            self.paint_area_with_colour(ui, rect, colour.unselected);
        }
    }

    /// Paints the area on `ui` defined by `rect` the colour `colour`.
    fn paint_area_with_colour(&self, ui: &mut Ui, rect: Rect, colour: Color32) {
        ui.painter().add(Shape::Rect(RectShape::new(
            rect,
            Rounding::ZERO,
            colour,
            Stroke::default(),
        )));
    }
}
