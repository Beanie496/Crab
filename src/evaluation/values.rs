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
    Score(77, 106), Score(254, 273), Score(278, 296), Score(393, 482), Score(955, 867), Score(10000, 10000),
];

/// Piece-square tables, copied verbatim from
/// <https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function>.
///
/// Order: pawn, knight, bishop, rook, queen, king.
#[rustfmt::skip]
pub const INITIAL_PIECE_SQUARE_TABLES: [[Score; Square::TOTAL]; PieceType::TOTAL] = [
    [
        Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0),
        Score(-32, 26), Score(-2, 20), Score(-32, 21), Score(-37, 14), Score(-33, 15), Score(18, 6), Score(48, -4), Score(-17, -10),
        Score(-24, 20), Score(-4, 18), Score(-11, 4), Score(-14, 13), Score(-3, 1), Score(-12, 3), Score(35, -6), Score(-8, -5),
        Score(-22, 22), Score(-7, 22), Score(-11, -3), Score(6, -5), Score(8, -11), Score(5, -12), Score(2, 2), Score(-26, 2),
        Score(-9, 50), Score(8, 37), Score(3, 16), Score(11, 8), Score(15, -3), Score(4, -3), Score(7, 13), Score(-22, 17),
        Score(14, 107), Score(3, 110), Score(54, 80), Score(50, 66), Score(66, 40), Score(61, 31), Score(-10, 57), Score(4, 69),
        Score(109, 167), Score(131, 170), Score(103, 167), Score(109, 136), Score(72, 147), Score(115, 116), Score(17, 179), Score(-7, 182),
        Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0), Score(0, 0),
    ],
    [
        Score(-103, -33), Score(-57, -79), Score(-76, -43), Score(-44, -14), Score(-47, -36), Score(-41, -36), Score(-35, -57), Score(-58, -74),
        Score(-62, -54), Score(-70, -21), Score(-35, -15), Score(-26, -4), Score(-34, -14), Score(1, -25), Score(-52, -37), Score(-43, -53),
        Score(-63, -38), Score(-34, -7), Score(-12, 3), Score(-16, 17), Score(-5, 21), Score(-7, 14), Score(3, -28), Score(-50, -27),
        Score(-45, -28), Score(-3, -20), Score(5, 28), Score(-15, 36), Score(1, 36), Score(3, 27), Score(19, 8), Score(-35, -28),
        Score(-16, -19), Score(0, 12), Score(9, 43), Score(37, 33), Score(11, 41), Score(69, 21), Score(15, 16), Score(5, -19),
        Score(-66, -29), Score(40, -14), Score(4, 24), Score(65, 31), Score(29, 13), Score(85, 0), Score(52, -27), Score(41, -38),
        Score(-61, -22), Score(-43, -3), Score(70, -27), Score(27, 4), Score(20, -1), Score(65, -25), Score(3, -17), Score(-10, -60),
        Score(-180, -54), Score(-85, -33), Score(-39, -21), Score(-65, -36), Score(46, -44), Score(-107, -33), Score(-15, -62), Score(-97, -99),
    ],
    [
        Score(-53, -38), Score(-12, -24), Score(-35, -49), Score(-25, -14), Score(-39, -24), Score(-38, -43), Score(-37, -1), Score(-50, -34),
        Score(-7, -28), Score(-8, -28), Score(3, -16), Score(-16, -6), Score(-17, 6), Score(28, -22), Score(18, -25), Score(-18, -49),
        Score(-16, -25), Score(0, -5), Score(-1, 17), Score(-6, 17), Score(-1, 30), Score(2, 8), Score(11, -20), Score(0, -19),
        Score(3, -25), Score(-4, 5), Score(1, 21), Score(8, 42), Score(10, 16), Score(-5, 17), Score(-4, -3), Score(-4, -8),
        Score(-20, -8), Score(3, 9), Score(6, 31), Score(44, 18), Score(44, 30), Score(17, 26), Score(1, 20), Score(3, -1),
        Score(-24, 10), Score(27, 13), Score(17, 17), Score(44, 15), Score(-9, -6), Score(28, 32), Score(36, 14), Score(5, -7),
        Score(-29, -13), Score(-2, 6), Score(-51, 12), Score(-36, -11), Score(20, 8), Score(49, -2), Score(-37, 9), Score(-63, -31),
        Score(-27, -15), Score(3, -20), Score(-94, -14), Score(-49, -9), Score(-32, -13), Score(-60, -13), Score(7, -21), Score(-10, -24),
    ],
    [
        Score(-29, -18), Score(-28, -7), Score(-18, 3), Score(-4, 4), Score(-1, -4), Score(-15, -15), Score(-49, -8), Score(-30, -47),
        Score(-68, -43), Score(-41, -23), Score(-44, -11), Score(-44, -6), Score(-30, -7), Score(-5, -13), Score(-19, -32), Score(-64, -10),
        Score(-51, -15), Score(-46, -1), Score(-42, -28), Score(-52, -17), Score(-17, -13), Score(-24, -24), Score(-4, -15), Score(-44, -30),
        Score(-34, -15), Score(-22, 8), Score(-18, 8), Score(-12, 1), Score(6, -11), Score(-34, 6), Score(18, -22), Score(-29, -17),
        Score(-39, 6), Score(-16, 6), Score(10, 18), Score(29, 8), Score(19, 1), Score(35, 9), Score(-4, 3), Score(0, -7),
        Score(-8, 22), Score(35, 4), Score(17, 16), Score(53, 13), Score(33, 17), Score(75, -5), Score(71, 8), Score(28, -2),
        Score(37, 18), Score(26, 26), Score(54, 31), Score(75, 26), Score(94, 12), Score(71, 14), Score(32, 20), Score(54, 11),
        Score(41, 13), Score(52, 8), Score(25, 26), Score(32, 29), Score(41, 24), Score(2, 12), Score(37, 19), Score(49, 9),
    ],
    [
        Score(-12, -47), Score(-40, -34), Score(-20, -36), Score(4, -43), Score(-29, -22), Score(-36, -39), Score(-42, -25), Score(-48, -44),
        Score(-39, -25), Score(-29, -29), Score(6, -30), Score(-5, -30), Score(1, -16), Score(11, -51), Score(-29, -48), Score(-13, -39),
        Score(-26, -28), Score(-4, -34), Score(-18, 13), Score(-11, 4), Score(-18, 0), Score(-5, 12), Score(14, -7), Score(-24, 2),
        Score(-3, -34), Score(-17, 23), Score(-11, 19), Score(-6, 62), Score(-6, 30), Score(-22, 42), Score(3, 34), Score(-6, 19),
        Score(-32, 0), Score(-14, 29), Score(-9, 26), Score(-9, 54), Score(11, 60), Score(23, 41), Score(-6, 60), Score(1, 41),
        Score(-30, -24), Score(-12, 3), Score(12, 15), Score(20, 52), Score(44, 41), Score(68, 40), Score(71, 35), Score(55, 0),
        Score(-23, -21), Score(-30, 27), Score(-7, 41), Score(5, 45), Score(-14, 61), Score(73, 39), Score(24, 25), Score(52, 2),
        Score(-21, -3), Score(17, 24), Score(28, 18), Score(19, 36), Score(69, 45), Score(44, 21), Score(47, 6), Score(47, 16),
    ],
    [
        Score(-63, -72), Score(19, -33), Score(-16, -16), Score(-100, -19), Score(-22, -35), Score(-68, -16), Score(60, -41), Score(32, -69),
        Score(-20, -29), Score(-25, -11), Score(-42, 5), Score(-101, 9), Score(-78, 17), Score(-26, 2), Score(35, -17), Score(26, -36),
        Score(-19, -27), Score(-13, -2), Score(-24, 12), Score(-60, 18), Score(-68, 33), Score(-30, 16), Score(-21, 8), Score(-34, -17),
        Score(-49, -15), Score(-1, -2), Score(-23, 27), Score(-44, 36), Score(-36, 32), Score(-48, 29), Score(-45, 18), Score(-49, -12),
        Score(-14, -8), Score(-21, 8), Score(-8, 20), Score(-25, 20), Score(-26, 35), Score(-16, 46), Score(-5, 41), Score(-34, 18),
        Score(-10, 8), Score(24, 23), Score(7, 8), Score(-13, 29), Score(-19, 28), Score(14, 46), Score(23, 41), Score(-18, 14),
        Score(29, -15), Score(2, 20), Score(-20, 19), Score(-3, 29), Score(-3, 36), Score(2, 43), Score(-34, 44), Score(-27, 19),
        Score(-66, -77), Score(23, -30), Score(16, -18), Score(-14, -15), Score(-55, -7), Score(-33, 21), Score(2, 9), Score(11, -25),
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
