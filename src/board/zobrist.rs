use super::{Board, CastlingRights};
use crate::{
    cfor,
    defs::{Piece, Side, Square},
};

#[allow(clippy::doc_markdown)]
/// Returns a pseudo-random number from `seed` using the Xorshift64 algorithm.
macro_rules! rand {
    ($seed:tt) => {{
        // ripped straight from wikipedia
        $seed ^= $seed << 13;
        $seed ^= $seed >> 7;
        $seed ^= $seed << 17;
        $seed
    }};
}

/// The size of a zobrist key.
pub type Key = u64;

/// A container for the zobrist keys. Initialised at program startup.
struct ZobristKeys {
    /// The keys for each of the pieces on each of the squares and the side to
    /// move, plus a 13th table for no piece.
    ///
    /// Since the first and last 8 indicies of the pawn table are never used,
    /// they can be reused. I'm using the A1 square of the Black pawn table
    /// for the side to move key.
    piece_and_side: [[Key; Piece::TOTAL + 1]; Square::TOTAL],
    /// The castling rights keys. One for each combination for fast lookup.
    castling_rights: [Key; 16],
    /// The en passant keys. 65 for fast lookup.
    ep: [Key; Square::TOTAL + 1],
}

/// The program's zobrist keys.
const ZOBRIST_KEYS: ZobristKeys = ZobristKeys::new();

impl Board {
    /// Makes a new, empty zobrist key.
    pub const fn new_zobrist() -> Key {
        0
    }

    /// Recalculates the zobrist key of the current board.
    pub fn refresh_zobrist(&mut self) {
        self.clear_zobrist();

        for square in 0..(Square::TOTAL as u8) {
            let square = Square(square);
            self.toggle_zobrist_piece(square, self.piece_on(square));
        }
        if self.side_to_move() == Side::BLACK {
            self.toggle_zobrist_side();
        }
        self.toggle_zobrist_castling_rights(self.castling_rights());
        self.toggle_zobrist_ep_square(self.ep_square());
    }

    /// Removes the zobrist key of `piece` on `start` and adds it to `end`.
    pub fn move_zobrist_piece(&mut self, start: Square, end: Square, piece: Piece) {
        self.toggle_zobrist_piece(start, piece);
        self.toggle_zobrist_piece(end, piece);
    }

    /// Toggles the zobrist key of the given piece on the given square.
    ///
    /// `piece` can be [`Piece::NONE`] but `square` has to be a valid square.
    pub fn toggle_zobrist_piece(&mut self, square: Square, piece: Piece) {
        self.zobrist ^= ZOBRIST_KEYS.piece_key(square, piece);
    }

    /// Toggles the side to move zobrist key.
    pub fn toggle_zobrist_side(&mut self) {
        self.zobrist ^= ZOBRIST_KEYS.side_key();
    }

    /// Toggles the zobrist keys of the given castling rights.
    pub fn toggle_zobrist_castling_rights(&mut self, rights: CastlingRights) {
        self.zobrist ^= ZOBRIST_KEYS.castling_rights_key(rights);
    }

    /// Toggles the zobrist keys of the given en passant square.
    pub fn toggle_zobrist_ep_square(&mut self, square: Square) {
        self.zobrist ^= ZOBRIST_KEYS.ep_square_key(square);
    }

    /// Zeroes the zobrist key.
    pub fn clear_zobrist(&mut self) {
        self.zobrist = 0;
    }

    /// Gets the zobrist key.
    pub const fn zobrist(&self) -> Key {
        self.zobrist
    }
}

impl ZobristKeys {
    /// Generates new pseudo-random zobrist keys.
    const fn new() -> Self {
        // arbitrary 8 bytes from /dev/random
        let mut seed = 0xc815_1848_573b_e077u64;
        let mut piece_and_side = [[0u64; Piece::TOTAL + 1]; Square::TOTAL];
        let mut castling_rights = [0u64; 16];
        let mut ep = [0u64; Square::TOTAL + 1];

        cfor!(let mut square = 0; square < Square::TOTAL; square += 1; {
            cfor!(let mut piece = 0; piece < Piece::TOTAL; piece += 1; {
                piece_and_side[square][piece] = rand!(seed);
            });
        });

        cfor!(let mut i = 0; i < castling_rights.len(); i += 1; {
            castling_rights[i] = rand!(seed);
        });

        cfor!(let mut square = Square::A3.0; square <= Square::H3.0; square += 1; {
            ep[square as usize] = rand!(seed);
        });
        cfor!(let mut square = Square::A6.0; square <= Square::H6.0; square += 1; {
            ep[square as usize] = rand!(seed);
        });

        Self {
            piece_and_side,
            castling_rights,
            ep,
        }
    }

    /// Calculates the key of the given piece on the given square.
    const fn piece_key(&self, square: Square, piece: Piece) -> Key {
        self.piece_and_side[square.to_index()][piece.to_index()]
    }

    /// Calculates the side to move key.
    const fn side_key(&self) -> Key {
        self.piece_and_side[Square::A1.to_index()][Piece::BPAWN.to_index()]
    }

    /// Calculates the key of the given castling rights.
    const fn castling_rights_key(&self, rights: CastlingRights) -> Key {
        self.castling_rights[rights.0 as usize]
    }

    /// Calculates the key of the given square.
    const fn ep_square_key(&self, square: Square) -> Key {
        self.ep[square.to_index()]
    }
}
