use super::Engine;

use crate::defs::{Nums, Piece, Side};

/// Values in centipawns for each piece. Pawn, knight, bishop, rook, queen and
/// king.
const PIECE_VALUES: [i16; Nums::PIECES] = [100, 300, 330, 500, 900, 10_000];

impl Engine {
    /// Calculates a static evaluation of the current board depending on
    /// various heuristics.
    ///
    /// Currently just calculates material balance.
    // the single-letter difference ('w' vs 'b') clearly distinguishes pieces
    // of different sides
    #[allow(clippy::similar_names)]
    #[inline]
    #[must_use]
    pub fn evaluate_board(&self) -> i16 {
        let mut material = 0;
        let white_bb = self.board.side::<{ Side::WHITE.to_bool() }>();
        let black_bb = self.board.side::<{ Side::BLACK.to_bool() }>();

        for piece in 0..Nums::PIECES as u8 {
            let piece = Piece::from(piece);
            if piece == Piece::NONE {
                break;
            }
            let white = (self.board.piece_any(piece) & white_bb)
                .inner()
                .count_ones() as i16;
            let black = (self.board.piece_any(piece) & black_bb)
                .inner()
                .count_ones() as i16;
            material += (white - black) * PIECE_VALUES[piece.to_index()];
        }

        if self.board.side_to_move() == Side::WHITE {
            material
        } else {
            -material
        }
    }
}
