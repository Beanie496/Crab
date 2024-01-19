use super::{Gui, SquareColour, SquareColourType};
use crate::{gui::draw::paint_area_with_colour, util::points_to_pixels};

use backend::defs::{Nums, Piece, Side, Square};
use eframe::{
    egui::{
        self, widgets::Button, Align, Color32, Context, Direction, Id, Layout, Pos2, Rect, Sense,
        SidePanel, Ui, Vec2,
    },
    App,
};

impl App for Gui {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // board width with 40 px margin
        let board_area_width = 880.0;
        let info_box_width = 1920.0 - 880.0;
        // I like this colour
        let bg_col = Color32::from_rgb(0x2e, 0x2e, 0x2e);
        self.update_board_area(ctx, board_area_width, bg_col);
        self.update_info_area(ctx, info_box_width, Color32::RED);
    }
}

impl Gui {
    /// Creates a [`SidePanel`] and draws the chessboard, buttons and timers.
    /// It also handles clicks in the board area.
    ///
    /// The reason why this isn't two functions is because the `Ui`s that make
    /// the squares (and detect the clicks) are created to draw the square and
    /// then immediately destroyed, so they need to handle clicks there and
    /// then.
    ///
    /// Buttons currently do nothing and timers are not implemented yet.
    fn update_board_area(&mut self, ctx: &Context, width: f32, col: Color32) {
        SidePanel::left(Id::new("board"))
            .resizable(false)
            .show_separator_line(false)
            .exact_width(points_to_pixels(ctx, width))
            .frame(egui::Frame::none().fill(col))
            .show(ctx, |ui| {
                self.update_board(ctx, ui);
                self.update_buttons(ctx, ui);
                self.update_labels(ctx, ui);
            });
    }

    /// Draws the area where all the information from the engine is displayed.
    ///
    /// Currently just paints the whole thing the colour `col`.
    fn update_info_area(&self, ctx: &Context, width: f32, col: Color32) {
        SidePanel::right(Id::new("info"))
            .resizable(false)
            .show_separator_line(false)
            .exact_width(points_to_pixels(ctx, width))
            .frame(egui::Frame::none().fill(col))
            .show(ctx, |_ui| {});
    }

    /// Draws the chessboard and all the pieces on it, handling clicks within
    /// the board as it does so.
    fn update_board(&mut self, ctx: &Context, ui: &mut Ui) {
        let mut colour = SquareColour::new(SquareColourType::Dark);
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

                self.update_square(&mut child, square_corners, square, colour);

                colour.flip_colour();
            }
            // when going onto a new rank, flip the square again because it needs
            // stay the same colour
            colour.flip_colour();
        }
    }

    /// Draws the buttons on the board [`SidePanel`] and handles clicks on
    /// them.
    ///
    /// Currently only draws them.
    fn update_buttons(&self, ctx: &Context, ui: &mut Ui) {
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
                // buttons won't lay out correctly because of floating-point
                // imprecision so I have to do this
                max: Pos2::new(points_to_pixels(ctx, 640.1), points_to_pixels(ctx, 110.0)),
            },
            Layout::left_to_right(Align::Center).with_main_wrap(true),
        );
        child.spacing_mut().item_spacing =
            Vec2::new(points_to_pixels(ctx, 20.0), points_to_pixels(ctx, 20.0));

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
    fn update_labels(&self, _ctx: &Context, _ui: &mut Ui) {}

    /// Draws a square on `ui` in the given region, assuming the square number
    /// is `square`. If it is selected, it will draw the selected field of
    /// `colour`; otherwise, it'll draw the unselected field.
    ///
    /// It will update the selected square of `self` if `ui` is clicked.
    fn update_square(&mut self, ui: &mut Ui, region: Rect, square: Square, colour: SquareColour) {
        if ui.interact(region, ui.id(), Sense::click()).clicked() {
            if let Some(selected) = self.selected_square() {
                self.set_selected_square(None);
                let start = selected;
                let end = square;
                if selected == square {
                    return;
                }

                self.move_piece(start, end);
            } else {
                self.set_selected_square(Some(square));
            }
        }
        if self.selected_square() == Some(square) {
            paint_area_with_colour(ui, region, colour.selected);
        } else {
            paint_area_with_colour(ui, region, colour.unselected);
        }
        self.update_piece(ui, square);
    }

    /// Adds the image of the piece that is on the given square. Adds nothing
    /// if there is no piece on the given square.
    fn update_piece(&self, ui: &mut Ui, square: Square) {
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
}