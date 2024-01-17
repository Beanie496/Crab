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

/// The GUI: used to save state between frames.
struct Gui {
    // redundant mailboxes to separate them from the internal board.
    piece_mailbox: [Piece; Nums::SQUARES],
    side_mailbox: [Side; Nums::SQUARES],
    engine: Engine,
}

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
