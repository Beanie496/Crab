use super::Score;
use crate::defs::{Piece, PieceType, Side, Square};

/// A C-style `for` loop to allow easier looping in `const` functions.
// idea for this is from viridithas
macro_rules! cfor {
    ($stmt:stmt; $cond:expr; $expr:expr; $body:block) => {{
        $stmt
        while ($cond) {
            $body;
            $expr;
        }
    }}
}

/// Values in centipawns for each piece.
///
/// Order: pawn, knight, bishop, rook, queen, king.
#[rustfmt::skip]
const PIECE_VALUES: [Score; PieceType::TOTAL] = [
    Score(82, 94), Score(337, 281), Score(365, 297), Score(477, 512), Score(1025, 936), Score(10_000, 10_000),
];

/// Piece-square tables, copied verbatim from
/// <https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function>.
#[rustfmt::skip]
const INITIAL_PIECE_SQUARE_TABLES: [[Score; Square::TOTAL]; PieceType::TOTAL] = [
[
    Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0),
    Score(-35, 13), Score(-1, 8), Score(-20, 8), Score(-23, 10), Score(-15, 13), Score(24, 0), Score(38, 2), Score(-22, -7),
    Score(-26, 4), Score(-4, 7), Score(-4, -6), Score(-10, 1), Score(3, 0), Score(3, -5), Score(33, -1), Score(-12, -8),
    Score(-27, 13), Score(-2, 9), Score(-5, -3), Score(12, -7), Score(17, -7), Score(6, -8), Score(10, 3), Score(-25, -1),
    Score(-14, 32), Score(13, 24), Score(6, 13), Score(21, 5), Score(23, -2), Score(12, 4), Score(17, 17), Score(-23, 17),
    Score(-6, 94), Score(7, 100), Score(26, 85), Score(31, 67), Score(65, 56), Score(56, 53), Score(25, 82), Score(-20, 84),
    Score(98, 178), Score(134, 173), Score(61, 158), Score(95, 134), Score(68, 147), Score(126, 132), Score(34, 165), Score(-11, 187),
    Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0),
],
[
    Score(-105, -29), Score(-21, -51), Score(-58, -23), Score(-33, -15), Score(-17, -22), Score(-28, -18), Score(-19, -50), Score(-23, -64),
    Score(-29, -42), Score(-53, -20), Score(-12, -10), Score(-3, -5), Score(-1, -2), Score(18, -20), Score(-14, -23), Score(-19, -44),
    Score(-23, -23), Score(-9, -3), Score(12, -1), Score(10, 15), Score(19, 10), Score(17, -3), Score(25, -20), Score(-16, -22),
    Score(-13, -18), Score(4, -6), Score(16, 16), Score(13, 25), Score(28, 16), Score(19, 17), Score(21, 4), Score(-8, -18),
    Score(-9, -17), Score(17, 3), Score(19, 22), Score(53, 22), Score(37, 22), Score(69, 11), Score(18, 8), Score(22, -18),
    Score(-47, -24), Score(60, -20), Score(37, 10), Score(65, 9), Score(84, -1), Score(129, -9), Score(73, -19), Score(44, -41),
    Score(-73, -25), Score(-41, -8), Score(72, -25), Score(36, -2), Score(23, -9), Score(62, -25), Score(7, -24), Score(-17, -52),
    Score(-167, -58), Score(-89, -38), Score(-34, -13), Score(-49, -28), Score(61, -31), Score(-97, -27), Score(-15, -63), Score(-107, -99),
],
[
    Score(-33, -23), Score(-3, -9), Score(-14, -23), Score(-21, -5), Score(-13, -9), Score(-12, -16), Score(-39, -5), Score(-21, -17),
    Score(4, -14), Score(15, -18), Score(16, -7), Score(0, -1), Score(7, 4), Score(21, -9), Score(33, -15), Score(1, -27),
    Score(0, -12), Score(15, -3), Score(15, 8), Score(15, 10), Score(14, 13), Score(27, 3), Score(18, -7), Score(10, -15),
    Score(-6, -6), Score(13, 3), Score(13, 13), Score(26, 19), Score(34, 7), Score(12, 10), Score(10, -3), Score(4, -9),
    Score(-4, -3), Score(5, 9), Score(19, 12), Score(50, 9), Score(37, 14), Score(37, 10), Score(7, 3), Score(-2, 2),
    Score(-16, 2), Score(37, -8), Score(43, 0), Score(40, -1), Score(35, -2), Score(50, 6), Score(37, 0), Score(-2, 4),
    Score(-26, -8), Score(16, -4), Score(-18, 7), Score(-13, -12), Score(30, -3), Score(59, -13), Score(18, -4), Score(-47, -14),
    Score(-29, -14), Score(4, -21), Score(-82, -11), Score(-37, -8), Score(-25, -7), Score(-42, -9), Score(7, -17), Score(-8, -24),
],
[
    Score(-19, -9), Score(-13, 2), Score(1, 3), Score(17, -1), Score(16, -5), Score(7, -13), Score(-37, 4), Score(-26, -20),
    Score(-44, -6), Score(-16, -6), Score(-20, 0), Score(-9, 2), Score(-1, -9), Score(11, -9), Score(-6, -11), Score(-71, -3),
    Score(-45, -4), Score(-25, 0), Score(-16, -5), Score(-17, -1), Score(3, -7), Score(0, -12), Score(-5, -8), Score(-33, -16),
    Score(-36, 3), Score(-26, 5), Score(-12, 8), Score(-1, 4), Score(9, -5), Score(-7, -6), Score(6, -8), Score(-23, -11),
    Score(-24, 4), Score(-11, 3), Score(7, 13), Score(26, 1), Score(24, 2), Score(35, 1), Score(-8, -1), Score(-20, 2),
    Score(-5, 7), Score(19, 7), Score(26, 7), Score(36, 5), Score(17, 4), Score(45, -3), Score(61, -5), Score(16, -3),
    Score(27, 11), Score(32, 13), Score(58, 13), Score(62, 11), Score(80, -3), Score(67, 3), Score(26, 8), Score(44, 3),
    Score(32, 13), Score(42, 10), Score(32, 18), Score(51, 15), Score(63, 12), Score(9, 12), Score(31, 8), Score(43, 5),
],
[
    Score(-1, -33), Score(-18, -28), Score(-9, -22), Score(10, -43), Score(-15, -5), Score(-25, -32), Score(-31, -20), Score(-50, -41),
    Score(-35, -22), Score(-8, -23), Score(11, -30), Score(2, -16), Score(8, -16), Score(15, -23), Score(-3, -36), Score(1, -32),
    Score(-14, -16), Score(2, -27), Score(-11, 15), Score(-2, 6), Score(-5, 9), Score(2, 17), Score(14, 10), Score(5, 5),
    Score(-9, -18), Score(-26, 28), Score(-9, 19), Score(-10, 47), Score(-2, 31), Score(-4, 34), Score(3, 39), Score(-3, 23),
    Score(-27, 3), Score(-27, 22), Score(-16, 24), Score(-16, 45), Score(-1, 57), Score(17, 40), Score(-2, 57), Score(1, 36),
    Score(-13, -20), Score(-17, 6), Score(7, 9), Score(8, 49), Score(29, 47), Score(56, 35), Score(47, 19), Score(57, 9),
    Score(-24, -17), Score(-39, 20), Score(-5, 32), Score(1, 41), Score(-16, 58), Score(57, 25), Score(28, 30), Score(54, 0),
    Score(-28, -9), Score(0, 22), Score(29, 22), Score(12, 27), Score(59, 27), Score(44, 19), Score(43, 10), Score(45, 20),
],
[
    Score(-15, -53), Score(36, -34), Score(12, -21), Score(-54, -11), Score(8, -28), Score(-28, -14), Score(24, -24), Score(14, -43),
    Score(1, -27), Score(7, -11), Score(-8, 4), Score(-64, 13), Score(-43, 14), Score(-16, 4), Score(9, -5), Score(8, -17),
    Score(-14, -19), Score(-14, -3), Score(-22, 11), Score(-46, 21), Score(-44, 23), Score(-30, 16), Score(-15, 7), Score(-27, -9),
    Score(-49, -18), Score(-1, -4), Score(-27, 21), Score(-39, 24), Score(-46, 27), Score(-44, 23), Score(-33, 9), Score(-51, -11),
    Score(-17, -8), Score(-20, 22), Score(-12, 24), Score(-27, 27), Score(-30, 26), Score(-25, 33), Score(-14, 26), Score(-36, 3),
    Score(-9, 10), Score(24, 17), Score(2, 23), Score(-16, 15), Score(-20, 20), Score(6, 45), Score(22, 44), Score(-22, 13),
    Score(29, -12), Score(-1, 17), Score(-20, 14), Score(-7, 17), Score(-8, 17), Score(-4, 38), Score(-38, 23), Score(-29, 11),
    Score(-65, -74), Score(23, -35), Score(16, -18), Score(-15, -18), Score(-56, -11), Score(-34, 15), Score(2, 4), Score(13, -17),
],
];

