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
    Score(72, 109), Score(245, 280), Score(270, 304), Score(385, 477), Score(949, 861), Score(10000, 10000),
];

/// Piece-square tables, copied verbatim from
/// <https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function>.
///
/// Order: pawn, knight, bishop, rook, queen, king.
#[rustfmt::skip]
pub const INITIAL_PIECE_SQUARE_TABLES: [[Score; Square::TOTAL]; PieceType::TOTAL] = [
    [
        Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0),
        Score(-30, 30), Score(-1, 21), Score(-26, 22), Score(-41, 15), Score(-32, 18), Score(21, 6), Score(54, -3), Score(-20, -8),
        Score(-21, 21), Score(-5, 19), Score(-8, 5), Score(-15, 9), Score(-7, 2), Score(-19, 6), Score(33, -3), Score(-6, -3),
        Score(-19, 26), Score(-10, 24), Score(-11, -1), Score(7, -8), Score(3, -13), Score(0, -14), Score(-4, 7), Score(-24, -3),
        Score(-8, 48), Score(7, 36), Score(7, 16), Score(6, 8), Score(15, -6), Score(4, -7), Score(4, 13), Score(-27, 21),
        Score(20, 104), Score(3, 108), Score(52, 80), Score(52, 63), Score(65, 39), Score(60, 31), Score(-12, 60), Score(5, 70),
        Score(111, 164), Score(129, 165), Score(105, 166), Score(111, 137), Score(72, 146), Score(112, 116), Score(17, 181), Score(-7, 181),
        Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0),
    ],
    [
        Score(-104, -34), Score(-63, -81), Score(-77, -44), Score(-50, -16), Score(-50, -39), Score(-42, -36), Score(-39, -60), Score(-60, -75),
        Score(-63, -56), Score(-71, -21), Score(-36, -18), Score(-25, -5), Score(-40, -13), Score(0, -27), Score(-52, -37), Score(-40, -52),
        Score(-66, -40), Score(-36, -7), Score(-16, 8), Score(-21, 15), Score(-10, 23), Score(-6, 14), Score(4, -27), Score(-55, -29),
        Score(-44, -25), Score(-4, -22), Score(6, 30), Score(-21, 39), Score(-3, 37), Score(5, 32), Score(19, 9), Score(-31, -27),
        Score(-16, -18), Score(5, 16), Score(15, 47), Score(43, 35), Score(12, 47), Score(74, 27), Score(20, 17), Score(3, -20),
        Score(-66, -28), Score(42, -11), Score(6, 25), Score(65, 36), Score(23, 13), Score(80, -1), Score(51, -25), Score(43, -37),
        Score(-61, -22), Score(-41, -1), Score(71, -24), Score(24, 4), Score(20, -1), Score(62, -25), Score(3, -17), Score(-12, -59),
        Score(-180, -54), Score(-85, -33), Score(-40, -22), Score(-65, -37), Score(45, -44), Score(-108, -34), Score(-15, -61), Score(-96, -99),
    ],
    [
        Score(-53, -38), Score(-11, -25), Score(-38, -50), Score(-28, -19), Score(-41, -26), Score(-43, -47), Score(-38, -3), Score(-51, -35),
        Score(-6, -29), Score(-11, -27), Score(5, -18), Score(-19, -3), Score(-20, 8), Score(28, -23), Score(17, -19), Score(-17, -51),
        Score(-19, -25), Score(4, -3), Score(4, 19), Score(-8, 20), Score(0, 36), Score(-1, 10), Score(7, -17), Score(3, -18),
        Score(4, -26), Score(-6, 5), Score(7, 25), Score(7, 45), Score(6, 17), Score(-6, 21), Score(-7, -4), Score(-2, -8),
        Score(-20, -7), Score(8, 14), Score(8, 36), Score(50, 23), Score(46, 30), Score(15, 28), Score(6, 22), Score(-3, -4),
        Score(-22, 10), Score(32, 17), Score(16, 20), Score(44, 15), Score(-14, -8), Score(22, 35), Score(37, 19), Score(11, -5),
        Score(-25, -10), Score(-4, 6), Score(-47, 15), Score(-38, -12), Score(21, 9), Score(44, -4), Score(-43, 9), Score(-59, -29),
        Score(-28, -15), Score(3, -19), Score(-95, -14), Score(-51, -10), Score(-33, -14), Score(-62, -15), Score(7, -21), Score(-9, -23),
    ],
    [
        Score(-31, -13), Score(-27, -13), Score(-21, 1), Score(-9, 3), Score(-5, -7), Score(-17, -16), Score(-47, -6), Score(-37, -54),
        Score(-70, -46), Score(-43, -27), Score(-48, -16), Score(-50, -12), Score(-36, -14), Score(-4, -17), Score(-19, -33), Score(-66, -13),
        Score(-53, -17), Score(-50, -6), Score(-45, -33), Score(-57, -20), Score(-23, -18), Score(-25, -26), Score(0, -17), Score(-43, -31),
        Score(-36, -17), Score(-22, 7), Score(-22, 11), Score(-18, -2), Score(2, -10), Score(-31, 7), Score(20, -25), Score(-27, -20),
        Score(-39, 7), Score(-14, 10), Score(15, 24), Score(29, 10), Score(24, 2), Score(38, 14), Score(-2, 4), Score(-1, -8),
        Score(-4, 27), Score(39, 9), Score(19, 18), Score(59, 17), Score(39, 24), Score(80, 0), Score(73, 12), Score(31, 0),
        Score(39, 23), Score(30, 32), Score(55, 32), Score(76, 28), Score(96, 17), Score(71, 14), Score(35, 23), Score(56, 14),
        Score(40, 11), Score(53, 9), Score(24, 26), Score(28, 27), Score(38, 24), Score(1, 11), Score(36, 19), Score(50, 10),
    ],
    [
        Score(-15, -48), Score(-43, -35), Score(-26, -38), Score(3, -46), Score(-31, -23), Score(-39, -40), Score(-42, -25), Score(-48, -44),
        Score(-41, -27), Score(-33, -30), Score(4, -30), Score(-4, -31), Score(0, -18), Score(12, -51), Score(-29, -48), Score(-15, -40),
        Score(-30, -29), Score(-6, -35), Score(-20, 12), Score(-15, 2), Score(-21, -1), Score(-11, 9), Score(10, -10), Score(-23, 2),
        Score(-8, -37), Score(-18, 23), Score(-10, 19), Score(-5, 63), Score(-3, 31), Score(-25, 40), Score(6, 36), Score(-10, 18),
        Score(-30, 0), Score(-10, 31), Score(-7, 27), Score(-4, 57), Score(16, 64), Score(25, 42), Score(-5, 63), Score(5, 41),
        Score(-27, -24), Score(-10, 4), Score(14, 16), Score(25, 54), Score(46, 44), Score(71, 41), Score(75, 37), Score(53, -2),
        Score(-20, -19), Score(-30, 27), Score(-8, 38), Score(7, 47), Score(-15, 60), Score(73, 39), Score(27, 27), Score(53, 3),
        Score(-19, -2), Score(17, 24), Score(28, 18), Score(18, 36), Score(66, 43), Score(46, 23), Score(48, 7), Score(46, 16),
    ],
    [
        Score(-63, -72), Score(14, -33), Score(-20, -14), Score(-102, -20), Score(-18, -39), Score(-70, -20), Score(65, -47), Score(34, -71),
        Score(-24, -32), Score(-30, -15), Score(-43, 4), Score(-100, 10), Score(-80, 15), Score(-31, 2), Score(36, -14), Score(31, -35),
        Score(-21, -30), Score(-14, -6), Score(-26, 12), Score(-60, 19), Score(-69, 34), Score(-30, 20), Score(-22, 5), Score(-34, -18),
        Score(-49, -16), Score(-2, -5), Score(-23, 26), Score(-44, 35), Score(-34, 36), Score(-47, 31), Score(-44, 23), Score(-50, -12),
        Score(-14, -8), Score(-21, 7), Score(-8, 22), Score(-24, 24), Score(-25, 37), Score(-16, 49), Score(-2, 47), Score(-34, 19),
        Score(-10, 7), Score(24, 20), Score(7, 8), Score(-13, 30), Score(-19, 29), Score(14, 43), Score(24, 44), Score(-18, 14),
        Score(29, -15), Score(2, 20), Score(-20, 18), Score(-3, 29), Score(-3, 35), Score(3, 43), Score(-33, 48), Score(-27, 20),
        Score(-66, -78), Score(23, -30), Score(16, -18), Score(-14, -14), Score(-55, -6), Score(-33, 23), Score(2, 10), Score(11, -25),
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
