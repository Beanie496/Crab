use crate::uci::Uci;

/// Functions to perform bit-related operations.
mod bits;
/// Items related to the board. Mainly [`Board`](board::Board).
mod board;
/// Definitions and enumerations.
mod defs;
/// A container for [`Engine`].
mod engine;
/// Items related to move generation.
mod movegen;
/// A container for [`Movelist`](movelist::Movelist).
mod movelist;
/// Handles UCI input
mod uci;
/// A collection of assorted useful functions.
mod util;

fn main() {
    Uci::main_loop();
}
