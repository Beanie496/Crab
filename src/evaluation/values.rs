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
    Score(47, 119), Score(42, 358), Score(62, 382), Score(99, 597), Score(340, 1061), Score(10000, 10000),
];

/// Piece-square tables, copied verbatim from
/// <https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function>.
///
/// Order: pawn, knight, bishop, rook, queen, king.
#[rustfmt::skip]
pub const INITIAL_PIECE_SQUARE_TABLES: [[Score; Square::TOTAL]; PieceType::TOTAL] = [
    [
        Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0),
        Score(-21, 21), Score(4, 18), Score(-24, 17), Score(-34, 12), Score(-27, 10), Score(17, 4), Score(43, -4), Score(-8, -14),
        Score(-12, 15), Score(1, 15), Score(-7, 1), Score(-12, 10), Score(-2, -1), Score(-12, 2), Score(30, -6), Score(0, -9),
        Score(-12, 17), Score(0, 18), Score(-6, -6), Score(9, -7), Score(10, -13), Score(4, -12), Score(3, 0), Score(-16, -3),
        Score(-1, 47), Score(11, 35), Score(8, 14), Score(16, 5), Score(19, -6), Score(8, -6), Score(7, 11), Score(-13, 14),
        Score(17, 108), Score(6, 110), Score(55, 81), Score(49, 65), Score(61, 40), Score(65, 28), Score(-1, 57), Score(8, 67),
        Score(93, 174), Score(85, 186), Score(133, 153), Score(109, 134), Score(74, 144), Score(38, 133), Score(-41, 207), Score(-8, 188),
        Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0),
    ],
    [
        Score(-73, -64), Score(-51, -85), Score(-84, -73), Score(-50, -13), Score(-45, -38), Score(-44, -46), Score(-34, -56), Score(-121, -86),
        Score(-69, -71), Score(-91, -12), Score(-35, -13), Score(-25, -2), Score(-34, -12), Score(-3, -20), Score(-64, -36), Score(-41, -58),
        Score(-61, -38), Score(-34, -5), Score(-12, 4), Score(-17, 19), Score(-9, 24), Score(-9, 17), Score(-2, -22), Score(-51, -20),
        Score(-40, -31), Score(0, -27), Score(3, 29), Score(-12, 34), Score(0, 37), Score(-2, 30), Score(14, 11), Score(-31, -29),
        Score(-15, -17), Score(-1, 15), Score(6, 45), Score(34, 35), Score(9, 44), Score(56, 28), Score(11, 23), Score(0, -13),
        Score(-84, -24), Score(31, -5), Score(4, 28), Score(54, 38), Score(17, 31), Score(24, 41), Score(37, -16), Score(26, -23),
        Score(-19, -41), Score(-29, -4), Score(62, -24), Score(25, 12), Score(9, 11), Score(60, -23), Score(-7, 8), Score(25, -102),
        Score(-122, -11), Score(-23, -4), Score(0, -41), Score(-100, -40), Score(-15, -47), Score(-116, -46), Score(-18, -56), Score(21, -108),
    ],
    [
        Score(-58, -58), Score(-12, -34), Score(-33, -48), Score(-35, -13), Score(-47, -20), Score(-35, -43), Score(-15, 8), Score(-66, -56),
        Score(-7, -42), Score(-9, -26), Score(3, -14), Score(-15, -7), Score(-16, 6), Score(21, -19), Score(14, -22), Score(-12, -72),
        Score(-14, -25), Score(-1, -2), Score(-3, 19), Score(-6, 18), Score(-1, 31), Score(-2, 10), Score(9, -17), Score(-2, -17),
        Score(13, -40), Score(-5, 7), Score(1, 21), Score(5, 44), Score(7, 18), Score(-5, 17), Score(-3, -1), Score(-7, -3),
        Score(-21, -6), Score(0, 11), Score(4, 33), Score(40, 19), Score(41, 30), Score(12, 29), Score(0, 23), Score(7, -2),
        Score(-23, 15), Score(20, 22), Score(17, 20), Score(37, 20), Score(-20, 5), Score(19, 41), Score(28, 22), Score(8, -13),
        Score(-16, -27), Score(-12, 17), Score(-70, 28), Score(-59, 10), Score(7, 24), Score(11, 22), Score(-71, 61), Score(-48, -66),
        Score(28, -30), Score(16, -22), Score(-109, -14), Score(-67, -2), Score(-50, -17), Score(-67, 0), Score(10, -37), Score(-15, -22),
    ],
    [
        Score(-29, -19), Score(-30, -7), Score(-22, 4), Score(-10, 6), Score(-7, -1), Score(-17, -14), Score(-47, -12), Score(-25, -54),
        Score(-62, -52), Score(-49, -19), Score(-54, -6), Score(-52, -3), Score(-43, -2), Score(-12, -12), Score(-17, -45), Score(-24, -35),
        Score(-56, -15), Score(-55, 5), Score(-44, -28), Score(-66, -11), Score(-26, -9), Score(-32, -21), Score(-9, -18), Score(-41, -36),
        Score(-28, -19), Score(-30, 11), Score(-25, 10), Score(-19, 4), Score(-2, -9), Score(-48, 14), Score(18, -28), Score(-31, -18),
        Score(-44, 9), Score(-25, 10), Score(2, 21), Score(18, 11), Score(10, 4), Score(21, 15), Score(0, 2), Score(16, -17),
        Score(-12, 23), Score(27, 6), Score(6, 21), Score(44, 16), Score(28, 18), Score(82, -14), Score(64, 10), Score(39, -8),
        Score(25, 21), Score(12, 31), Score(37, 38), Score(61, 30), Score(79, 17), Score(49, 22), Score(19, 24), Score(55, 10),
        Score(26, 18), Score(41, 9), Score(-31, 43), Score(-78, 64), Score(-56, 57), Score(-102, 40), Score(35, 22), Score(48, 10),
    ],
    [
        Score(-17, -88), Score(-50, -33), Score(-29, -58), Score(-13, -33), Score(-32, -47), Score(-55, -42), Score(-72, -37), Score(-32, -65),
        Score(-49, -23), Score(-41, -33), Score(-13, -16), Score(-21, -27), Score(-18, -6), Score(2, -76), Score(-49, -50), Score(-28, -58),
        Score(-30, -58), Score(-20, -26), Score(-36, 23), Score(-29, 11), Score(-35, 3), Score(-25, 21), Score(-4, -8), Score(-46, 24),
        Score(-14, -47), Score(-24, 12), Score(-26, 27), Score(-32, 82), Score(-21, 34), Score(-47, 66), Score(-18, 42), Score(-22, 17),
        Score(-38, 6), Score(-28, 33), Score(-22, 32), Score(-25, 70), Score(-12, 73), Score(-1, 56), Score(-31, 79), Score(-21, 59),
        Score(-39, -19), Score(-25, 4), Score(-6, 25), Score(3, 65), Score(19, 55), Score(35, 66), Score(33, 60), Score(30, 6),
        Score(-21, -41), Score(-43, 39), Score(-38, 72), Score(-13, 56), Score(-37, 85), Score(42, 62), Score(-2, 42), Score(18, 31),
        Score(-20, 11), Score(51, -13), Score(12, 27), Score(-27, 60), Score(31, 75), Score(14, 39), Score(62, -23), Score(47, 3),
    ],
    [
        Score(-108, -49), Score(27, -40), Score(-6, -23), Score(-109, -12), Score(-9, -42), Score(-52, -23), Score(68, -46), Score(43, -76),
        Score(-23, -26), Score(-24, -11), Score(-51, 8), Score(-95, 7), Score(-73, 14), Score(-16, -4), Score(42, -23), Score(36, -44),
        Score(-21, -31), Score(-11, -5), Score(-30, 14), Score(-84, 23), Score(-86, 36), Score(-33, 15), Score(-20, 6), Score(-28, -20),
        Score(-67, -11), Score(-13, -1), Score(-23, 28), Score(-83, 45), Score(-26, 29), Score(-65, 32), Score(-85, 27), Score(-32, -20),
        Score(14, -13), Score(-14, 6), Score(14, 17), Score(-15, 17), Score(-9, 32), Score(6, 42), Score(15, 37), Score(-39, 21),
        Score(-13, 8), Score(16, 25), Score(56, -2), Score(-12, 37), Score(-24, 32), Score(55, 38), Score(39, 38), Score(15, 6),
        Score(26, -31), Score(16, 19), Score(-25, 23), Score(14, 35), Score(6, 52), Score(39, 37), Score(-32, 54), Score(-22, 27),
        Score(-74, -98), Score(21, -17), Score(15, -27), Score(-9, -4), Score(-48, 7), Score(-27, 32), Score(-6, 15), Score(-2, -68),
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
