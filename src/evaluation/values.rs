/*
 * Crab, a UCI-compatible chess engine
 * Copyright (C) 2024 Jasper Shovelton
 *
 * Crab is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Crab is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE. See the GNU General Public License for more
 * details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Crab. If not, see <https://www.gnu.org/licenses/>.
 */

use super::Score;
use crate::{
    cfor,
    defs::{Piece, PieceType, Side, Square},
};

/// Values in centipawns for each piece.
///
/// Order: pawn, knight, bishop, rook, queen, king.
#[rustfmt::skip]
pub const BASE_PIECE_VALUES: [Score; PieceType::TOTAL] = [
    Score(60, 117), Score(181, 323), Score(205, 347), Score(308, 527), Score(909, 863), Score(10000, 10000),
];

/// Piece-square tables, copied verbatim from
/// <https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function>.
///
/// Order: pawn, knight, bishop, rook, queen, king.
#[rustfmt::skip]
pub const INITIAL_PIECE_SQUARE_TABLES: [[Score; Square::TOTAL]; PieceType::TOTAL] = [
    [
        Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0),
        Score(-26, 29), Score(-1, 21), Score(-24, 20), Score(-44, 22), Score(-31, 20), Score(17, 7), Score(52, -2), Score(-19, -8),
        Score(-17, 21), Score(-4, 19), Score(-8, 5), Score(-15, 8), Score(-9, 3), Score(-27, 10), Score(30, -3), Score(-7, -3),
        Score(-16, 26), Score(-10, 25), Score(-10, -1), Score(7, -8), Score(2, -12), Score(-4, -13), Score(-8, 10), Score(-21, -5),
        Score(-6, 47), Score(8, 37), Score(9, 15), Score(7, 7), Score(16, -8), Score(5, -9), Score(4, 12), Score(-29, 22),
        Score(33, 99), Score(5, 109), Score(48, 82), Score(54, 60), Score(62, 38), Score(57, 31), Score(-23, 67), Score(5, 71),
        Score(124, 162), Score(122, 158), Score(118, 164), Score(119, 137), Score(78, 140), Score(89, 120), Score(14, 193), Score(-8, 180),
        Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0),
    ],
    [
        Score(-111, -43), Score(-64, -84), Score(-84, -53), Score(-73, -15), Score(-56, -50), Score(-46, -37), Score(-32, -70), Score(-79, -86),
        Score(-65, -73), Score(-80, -25), Score(-34, -31), Score(-26, -5), Score(-44, -5), Score(1, -35), Score(-48, -33), Score(-30, -51),
        Score(-71, -45), Score(-37, -7), Score(-18, 15), Score(-29, 16), Score(-16, 31), Score(-10, 18), Score(1, -23), Score(-63, -34),
        Score(-45, -11), Score(-5, -31), Score(3, 34), Score(-29, 46), Score(-8, 42), Score(1, 40), Score(16, 11), Score(-27, -28),
        Score(-18, -12), Score(3, 19), Score(20, 46), Score(46, 32), Score(6, 55), Score(68, 39), Score(17, 16), Score(0, -22),
        Score(-68, -23), Score(37, 2), Score(10, 24), Score(54, 45), Score(-9, 25), Score(45, 5), Score(43, -10), Score(51, -32),
        Score(-59, -22), Score(-32, 6), Score(69, -12), Score(3, 10), Score(18, 1), Score(42, -24), Score(3, -15), Score(-23, -54),
        Score(-173, -53), Score(-87, -33), Score(-52, -27), Score(-64, -45), Score(36, -41), Score(-115, -39), Score(-15, -54), Score(-86, -95),
    ],
    [
        Score(-48, -42), Score(-9, -31), Score(-37, -51), Score(-33, -31), Score(-50, -32), Score(-38, -55), Score(-41, -13), Score(-56, -44),
        Score(-4, -34), Score(-14, -21), Score(5, -24), Score(-22, 5), Score(-23, 14), Score(28, -26), Score(13, -11), Score(-13, -67),
        Score(-24, -21), Score(1, 1), Score(3, 20), Score(-12, 24), Score(-4, 43), Score(-6, 15), Score(1, -2), Score(5, -17),
        Score(3, -29), Score(-13, 7), Score(7, 26), Score(1, 48), Score(0, 22), Score(-12, 32), Score(-14, -4), Score(-1, -10),
        Score(-23, 0), Score(4, 21), Score(4, 43), Score(51, 24), Score(47, 28), Score(8, 33), Score(6, 24), Score(-13, -4),
        Score(-13, 9), Score(37, 21), Score(8, 27), Score(40, 18), Score(-40, -10), Score(-8, 50), Score(31, 31), Score(20, -6),
        Score(-4, 4), Score(-6, 8), Score(-28, 26), Score(-52, -19), Score(24, 10), Score(15, -4), Score(-83, 12), Score(-33, -17),
        Score(-33, -16), Score(-2, -12), Score(-104, -10), Score(-66, -15), Score(-44, -18), Score(-80, -32), Score(4, -24), Score(-1, -15),
    ],
    [
        Score(-47, -2), Score(-39, -16), Score(-36, 9), Score(-25, 10), Score(-22, 1), Score(-34, -8), Score(-58, -1), Score(-62, -49),
        Score(-76, -59), Score(-50, -46), Score(-57, -28), Score(-64, -18), Score(-50, -24), Score(-14, -19), Score(-23, -40), Score(-75, -29),
        Score(-66, -18), Score(-65, -12), Score(-55, -53), Score(-86, -16), Score(-41, -16), Score(-37, -29), Score(1, -29), Score(-45, -40),
        Score(-50, -15), Score(-30, 6), Score(-47, 25), Score(-47, 8), Score(-20, 3), Score(-32, 7), Score(18, -36), Score(-22, -37),
        Score(-45, 9), Score(-26, 17), Score(5, 33), Score(14, 15), Score(21, -1), Score(26, 23), Score(2, 4), Score(-11, -7),
        Score(-11, 32), Score(29, 17), Score(13, 20), Score(63, 16), Score(47, 33), Score(89, 5), Score(72, 17), Score(41, 1),
        Score(26, 30), Score(20, 39), Score(45, 38), Score(66, 34), Score(87, 24), Score(69, 16), Score(45, 27), Score(61, 21),
        Score(34, 11), Score(51, 10), Score(11, 30), Score(-3, 39), Score(10, 33), Score(-11, 12), Score(33, 22), Score(58, 15),
    ],
    [
        Score(-24, -48), Score(-52, -41), Score(-34, -40), Score(2, -55), Score(-34, -26), Score(-53, -46), Score(-45, -30), Score(-48, -45),
        Score(-50, -37), Score(-42, -32), Score(2, -28), Score(-4, -38), Score(-1, -26), Score(12, -56), Score(-25, -47), Score(-28, -44),
        Score(-42, -32), Score(-8, -37), Score(-23, 12), Score(-17, -6), Score(-24, 0), Score(-17, 11), Score(8, -19), Score(-19, -3),
        Score(-9, -50), Score(-23, 23), Score(-12, 19), Score(-10, 66), Score(-3, 32), Score(-27, 32), Score(2, 41), Score(-15, 17),
        Score(-23, 0), Score(-10, 33), Score(-7, 32), Score(-3, 65), Score(22, 72), Score(24, 47), Score(-10, 80), Score(5, 34),
        Score(-14, -32), Score(-5, 6), Score(17, 17), Score(30, 58), Score(43, 58), Score(76, 44), Score(75, 40), Score(49, -8),
        Score(-13, -9), Score(-26, 29), Score(-4, 28), Score(13, 53), Score(-14, 57), Score(72, 45), Score(39, 37), Score(59, 6),
        Score(-3, 3), Score(18, 24), Score(27, 21), Score(13, 38), Score(56, 47), Score(57, 33), Score(55, 10), Score(44, 14),
    ],
    [
        Score(-60, -70), Score(15, -32), Score(-16, -10), Score(-104, -23), Score(-3, -53), Score(-54, -27), Score(66, -51), Score(41, -80),
        Score(-43, -43), Score(-53, -17), Score(-42, 3), Score(-90, 10), Score(-81, 16), Score(-31, 1), Score(38, -16), Score(36, -41),
        Score(-30, -46), Score(-12, -16), Score(-39, 15), Score(-60, 20), Score(-77, 37), Score(-39, 23), Score(-23, 4), Score(-33, -21),
        Score(-46, -20), Score(-8, -15), Score(-22, 26), Score(-48, 37), Score(-29, 36), Score(-48, 31), Score(-46, 27), Score(-55, -13),
        Score(-16, -10), Score(-22, 3), Score(-7, 31), Score(-19, 36), Score(-25, 39), Score(-17, 50), Score(6, 54), Score(-32, 23),
        Score(-12, 4), Score(25, 17), Score(8, 11), Score(-11, 36), Score(-16, 35), Score(17, 39), Score(28, 49), Score(-16, 16),
        Score(26, -18), Score(3, 19), Score(-22, 10), Score(-1, 29), Score(-4, 34), Score(8, 41), Score(-30, 63), Score(-25, 26),
        Score(-67, -83), Score(24, -29), Score(15, -17), Score(-11, -6), Score(-54, 0), Score(-30, 34), Score(4, 17), Score(11, -25),
    ],
];

/// Creates the initial piece-square table for White and Black, with an extra
/// table of 0's at the end to allow [`Piece::NONE`] to index into it.
#[allow(clippy::similar_names)]
pub const fn create_piece_square_tables() -> [[Score; Square::TOTAL]; Piece::TOTAL + 1] {
    let mut psqt = [[Score(0, 0); Square::TOTAL]; Piece::TOTAL + 1];
    cfor!(let mut piece = 0; piece < PieceType::TOTAL; piece += 1; {
        let w_piece = Piece::from_piecetype(PieceType(piece as u8), Side::WHITE);
        let b_piece = Piece::from_piecetype(PieceType(piece as u8), Side::BLACK);
        cfor!(let mut square = 0; square < Square::TOTAL; square += 1; {
            let flipped_square = square ^ 56;
            let Score(mut mg, mut eg) = BASE_PIECE_VALUES[piece];
            let Score(mg_psq, eg_psq) = INITIAL_PIECE_SQUARE_TABLES[piece][square];
            mg += mg_psq;
            eg += eg_psq;
            psqt[w_piece.to_index()][square] = Score(mg, eg);
            psqt[b_piece.to_index()][flipped_square] = Score(-mg, -eg);
        });
    });
    psqt
}
