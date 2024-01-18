use backend::{
    defs::{Nums, Piece, Side, Square},
    engine::Engine,
};
use eframe::{
    egui::{load::Bytes, Color32, Context, Vec2, ViewportBuilder},
    run_native, App, CreationContext, Error, Frame, NativeOptions,
};
use egui_extras::install_image_loaders;

/// For manipulating the internal state of the GUI.
mod board;
/// For drawing-related items.
mod draw;
/// Utility.
mod util;

/// Helper enum for `SquareColour` to show which square it is.
#[derive(Copy, Clone, PartialEq)]
enum SquareColourType {
    Light,
    Dark,
}

/// The 4 colours that each square can take.
///
/// Yes I say 'colour' not 'color'. This isn't a library crate and I'm British.
#[derive(Copy, Clone)]
struct SquareColour {
    unselected: Color32,
    selected: Color32,
    square_type: SquareColourType,
}

/// Helper for `SQUARE_COLOURS` to give a name to both colours instead of just
/// an array.
struct SquareColourTypes {
    light: SquareColour,
    dark: SquareColour,
}

/// The GUI: used to save state between frames.
struct Gui {
    // redundant mailboxes to separate them from the internal board.
    piece_mailbox: [Piece; Nums::SQUARES],
    side_mailbox: [Side; Nums::SQUARES],
    // allowed dead code because I'll use it in a few commits' time
    #[allow(dead_code)]
    engine: Engine,
    selected_square: Option<Square>,
}

/// Stores the global square colours for selected and unselected light and dark
/// squares.
const SQUARE_COLOURS: SquareColourTypes = SquareColourTypes {
    // I took the method of changing the selected square colour according to
    // the way chess.com does it: increasing red and green by a certain amount
    // (increasing it more if the base colour is darker) and decreasing blue
    // by a certain amount (decreasing it more if the base colour is lighter)
    // It looks good to me so I'll use these colours.
    light: SquareColour {
        selected: Color32::from_rgb(0xf2, 0xf2, 0x7f),
        // 0xfff is too harsh so tone it down a little
        unselected: Color32::from_rgb(0xee, 0xee, 0xee),
        square_type: SquareColourType::Light,
    },
    dark: SquareColour {
        selected: Color32::from_rgb(0xd0, 0xc2, 0x38),
        // universally-recognised and looks nice
        unselected: Color32::from_rgb(0xb8, 0x87, 0x62),
        square_type: SquareColourType::Dark,
    },
};

impl App for Gui {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // board width with 40 px margin
        let board_area_width = 880.0;
        let info_box_width = 1920.0 - 880.0;
        // I like this colour
        let bg_col = Color32::from_rgb(0x2e, 0x2e, 0x2e);
        self.draw_board_area(ctx, board_area_width, bg_col);
        self.draw_info_area(ctx, info_box_width, Color32::RED);
    }
}

impl Gui {
    /// Returns the selected square of `self`.
    pub fn selected_square(&self) -> Option<Square> {
        self.selected_square
    }

    /// Sets the selected square of `self`.
    pub fn set_selected_square(&mut self, square: Option<Square>) {
        self.selected_square = square;
    }
}

impl Gui {
    /// Creates a new [`Gui`] and initialises itself to a chessboard's starting
    /// position.
    fn new(cc: &CreationContext<'_>) -> Self {
        install_image_loaders(&cc.egui_ctx);
        include_piece_images(&cc.egui_ctx);

        let engine = Engine::new();
        let mut side_mailbox = [Side::NONE; Nums::SQUARES];
        for (square, side) in side_mailbox.iter_mut().enumerate() {
            *side = engine.board.side_of(Square::from(square as u8));
        }

        Self {
            piece_mailbox: engine.board.clone_piece_board(),
            side_mailbox,
            engine,
            selected_square: None,
        }
    }
}

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

/// Embeds all 12 piece images into the binary to allow easy (and efficient)
/// access later.
fn include_piece_images(ctx: &Context) {
    // can't make this any shorter (e.g. with a loop) because `include_bytes`
    // requires a string literal
    ctx.include_bytes(
        "pieces/wp.png",
        Bytes::Static(include_bytes!("pieces/wp.png")),
    );
    ctx.include_bytes(
        "pieces/wn.png",
        Bytes::Static(include_bytes!("pieces/wn.png")),
    );
    ctx.include_bytes(
        "pieces/wb.png",
        Bytes::Static(include_bytes!("pieces/wb.png")),
    );
    ctx.include_bytes(
        "pieces/wr.png",
        Bytes::Static(include_bytes!("pieces/wr.png")),
    );
    ctx.include_bytes(
        "pieces/wq.png",
        Bytes::Static(include_bytes!("pieces/wq.png")),
    );
    ctx.include_bytes(
        "pieces/wk.png",
        Bytes::Static(include_bytes!("pieces/wk.png")),
    );
    ctx.include_bytes(
        "pieces/bp.png",
        Bytes::Static(include_bytes!("pieces/bp.png")),
    );
    ctx.include_bytes(
        "pieces/bn.png",
        Bytes::Static(include_bytes!("pieces/bn.png")),
    );
    ctx.include_bytes(
        "pieces/bb.png",
        Bytes::Static(include_bytes!("pieces/bb.png")),
    );
    ctx.include_bytes(
        "pieces/br.png",
        Bytes::Static(include_bytes!("pieces/br.png")),
    );
    ctx.include_bytes(
        "pieces/bq.png",
        Bytes::Static(include_bytes!("pieces/bq.png")),
    );
    ctx.include_bytes(
        "pieces/bk.png",
        Bytes::Static(include_bytes!("pieces/bk.png")),
    );
}
