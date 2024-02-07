use super::{Gui, SquareColor, SquareColorType};
use crate::{
    gui::draw::{add_button_to_region, paint_area_with_color},
    util::pixels_to_points,
};

use backend::defs::{File, Piece, Rank, Square};
use eframe::{
    egui::{
        self,
        containers::{Frame, Window},
        Align, CentralPanel, Color32, Context, Id, Layout, Pos2, Rect, Rounding, Sense, SidePanel,
        Stroke, Ui, Vec2,
    },
    App,
};
use egui_extras::{Column, TableBuilder};

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
        // I like this color
        let bg_col = Color32::from_rgb(0x2e, 0x2e, 0x2e);

        self.update_board_area(ctx, bg_col);
        self.update_info_area(ctx, bg_col);
    }
}

impl Gui {
    /// Updates the chessboard, buttons, and timers, and handles clicks in the
    /// board area.
    ///
    /// Timers are not implemented yet.
    fn update_board_area(&mut self, ctx: &Context, col: Color32) {
        SidePanel::left(Id::new("board"))
            .resizable(false)
            .show_separator_line(false)
            // board width with 40 px margin between it and the edge of the
            // screen
            .exact_width(pixels_to_points(ctx, 840.0))
            .frame(egui::Frame::none().fill(col))
            .show(ctx, |ui| {
                self.update_board(ctx, ui);
                self.update_buttons(ctx, ui);
                self.update_labels(ctx, ui);
            });
    }

    /// Draws the area where all the information from the engine is displayed.
    fn update_info_area(&self, ctx: &Context, col: Color32) {
        CentralPanel::default()
            .frame(egui::Frame::none().fill(col))
            .show(ctx, |ui| {
                self.update_info_box(ctx, ui);
            });
    }

    /// Draws the information box, which contains several fields to display
    /// engine output.
    ///
    /// Currently doesn't display engine output.
    fn update_info_box(&self, ctx: &Context, ui: &Ui) {
        let info_box_size = Vec2::new(
            // available width/height minus the 40 px margin
            ui.available_width() - pixels_to_points(ctx, 80.0),
            ui.available_height() - pixels_to_points(ctx, 80.0),
        );
        let top_left_point = Pos2::new(
            // edge of the board + margin
            pixels_to_points(ctx, 880.0),
            // top of the screen + margin
            pixels_to_points(ctx, 40.0),
        );
        let bottom_right_point = top_left_point + info_box_size;

        Window::new("Eval")
            .frame(
                Frame::none()
                    .stroke(Stroke::new(pixels_to_points(ctx, 5.0), Color32::DARK_GRAY))
                    .rounding(Rounding::same(pixels_to_points(ctx, 10.0))),
            )
            .movable(false)
            .fixed_rect(Rect::from_min_max(top_left_point, bottom_right_point))
            .collapsible(false)
            .scroll2(true)
            .show(ctx, |ui| {
                ui.expand_to_include_x(bottom_right_point.x);
                ui.expand_to_include_y(bottom_right_point.y);
                self.add_table(ctx, ui);
            });
    }

    /// Display the table of the info box.
    // TODO: make this actually display info (using `self`)
    #[allow(clippy::unused_self)]
    fn add_table(&self, ctx: &Context, ui: &mut Ui) {
        TableBuilder::new(ui)
            .columns(
                Column::auto()
                    .resizable(true)
                    .at_least(pixels_to_points(ctx, 100.0)),
                5,
            )
            .column(Column::remainder())
            .header(pixels_to_points(ctx, 35.0), |mut header| {
                header.col(|ui| {
                    // add a space so it isn't hugging the side
                    ui.heading(" Depth");
                });
                header.col(|ui| {
                    ui.heading("Time");
                });
                header.col(|ui| {
                    ui.heading("Nodes");
                });
                header.col(|ui| {
                    ui.heading("NPS");
                });
                header.col(|ui| {
                    ui.heading("Eval");
                });
                header.col(|ui| {
                    ui.heading("PV");
                });
            })
            .body(|mut body| {
                // this is all just example text
                body.row(0.0, |mut row| {
                    row.col(|ui| {
                        ui.label(" 100/100");
                    });
                    row.col(|ui| {
                        ui.label("983 ms");
                    });
                    row.col(|ui| {
                        ui.label("100,000,000,000");
                    });
                    row.col(|ui| {
                        ui.label("9,999,999k");
                    });
                    row.col(|ui| {
                        ui.label("+100.00");
                    });
                    row.col(|ui| {
                        ui.label("Nf3 d5 d4 Nf6 c4 e6 Nc3 c6 Bg5 h6");
                    });
                });
            });
    }