/// Creates piece-square tables for both sides from internal initial White-only
/// piece-square tables.
// TODO: not too sure about allowing `module_name_repetitions`...maybe rename
// the module and/or function?
#[allow(clippy::module_name_repetitions, clippy::similar_names)]
pub const fn create_piece_square_tables() -> [[Score; Square::TOTAL]; Piece::TOTAL + 1] {
    let mut psqt = [[Score(0, 0); Square::TOTAL]; Piece::TOTAL + 1];
    cfor!(let mut piece = 0; piece < PieceType::TOTAL; piece += 1; {
        let w_piece_index = Piece::from(PieceType::from(piece as u8), Side::WHITE).to_index();
        let b_piece_index = Piece::from(PieceType::from(piece as u8), Side::BLACK).to_index();
        cfor!(let mut square = 0; square < Square::TOTAL; square += 1; {
            let flipped_square = square ^ 56;
            let Score(mut mg, mut eg) = PIECE_VALUES[piece];
            let Score(mg_psq, eg_psq) = INITIAL_PIECE_SQUARE_TABLES[piece][square];
            mg += mg_psq;
            eg += eg_psq;
            psqt[w_piece_index][square] = Score(mg, eg);
            psqt[b_piece_index][flipped_square] = Score(-mg, -eg);
        });
    });
    psqt
}
