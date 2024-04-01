use super::Score;
use crate::{
    cfor,
    defs::{Piece, PieceType, Side},
};

/// Values in centipawns for each piece.
///
/// Order: pawn, knight, bishop, rook, queen, king.
#[rustfmt::skip]
const BASE_PIECE_VALUES: [Score; PieceType::TOTAL] = [
    Score(100), Score(300), Score(300), Score(500), Score(900), Score(10_000),
];

/// Creates the initial table of piece values for White and Black with an extra
/// value of 0 at the end.
pub const fn create_piece_values() -> [Score; Piece::TOTAL + 1] {
    let mut piece_values = [Score(0); Piece::TOTAL + 1];
    cfor!(let mut piece_type = 0; piece_type < PieceType::TOTAL; piece_type += 1; {
        let w_piece = Piece::from_piecetype(PieceType(piece_type as u8), Side::WHITE);
        let b_piece = Piece::from_piecetype(PieceType(piece_type as u8), Side::BLACK);
        piece_values[w_piece.to_index()] = BASE_PIECE_VALUES[piece_type];
        let Score(value) = BASE_PIECE_VALUES[piece_type];
        piece_values[b_piece.to_index()] = Score(-value);
    });
    piece_values[piece_values.len() - 1] = Score(0);
    piece_values
}
