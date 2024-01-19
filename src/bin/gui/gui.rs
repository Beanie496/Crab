use backend::{
    defs::{Nums, Piece, Side, Square},
    engine::Engine,
};
use eframe::{
    egui::{load::Bytes, Color32, Context},
    CreationContext,
};
use egui_extras::install_image_loaders;

/// For manipulating the internal state of the GUI.
mod board;
/// For drawing basic items.
mod draw;
/// Defines what updates each frame and draws it.
mod update;

/// Helper enum for `SquareColour` to show which square it is.
#[derive(Copy, Clone, PartialEq)]
enum SquareColourType {
    Light,
    Dark,
}

/// The GUI: used to save state between frames.
pub struct Gui {
    // redundant mailboxes to separate them from the internal board.
    piece_mailbox: [Piece; Nums::SQUARES],
    side_mailbox: [Side; Nums::SQUARES],
    engine: Engine,
    selected_square: Option<Square>,
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

/// Stores the square colours for selected and unselected light and dark
/// squares.
impl SquareColour {
    const DARK: Color32 = Color32::from_rgb(0xb8, 0x87, 0x62);
    const LIGHT: Color32 = Color32::from_rgb(0xee, 0xee, 0xee);
    const SELECTED_DARK: Color32 = Color32::from_rgb(0xd0, 0xc2, 0x38);
    const SELECTED_LIGHT: Color32 = Color32::from_rgb(0xf2, 0xf2, 0x7f);
}

impl Gui {
    /// Creates a new [`Gui`] and initialises itself to a chessboard's starting
    /// position.
    pub fn new(cc: &CreationContext<'_>) -> Self {
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

impl SquareColour {
    fn new(square_type: SquareColourType) -> Self {
        match square_type {
            SquareColourType::Light => Self {
                selected: Self::SELECTED_LIGHT,
                unselected: Self::LIGHT,
                square_type: SquareColourType::Light,
            },
            SquareColourType::Dark => Self {
                selected: Self::SELECTED_DARK,
                unselected: Self::DARK,
                square_type: SquareColourType::Dark,
            },
        }
    }
}

impl Gui {
    /// Returns the selected square of `self`.
    fn selected_square(&self) -> Option<Square> {
        self.selected_square
    }

    /// Sets the selected square of `self`.
    fn set_selected_square(&mut self, square: Option<Square>) {
        self.selected_square = square;
    }
}

impl SquareColour {
    fn flip_colour(&mut self) {
        *self = if self.square_type == SquareColourType::Light {
            Self::new(SquareColourType::Dark)
        } else {
            Self::new(SquareColourType::Light)
        };
    }
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
