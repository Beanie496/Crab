use crate::defs::{ Move, Piece, Side, Square };

/// Creates a [`Move`] given a start square, end square, piece and side.
pub fn create_move(start: Square, end: Square, piece: Piece, side: Side) -> Move {
    start as Move
        | ((end as Move) << 6)
        | ((piece as Move) << 12)
        | ((side as Move) << 15)
}

/// Turns a [`Move`] into its components: start square, end square, piece and
/// side, in that order.
pub fn decompose_move(mv: Move) -> (Square, Square, Piece, Side) {
    let start = mv & 0x3f;
    let end = (mv >> 6) & 0x3f;
    let piece = (mv >> 12) & 0x7;
    let side = (mv >> 15) & 0x1;
    (start as Square, end as Square, piece as Piece, side as Side)
}

#[cfg(test)]
mod tests {
    use crate::defs::{ Pieces, Sides, Squares };
    use super::create_move;

    #[test]
    fn create_move_works() {
        // these asserts will use magic values known to be correct
        assert_eq!(
            create_move(Squares::A1, Squares::H8, Pieces::KNIGHT, Sides::BLACK),
            (63 << 6) | (1 << 12) | (1 << 15),
        );
        assert_eq!(
            create_move(Squares::A8, Squares::H1, Pieces::KING, Sides::WHITE),
            56 | (7 << 6) | (5 << 12),
        );
    }
}
