use crate::defs::{ Move, Piece, Side, Square };

/// Returns a Move given a start square, end square, piece and side.
pub fn create_move(start: Square, end: Square, piece: Piece, side: Side) -> Move {
    start as Move
        | ((end as Move) << 6)
        | ((piece as Move) << 12)
        | ((side as Move) << 15)
}

/// Returns a tuple of a start square, end square, piece and side given a Move.
pub fn decompose_move(mv: Move) -> (Square, Square, Piece, Side) {
    let start = mv & 0x3f;
    let end = (mv >> 6) & 0x3f;
    let piece = (mv >> 12) & 0x7;
    let side = (mv >> 15) & 0x1;
    (start as Square, end as Square, piece as Piece, side as Side)
}
