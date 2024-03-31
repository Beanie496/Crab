use super::{Board, CastlingRights, Key};
use crate::{
    cfor,
    defs::{Piece, Square},
    evaluation::{Score, PHASE_WEIGHTS, PIECE_SQUARE_TABLES},
    index_unchecked, out_of_bounds_is_unreachable,
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
static ZOBRIST_KEYS: ZobristKeys = ZobristKeys::new();

impl Board {
    /// Gets the phase of the game. 0 is midgame and 24 is endgame.
    ///
    /// Since this value is incrementally upadated, this function is zero-cost
    /// to call.
    pub const fn phase(&self) -> u8 {
        self.phase
    }

    /// Calculates the current material + piece-square table balance.
    ///
    /// Since this value is incrementally upadated, this function is zero-cost
    /// to call.
    pub const fn psq(&self) -> Score {
        self.psq
    }

    /// Gets the zobrist key.
    pub const fn zobrist(&self) -> Key {
        self.zobrist
    }

    /// Moves the accumulated `piece` from `start` to `end`.
    pub fn move_accumulated_piece(&mut self, start: Square, end: Square, piece: Piece) {
        self.move_psq_piece(start, end, piece);
        self.move_zobrist_piece(start, end, piece);
    }

    /// Adds `piece` on `square` to the accumulators.
    pub fn add_accumulated_piece(&mut self, square: Square, piece: Piece) {
        self.add_phase_piece(piece);
        self.add_psq_piece(square, piece);
        self.toggle_zobrist_piece(square, piece);
    }

    /// Removes `piece` on `square` from the accumulators.
    pub fn remove_accumulated_piece(&mut self, square: Square, piece: Piece) {
        self.remove_phase_piece(piece);
        self.remove_psq_piece(square, piece);
        self.toggle_zobrist_piece(square, piece);
    }

    /// Adds `piece` to `self.phase`.
    fn add_phase_piece(&mut self, piece: Piece) {
        self.phase += index_unchecked!(PHASE_WEIGHTS, piece.to_index());
    }

    /// Removes `piece` from `self.phase`.
    fn remove_phase_piece(&mut self, piece: Piece) {
        self.phase -= index_unchecked!(PHASE_WEIGHTS, piece.to_index());
    }

    /// Updates the piece-square table accumulator by adding the difference
    /// between the psqt value of the start and end square (which can be
    /// negative).
    fn move_psq_piece(&mut self, start: Square, end: Square, piece: Piece) {
        self.remove_psq_piece(start, piece);
        self.add_psq_piece(end, piece);
    }

    /// Adds the piece-square table value for `piece` at `square` to the psqt
    /// accumulator.
    fn add_psq_piece(&mut self, square: Square, piece: Piece) {
        out_of_bounds_is_unreachable!(piece.to_index(), PIECE_SQUARE_TABLES.len());
        out_of_bounds_is_unreachable!(square.to_index(), PIECE_SQUARE_TABLES[0].len());
        self.psq += PIECE_SQUARE_TABLES[piece.to_index()][square.to_index()];
    }

    /// Removes the piece-square table value for `piece` at `square` from the
    /// psqt accumulator.
    fn remove_psq_piece(&mut self, square: Square, piece: Piece) {
        out_of_bounds_is_unreachable!(piece.to_index(), PIECE_SQUARE_TABLES.len());
        out_of_bounds_is_unreachable!(square.to_index(), PIECE_SQUARE_TABLES[0].len());
        self.psq -= PIECE_SQUARE_TABLES[piece.to_index()][square.to_index()];
    }

    /// Removes the zobrist key of `piece` on `start` and adds it to `end`.
    fn move_zobrist_piece(&mut self, start: Square, end: Square, piece: Piece) {
        self.toggle_zobrist_piece(start, piece);
        self.toggle_zobrist_piece(end, piece);
    }

    /// Toggles the zobrist key of the given piece on the given square.
    ///
    /// `piece` can be [`Piece::NONE`] but `square` has to be a valid square.
    fn toggle_zobrist_piece(&mut self, square: Square, piece: Piece) {
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
    fn piece_key(&self, square: Square, piece: Piece) -> Key {
        out_of_bounds_is_unreachable!(square.to_index(), self.piece_and_side.len());
        out_of_bounds_is_unreachable!(piece.to_index(), self.piece_and_side[0].len());
        self.piece_and_side[square.to_index()][piece.to_index()]
    }

    /// Calculates the side to move key.
    const fn side_key(&self) -> Key {
        self.piece_and_side[Square::A1.to_index()][Piece::BPAWN.to_index()]
    }

    /// Calculates the key of the given castling rights.
    fn castling_rights_key(&self, rights: CastlingRights) -> Key {
        index_unchecked!(self.castling_rights, rights.0 as usize)
    }

    /// Calculates the key of the given square.
    fn ep_square_key(&self, square: Square) -> Key {
        index_unchecked!(self.ep, square.to_index())
    }
}