    /// Draws the chessboard and all the pieces on it, handling clicks within
    /// the board as it does so.
    fn update_board(&mut self, ctx: &Context, ui: &mut Ui) {
        let mut color = SquareColor::new(SquareColorType::Dark);
        let available_height = ui.available_height();

        let mut square_corners = Rect::from_min_max(
            // the board to be drawn is 800x800 pixels and sits at the
            // bottom left with a margix of 40 pixels between it and
            // the bottom and left. You can figure out the rest :)
            Pos2::new(
                pixels_to_points(ctx, 40.0),
                available_height - pixels_to_points(ctx, 140.0),
            ),
            Pos2::new(
                pixels_to_points(ctx, 140.0),
                available_height - pixels_to_points(ctx, 40.0),
            ),
        );
        // draw the board, starting at the bottom left square; go left to right then
        // bottom to top
        for rank in 0..Rank::TOTAL {
            for file in 0..File::TOTAL {
                let square = Square::from_pos(Rank(rank as u8), File(file as u8));

                self.update_square(ui, square_corners, square, color);
                self.add_piece(ui, square_corners, square);

                square_corners = square_corners.translate(Vec2::new(
                    pixels_to_points(ctx, 100.0),
                    pixels_to_points(ctx, 0.0),
                ));
                color.flip_color();
            }
            square_corners = square_corners.translate(Vec2::new(
                pixels_to_points(ctx, -800.0),
                pixels_to_points(ctx, -100.0),
            ));
            // when going onto a new rank, flip the square again because it needs
            // stay the same color
            color.flip_color();
        }
    }

    /// Draws the buttons on the board [`SidePanel`] and handles clicks on
    /// them.
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

        // 4 buttons, each 190x70 with 20px spacing, arranged in a 2x2 grid
        // with 20px spacing
        add_button_to_region(
            ui,
            Rect::from_min_max(
                Pos2::new(pixels_to_points(ctx, 240.0), pixels_to_points(ctx, 40.0)),
                Pos2::new(pixels_to_points(ctx, 430.0), pixels_to_points(ctx, 110.0)),
            ),
            "Stop",
            || self.stop(),
        );
        add_button_to_region(
            ui,
            Rect::from_min_max(
                Pos2::new(pixels_to_points(ctx, 450.0), pixels_to_points(ctx, 40.0)),
                Pos2::new(pixels_to_points(ctx, 640.0), pixels_to_points(ctx, 110.0)),
            ),
            "Restart",
            || self.restart(),
        );
        add_button_to_region(
            ui,
            Rect::from_min_max(
                Pos2::new(pixels_to_points(ctx, 240.0), pixels_to_points(ctx, 130.0)),
                Pos2::new(pixels_to_points(ctx, 430.0), pixels_to_points(ctx, 200.0)),
            ),
            "Import FEN",
            || self.start_importing_fen(),
        );
        add_button_to_region(
            ui,
            Rect::from_min_max(
                Pos2::new(pixels_to_points(ctx, 450.0), pixels_to_points(ctx, 130.0)),
                Pos2::new(pixels_to_points(ctx, 640.0), pixels_to_points(ctx, 200.0)),
            ),
            "Copy FEN",
            || self.copy_fen_to_clipboard(),
        );
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
            return;
        }

        if ui.allocate_rect(region, Sense::click()).clicked() {
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
    }

    /// Adds the image of the piece on `square` to `ui`. Adds nothing if there
    /// is no piece on the given square.
    pub fn add_piece(&self, ui: &mut Ui, region: Rect, square: Square) {
        ui.allocate_ui_at_rect(region, |ui| {
            ui.image(match self.piece_on(square) {
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
            });
        });
    }
}
