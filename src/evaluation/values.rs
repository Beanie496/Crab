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
    Score(124, 143), Score(375, 352), Score(422, 370), Score(557, 599), Score(1113, 1028), Score(10000, 10000),
];

/// Piece-square tables, copied verbatim from
/// <https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function>.
///
/// Order: pawn, knight, bishop, rook, queen, king.
#[rustfmt::skip]
pub const INITIAL_PIECE_SQUARE_TABLES: [[Score; Square::TOTAL]; PieceType::TOTAL] = [
    [
        Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0),
        Score(-45, 56), Score(10, 48), Score(-39, 48), Score(-48, 21), Score(-36, 40), Score(44, 30), Score(78, 28), Score(-25, 15),
        Score(-28, 42), Score(3, 41), Score(-6, 19), Score(-17, 25), Score(4, 16), Score(-6, 19), Score(62, 16), Score(-6, 15),
        Score(-28, 49), Score(5, 47), Score(-7, 14), Score(26, 5), Score(31, -1), Score(12, 4), Score(15, 25), Score(-35, 21),
        Score(0, 86), Score(32, 71), Score(20, 44), Score(42, 29), Score(47, 15), Score(24, 22), Score(27, 49), Score(-26, 50),
        Score(48, 162), Score(51, 167), Score(88, 149), Score(87, 124), Score(121, 100), Score(116, 96), Score(53, 132), Score(14, 138),
        Score(174, 256), Score(208, 251), Score(137, 234), Score(170, 208), Score(136, 219), Score(177, 197), Score(82, 237), Score(48, 259),
        Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0),
    ],
    [
        Score(-114, -51), Score(-66, -103), Score(-109, -71), Score(-67, -39), Score(-48, -54), Score(-65, -60), Score(-44, -86), Score(-82, -95),
        Score(-79, -79), Score(-97, -33), Score(-32, -29), Score(-16, -14), Score(-21, -14), Score(27, -47), Score(-62, -57), Score(-53, -66),
        Score(-69, -55), Score(-31, -10), Score(11, 8), Score(5, 36), Score(21, 30), Score(20, 10), Score(34, -44), Score(-55, -45),
        Score(-42, -33), Score(13, -8), Score(29, 48), Score(14, 64), Score(41, 53), Score(29, 46), Score(46, 15), Score(-27, -29),
        Score(-14, -15), Score(27, 26), Score(37, 70), Score(97, 61), Score(60, 68), Score(127, 44), Score(41, 30), Score(34, -18),
        Score(-79, -25), Score(97, -6), Score(56, 47), Score(121, 56), Score(126, 36), Score(176, 28), Score(114, -8), Score(74, -33),
        Score(-74, -16), Score(-42, 9), Score(123, -9), Score(72, 36), Score(48, 18), Score(109, -13), Score(31, -3), Score(-3, -62),
        Score(-163, -32), Score(-73, -19), Score(-12, -1), Score(-47, -15), Score(76, -14), Score(-93, -16), Score(-14, -56), Score(-78, -91),
    ],
    [
        Score(-74, -60), Score(-4, -35), Score(-38, -47), Score(-45, -1), Score(-41, -19), Score(-33, -40), Score(-38, -7), Score(-67, -49),
        Score(5, -26), Score(17, -29), Score(31, -4), Score(-4, 7), Score(3, 24), Score(53, -15), Score(55, -18), Score(-3, -51),
        Score(-2, -16), Score(24, 14), Score(23, 43), Score(21, 45), Score(24, 58), Score(37, 23), Score(39, -8), Score(19, -14),
        Score(12, -2), Score(17, 31), Score(25, 54), Score(46, 72), Score(54, 40), Score(19, 43), Score(23, 7), Score(8, 8),
        Score(-11, 18), Score(17, 41), Score(38, 61), Score(98, 48), Score(87, 60), Score(65, 55), Score(18, 45), Score(19, 24),
        Score(-21, 46), Score(68, 33), Score(77, 38), Score(86, 36), Score(43, 23), Score(95, 53), Score(77, 32), Score(19, 13),
        Score(-30, 13), Score(19, 38), Score(-43, 47), Score(-8, 14), Score(62, 34), Score(90, 16), Score(4, 40), Score(-66, -27),
        Score(11, 20), Score(22, 1), Score(-78, 16), Score(-31, 25), Score(-11, 12), Score(-16, 15), Score(15, -6), Score(1, -18),
    ],
    [
        Score(-13, -1), Score(-8, 24), Score(12, 37), Score(38, 37), Score(38, 28), Score(19, 11), Score(-58, 21), Score(-31, -29),
        Score(-68, -29), Score(-26, -1), Score(-30, 19), Score(-24, 22), Score(-10, 12), Score(28, 14), Score(-7, -14), Score(-73, -6),
        Score(-61, 12), Score(-43, 40), Score(-23, -2), Score(-42, 16), Score(8, 17), Score(-2, 5), Score(7, 19), Score(-46, -19),
        Score(-30, 27), Score(-24, 53), Score(-8, 57), Score(11, 47), Score(35, 24), Score(-22, 42), Score(39, 10), Score(-29, 13),
        Score(-25, 53), Score(-2, 53), Score(43, 68), Score(71, 50), Score(64, 47), Score(83, 54), Score(13, 40), Score(12, 39),
        Score(25, 67), Score(68, 61), Score(68, 65), Score(95, 64), Score(70, 61), Score(109, 48), Score(117, 54), Score(65, 40),
        Score(76, 74), Score(78, 78), Score(119, 84), Score(129, 81), Score(148, 63), Score(121, 66), Score(71, 71), Score(98, 59),
        Score(74, 78), Score(74, 72), Score(58, 94), Score(73, 90), Score(75, 84), Score(24, 76), Score(61, 74), Score(85, 65),
    ],
    [
        Score(-3, -47), Score(-37, -30), Score(-10, -36), Score(31, -34), Score(-19, -21), Score(-54, -44), Score(-51, -28), Score(-55, -48),
        Score(-47, -18), Score(-15, -23), Score(34, -8), Score(18, -16), Score(27, 5), Score(46, -44), Score(-29, -38), Score(-4, -33),
        Score(-13, -9), Score(16, -12), Score(-9, 62), Score(8, 39), Score(-2, 32), Score(14, 64), Score(46, 28), Score(2, 32),
        Score(9, -2), Score(-8, 68), Score(3, 74), Score(11, 120), Score(19, 86), Score(-3, 103), Score(24, 103), Score(13, 55),
        Score(-21, 41), Score(-10, 70), Score(7, 80), Score(14, 111), Score(42, 132), Score(59, 107), Score(17, 133), Score(26, 97),
        Score(-14, 6), Score(-4, 46), Score(38, 63), Score(59, 123), Score(86, 121), Score(122, 110), Score(105, 81), Score(112, 65),
        Score(-16, 8), Score(-36, 84), Score(15, 100), Score(37, 107), Score(22, 130), Score(120, 96), Score(77, 92), Score(108, 61),
        Score(18, 56), Score(61, 87), Score(83, 91), Score(46, 95), Score(129, 105), Score(86, 80), Score(91, 64), Score(105, 86),
    ],
    [
        Score(-74, -104), Score(36, -48), Score(-12, -30), Score(-112, -49), Score(-20, -51), Score(-77, -34), Score(59, -31), Score(26, -78),
        Score(-30, -50), Score(-22, -24), Score(-48, 3), Score(-121, -1), Score(-100, 16), Score(-42, 6), Score(28, -9), Score(20, -41),
        Score(-32, -53), Score(-23, -10), Score(-39, 10), Score(-93, 26), Score(-97, 42), Score(-56, 25), Score(-36, 10), Score(-52, -32),
        Score(-69, -38), Score(-10, -10), Score(-34, 35), Score(-54, 41), Score(-40, 41), Score(-65, 37), Score(-64, 13), Score(-64, -30),
        Score(-16, -6), Score(-22, 29), Score(1, 37), Score(-13, 37), Score(-22, 49), Score(-13, 63), Score(-4, 51), Score(-43, 17),
        Score(-8, 26), Score(29, 50), Score(22, 38), Score(-12, 50), Score(-19, 47), Score(32, 82), Score(42, 85), Score(-18, 20),
        Score(33, 14), Score(10, 62), Score(-17, 46), Score(3, 60), Score(0, 59), Score(10, 75), Score(-36, 48), Score(-24, 40),
        Score(-66, -71), Score(27, -4), Score(18, 3), Score(-11, 7), Score(-52, 15), Score(-30, 35), Score(4, 30), Score(14, -4),
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
