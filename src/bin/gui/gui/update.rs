use super::{Gui, SquareColor, SquareColorType};
use crate::{gui::draw::paint_area_with_color, util::pixels_to_points};

use backend::defs::{File, Piece, Rank, Square};
use eframe::{
    egui::{
        self, widgets::Button, Align, Color32, Context, Direction, Id, Layout, Pos2, Rect, Sense,
        SidePanel, Ui, Vec2,
    },
    App,
};

/// Information about the current frame that the next frame needs to know.
#[derive(Default)]
pub struct FrameState {
    /// Whether or not the user is entering a FEN string.
    pub is_importing_fen: bool,
    /// The FEN string that the user is entering. Not defined if
    /// `is_importing_fen` is false.
    pub entered_fen_string: String,
    /// Set to `true` when `Stop` is clicked.
    pub has_stopped: bool,
    /// Which square is selected, if any.
    pub selected_square: Option<Square>,
}

impl App for Gui {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // board width with 40 px margin
        let board_area_width = 880.0;
        let info_box_width = 1920.0 - 880.0;
        // I like this color
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
            .exact_width(pixels_to_points(ctx, width))
            .frame(egui::Frame::none().fill(col))
            .show(ctx, |ui| {
                self.update_board(ctx, ui);
                self.update_buttons(ctx, ui);
                self.update_labels(ctx, ui);
            });
    }

    /// Draws the area where all the information from the engine is displayed.
    ///
    /// Currently just paints the whole thing the color `col`.
    // I'm using `self` in a few commits' time, hence the lint allow
    #[allow(clippy::unused_self)]
    fn update_info_area(&self, ctx: &Context, width: f32, col: Color32) {
        SidePanel::right(Id::new("info"))
            .resizable(false)
            .show_separator_line(false)
            .exact_width(pixels_to_points(ctx, width))
            .frame(egui::Frame::none().fill(col))
            .show(ctx, |_ui| {});
    }

    /// Draws the chessboard and all the pieces on it, handling clicks within
    /// the board as it does so.
    fn update_board(&mut self, ctx: &Context, ui: &mut Ui) {
        let mut color = SquareColor::new(SquareColorType::Dark);
        // draw the board, starting at the bottom left square; go left to right then
        // bottom to top
        for rank in 0..Rank::TOTAL {
            for file in 0..File::TOTAL {
                let square = Square::from_pos(Rank(rank as u8), File(file as u8));
                let square_corners = Rect {
                    // the board to be drawn is 800x800 pixels and sits at the
                    // bottom left with a margix of 40 pixels between it and
                    // the bottom and left. You can figure out the rest :)
                    min: Pos2::new(
                        pixels_to_points(ctx, 100.0f32.mul_add(file as f32, 40.0)),
                        // yeah idk why the available height is given in pixels to
                        // begin with
                        ui.available_height()
                            - pixels_to_points(ctx, 100.0f32.mul_add(rank as f32, 140.0)),
                    ),
                    max: Pos2::new(
                        pixels_to_points(ctx, 100.0f32.mul_add(file as f32, 140.0)),
                        ui.available_height()
                            - pixels_to_points(ctx, 100.0f32.mul_add(rank as f32, 40.0)),
                    ),
                };

                // create a new Ui for the square. This is done so that adding
                // the piece later adds it exactly where I want it.
                let mut child = ui.child_ui_with_id_source(
                    square_corners,
                    Layout::centered_and_justified(Direction::LeftToRight),
                    square.0,
                );

                self.update_square(&mut child, square_corners, square, color);

                color.flip_color();
            }
            // when going onto a new rank, flip the square again because it needs
            // stay the same color
            color.flip_color();
        }
    }

    /// Draws the buttons on the board [`SidePanel`] and handles clicks on
    /// them.
    ///
    /// Currently only draws them.
    // I'm using `self` in a few commits' time, hence the lint allow
    fn update_buttons(&mut self, ctx: &Context, ui: &mut Ui) {
        if self.is_importing_fen() {
            let mut child = ui.child_ui(
                // this is the region that the two bottom buttons cover
                Rect::from_min_max(
                    Pos2::new(pixels_to_points(ctx, 240.0), pixels_to_points(ctx, 130.0)),
                    Pos2::new(pixels_to_points(ctx, 640.0), pixels_to_points(ctx, 200.0)),
                ),
                Layout::left_to_right(Align::Center),
            );

            if child
                .text_edit_singleline(self.entered_fen_string_mut())
                .lost_focus()
            {
                self.set_position_to_entered_fen();
                self.stop_importing_fen();
            }
            return;
        }

        // I need child UI's to lay out the buttons exactly where I want them
        let mut child = ui.child_ui(
            Rect::from_min_max(
                // this is REALLY fucked. The width of the child UI is the width of
                // two buttons, plus the spacing between. Ok. The HEIGHT is the
                // height of ONE button so `Align::Center` causes the button to
                // fill the whole vertical space, then overflow the UI to form a
                // nice 2x2 grid. Why am I doing this? So the text is in the centre
                // of the buttons. Because aligning the text within the buttons is
                // not a feature for SOME GOD DAMN REASON.
                Pos2::new(pixels_to_points(ctx, 240.0), pixels_to_points(ctx, 40.0)),
                // buttons won't lay out correctly because of floating-point
                // imprecision so I have to do this
                // oh, and any value smaller than or equal to 0.00003 will
                // break. That includes `f32::EPSILON`.
                Pos2::new(
                    pixels_to_points(ctx, 640.0 + 0.00004),
                    pixels_to_points(ctx, 110.0),
                ),
            ),
            Layout::left_to_right(Align::Center).with_main_wrap(true),
        );

        child.spacing_mut().item_spacing =
            Vec2::new(pixels_to_points(ctx, 20.0), pixels_to_points(ctx, 20.0));

        let stop = Button::new("Stop").min_size(Vec2::new(
            pixels_to_points(ctx, 190.0),
            pixels_to_points(ctx, 70.0),
        ));
        let restart = Button::new("Restart").min_size(Vec2::new(
            pixels_to_points(ctx, 190.0),
            pixels_to_points(ctx, 70.0),
        ));
        let import_fen = Button::new("Import FEN").min_size(Vec2::new(
            pixels_to_points(ctx, 190.0),
            pixels_to_points(ctx, 70.0),
        ));
        let copy_fen = Button::new("Copy FEN").min_size(Vec2::new(
            pixels_to_points(ctx, 190.0),
            pixels_to_points(ctx, 70.0),
        ));

        if child.add(stop).clicked() {
            self.stop();
        }
        if child.add(restart).clicked() {
            self.restart();
        }
        if child.add(import_fen).clicked() {
            self.start_importing_fen();
        }
        if child.add(copy_fen).clicked() {
            self.copy_fen_to_clipboard();
        };
    }

    /// Draws the labels on the board [`SidePanel`].
    ///
    /// Not implemented yet.
    #[allow(clippy::unused_self)]
    fn update_labels(&self, _ctx: &Context, _ui: &mut Ui) {}

    /// Draws a square on `ui` in the given region, assuming the square number
    /// is `square`. If it is selected, it will draw the selected field of
    /// `color`; otherwise, it'll draw the unselected field.
    ///
    /// It will update the selected square of `self` if `ui` is clicked.
    fn update_square(&mut self, ui: &mut Ui, region: Rect, square: Square, color: SquareColor) {
        if self.has_stopped() {
            paint_area_with_color(ui, region, color.unselected);
            self.update_piece(ui, square);
            return;
        }

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
            paint_area_with_color(ui, region, color.selected);
        } else {
            paint_area_with_color(ui, region, color.unselected);
        }
        self.update_piece(ui, square);
    }

    /// Adds the image of the piece that is on the given square. Adds nothing
    /// if there is no piece on the given square.
    fn update_piece(&self, ui: &mut Ui, square: Square) {
        let image_path = match self.piece_on(square) {
            Piece::WPAWN => "pieces/wp.png",
            Piece::WKNIGHT => "pieces/wn.png",
            Piece::WBISHOP => "pieces/wb.png",
            Piece::WROOK => "pieces/wr.png",
            Piece::WQUEEN => "pieces/wq.png",
            Piece::WKING => "pieces/wk.png",
            Piece::BPAWN => "pieces/bp.png",
            Piece::BKNIGHT => "pieces/bn.png",
            Piece::BBISHOP => "pieces/bb.png",
            Piece::BROOK => "pieces/br.png",
            Piece::BQUEEN => "pieces/bq.png",
            Piece::BKING => "pieces/bk.png",
            // no piece
            _ => return,
        };
        ui.image(image_path);
    }
}
