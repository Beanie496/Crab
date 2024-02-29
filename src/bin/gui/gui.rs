use std::sync::{mpsc::Receiver, Arc, Mutex, MutexGuard};

use backend::{
    defs::{Piece, Square},
    engine::{Engine, SearchResult},
};
use clipboard::{ClipboardContext, ClipboardProvider};
use eframe::{
    egui::{load::Bytes, Color32, Context},
    CreationContext,
};
use egui_extras::install_image_loaders;

use update::FrameState;

/// For manipulating the internal state of the GUI.
mod board;
/// For drawing basic items.
mod draw;
/// Defines what updates each frame and draws it.
mod update;

/// Helper enum for `SquareColor` to show which square it is.
#[derive(Copy, Clone, PartialEq)]
// variants are self-explanatory
#[allow(clippy::missing_docs_in_private_items)]
enum SquareColorType {
    Light,
    Dark,
}

/// The GUI: used to save state between frames.
pub struct Gui {
    /// A redundant piece mailbox to separate it from the internal board.
    piece_mailbox: [Piece; Square::TOTAL],
    /// The internal engine, used for calculating legal moves and searching.
    engine: Arc<Mutex<Engine>>,
    /// See documentation for [`FrameState`].
    state: FrameState,
    /// The receiver used to obtain information about the search.
    info_rx: Option<Receiver<SearchResult>>,
}

/// The 4 colors that each square can take.
#[derive(Copy, Clone)]
struct SquareColor {
    /// The color it will use if the square is unselected.
    unselected: Color32,
    /// The color it will use if the square it selected.
    selected: Color32,
    /// Which type of square it is coloring: light or dark.
    square_type: SquareColorType,
}

/// Stores the square colors for selected and unselected light and dark
/// squares.
impl SquareColor {
    ///  The color of a dark square: a light brown.
    const DARK: Color32 = Color32::from_rgb(0xb8, 0x87, 0x62);
    /// The color of a light square: a very faint grey, almost white.
    const LIGHT: Color32 = Color32::from_rgb(0xee, 0xee, 0xee);
    /// The color of a selected dark square: a brown-ish yellow.
    const SELECTED_DARK: Color32 = Color32::from_rgb(0xd0, 0xc2, 0x38);
    /// The color of a selected light square: a light yellow.
    const SELECTED_LIGHT: Color32 = Color32::from_rgb(0xf2, 0xf2, 0x7f);
}

impl Gui {
    /// Creates a new [`Gui`] and initialises itself to a chessboard's starting
    /// position.
    pub fn new(cc: &CreationContext<'_>) -> Self {
        install_image_loaders(&cc.egui_ctx);
        include_piece_images(&cc.egui_ctx);

        let engine = Arc::new(Mutex::new(Engine::new()));
        #[allow(clippy::unwrap_used)]
        let piece_mailbox = engine.lock().unwrap().board.clone_mailbox();

        Self {
            piece_mailbox,
            engine,
            state: FrameState::default(),
            info_rx: None,
        }
    }

    /// Returns a [`MutexGuard`] to `self.engine`.
    fn engine(&self) -> MutexGuard<'_, Engine> {
        #[allow(clippy::unwrap_used)]
        self.engine.lock().unwrap()
    }

    /// Returns the selected square of `self`.
    const fn selected_square(&self) -> Option<Square> {
        self.state.selected_square
    }

    /// Sets the selected square of `self`.
    fn set_selected_square(&mut self, square: Option<Square>) {
        self.state.selected_square = square;
    }

    /// Checks if `self` has stopped running (i.e. the `Stop` button has been
    /// clicked).
    const fn has_stopped(&self) -> bool {
        self.state.has_stopped
    }

    /// Checks if we're in the process of importing a FEN.
    const fn is_importing_fen(&self) -> bool {
        self.state.is_importing_fen
    }

    /// Returns a mutable reference to the buffer used to store the FEN string
    /// entered by the user.
    fn entered_fen_string_mut(&mut self) -> &mut String {
        &mut self.state.entered_fen_string
    }

    /// Clears the string buffer for the entered FEN string.
    fn clear_entered_fen(&mut self) {
        self.state.entered_fen_string.clear();
    }

    /// Stop importing a FEN string (i.e.) show the buttons again and clear the
    /// string buffer.
    fn stop_importing_fen(&mut self) {
        self.state.is_importing_fen = false;
        self.clear_entered_fen();
    }

    /// Start importing a FEN string (i.e. hide the buttons and show a text
    /// field).
    fn start_importing_fen(&mut self) {
        self.state.is_importing_fen = true;
    }

    /// Sets the board position to the string that the user entered.
    fn set_position_to_entered_fen(&mut self) {
        self.engine()
            .set_position(&self.state.entered_fen_string, "");
        self.set_selected_square(None);
        self.state = FrameState::default();
        self.regenerate_mailboxes();
    }

    /// Copies the current FEN representation of the board to the system
    /// clipboard.
    fn copy_fen_to_clipboard(&self) {
        match ClipboardContext::new() {
            Ok(mut clipboard) => clipboard
                .set_contents(self.engine().board.current_fen_string())
                .unwrap_or_else(|error| println!("{error}")),
            Err(error) => println!("{error}"),
        };
    }

    /// Stops `self` from responding to input.
    fn stop(&mut self) {
        self.state.has_stopped = true;
    }

    /// Resets the state of `self`.
    ///
    /// Sets the board to the starting position and starts running if `self`
    /// had been stopped.
    fn restart(&mut self) {
        self.engine().board.set_startpos();
        self.set_selected_square(None);
        self.state = FrameState::default();
        self.regenerate_mailboxes();
    }
}

impl SquareColor {
    /// Creates a new `SquareColor`. If `square_type ==
    /// SquareColorType::Light`, it'll set the selected and unselected colors
    /// to a light color. Otherwise, it'll set them to a dark color.
    const fn new(square_type: SquareColorType) -> Self {
        match square_type {
            SquareColorType::Light => Self {
                selected: Self::SELECTED_LIGHT,
                unselected: Self::LIGHT,
                square_type: SquareColorType::Light,
            },
            SquareColorType::Dark => Self {
                selected: Self::SELECTED_DARK,
                unselected: Self::DARK,
                square_type: SquareColorType::Dark,
            },
        }
    }

    /// Flips the selected and unselected color of `self` depending on the
    /// value of `self.square_type`.
    ///
    /// If `self.square_type` is `Light`, it will set `self` to
    /// `Self::new(Dark)`. Otherwise, it'll set it to `Self::new(Light)`.
    fn flip_color(&mut self) {
        *self = if self.square_type == SquareColorType::Light {
            Self::new(SquareColorType::Dark)
        } else {
            Self::new(SquareColorType::Light)
        };
    }
}

/// Embeds all 12 piece images into the binary to allow easy (and efficient)
/// access later.
fn include_piece_images(ctx: &Context) {
    // can't make this any shorter (e.g. with a loop) because `include_bytes!`
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
