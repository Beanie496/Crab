use crate::defs::{Move, Piece, Side, Square};

/// Creates a [`Move`] given a start square, end square, piece and side.
pub fn create_move<const IS_WHITE: bool, const PIECE: Piece>(start: Square, end: Square) -> Move {
    start as Move | (end as Move) << 6 | (PIECE as Move) << 12 | (IS_WHITE as Move) << 15
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
    use super::{create_move, decompose_move};
    use crate::defs::{Pieces, Sides, Squares};

    #[test]
    fn create_move_works() {
        // these asserts will use magic values known to be correct
        assert_eq!(
            create_move::<false, { Pieces::KNIGHT }>(Squares::A1, Squares::H8),
            63 << 6 | 1 << 12,
        );
        assert_eq!(
            create_move::<true, { Pieces::KING }>(Squares::A8, Squares::H1),
            56 | 7 << 6 | 5 << 12 | 1 << 15,
        );
    }

    #[test]
    fn decompose_move_works() {
        assert_eq!(
            decompose_move(63 << 6 | 1 << 12 | 1 << 15),
            (Squares::A1, Squares::H8, Pieces::KNIGHT, Sides::WHITE),
        );
        assert_eq!(
            decompose_move(56 | 7 << 6 | 5 << 12),
            (Squares::A8, Squares::H1, Pieces::KING, Sides::BLACK),
        );
    }
}
