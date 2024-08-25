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

use super::{Evaluation, Score};
use crate::{
    cfor,
    defs::{Piece, PieceType, Side, Square},
};

/// A [`Score`].
///
/// Used to avoid wrapping every single number in this file inside an
/// [`Evaluation`].
#[derive(Debug)]
pub struct RawScore(pub i32, pub i32);

/// Values in centipawns for each piece.
///
/// Order: pawn, knight, bishop, rook, queen, king.
#[rustfmt::skip]
pub const BASE_PIECE_VALUES: [RawScore; PieceType::TOTAL] = [
    RawScore(77, 106), RawScore(254, 273), RawScore(278, 296), RawScore(393, 482), RawScore(955, 867), RawScore(10000, 10000),
];

/// Piece-square tables, copied verbatim from
/// <https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function>.
///
/// Order: pawn, knight, bishop, rook, queen, king.
#[rustfmt::skip]
pub const INITIAL_PIECE_SQUARE_TABLES: [[RawScore; Square::TOTAL]; PieceType::TOTAL] = [
    [
        RawScore(0, 0), RawScore(0, 0), RawScore(0, 0), RawScore(0, 0), RawScore(0, 0), RawScore(0, 0), RawScore(0, 0), RawScore(0, 0),
        RawScore(-32, 26), RawScore(-2, 20), RawScore(-32, 21), RawScore(-37, 14), RawScore(-33, 15), RawScore(18, 6), RawScore(48, -4), RawScore(-17, -10),
        RawScore(-24, 20), RawScore(-4, 18), RawScore(-11, 4), RawScore(-14, 13), RawScore(-3, 1), RawScore(-12, 3), RawScore(35, -6), RawScore(-8, -5),
        RawScore(-22, 22), RawScore(-7, 22), RawScore(-11, -3), RawScore(6, -5), RawScore(8, -11), RawScore(5, -12), RawScore(2, 2), RawScore(-26, 2),
        RawScore(-9, 50), RawScore(8, 37), RawScore(3, 16), RawScore(11, 8), RawScore(15, -3), RawScore(4, -3), RawScore(7, 13), RawScore(-22, 17),
        RawScore(14, 107), RawScore(3, 110), RawScore(54, 80), RawScore(50, 66), RawScore(66, 40), RawScore(61, 31), RawScore(-10, 57), RawScore(4, 69),
        RawScore(109, 167), RawScore(131, 170), RawScore(103, 167), RawScore(109, 136), RawScore(72, 147), RawScore(115, 116), RawScore(17, 179), RawScore(-7, 182),
        RawScore(0, 0), RawScore(0, 0), RawScore(0, 0), RawScore(0, 0), RawScore(0, 0), RawScore(0, 0), RawScore(0, 0), RawScore(0, 0),
    ],
    [
        RawScore(-103, -33), RawScore(-57, -79), RawScore(-76, -43), RawScore(-44, -14), RawScore(-47, -36), RawScore(-41, -36), RawScore(-35, -57), RawScore(-58, -74),
        RawScore(-62, -54), RawScore(-70, -21), RawScore(-35, -15), RawScore(-26, -4), RawScore(-34, -14), RawScore(1, -25), RawScore(-52, -37), RawScore(-43, -53),
        RawScore(-63, -38), RawScore(-34, -7), RawScore(-12, 3), RawScore(-16, 17), RawScore(-5, 21), RawScore(-7, 14), RawScore(3, -28), RawScore(-50, -27),
        RawScore(-45, -28), RawScore(-3, -20), RawScore(5, 28), RawScore(-15, 36), RawScore(1, 36), RawScore(3, 27), RawScore(19, 8), RawScore(-35, -28),
        RawScore(-16, -19), RawScore(0, 12), RawScore(9, 43), RawScore(37, 33), RawScore(11, 41), RawScore(69, 21), RawScore(15, 16), RawScore(5, -19),
        RawScore(-66, -29), RawScore(40, -14), RawScore(4, 24), RawScore(65, 31), RawScore(29, 13), RawScore(85, 0), RawScore(52, -27), RawScore(41, -38),
        RawScore(-61, -22), RawScore(-43, -3), RawScore(70, -27), RawScore(27, 4), RawScore(20, -1), RawScore(65, -25), RawScore(3, -17), RawScore(-10, -60),
        RawScore(-180, -54), RawScore(-85, -33), RawScore(-39, -21), RawScore(-65, -36), RawScore(46, -44), RawScore(-107, -33), RawScore(-15, -62), RawScore(-97, -99),
    ],
    [
        RawScore(-53, -38), RawScore(-12, -24), RawScore(-35, -49), RawScore(-25, -14), RawScore(-39, -24), RawScore(-38, -43), RawScore(-37, -1), RawScore(-50, -34),
        RawScore(-7, -28), RawScore(-8, -28), RawScore(3, -16), RawScore(-16, -6), RawScore(-17, 6), RawScore(28, -22), RawScore(18, -25), RawScore(-18, -49),
        RawScore(-16, -25), RawScore(0, -5), RawScore(-1, 17), RawScore(-6, 17), RawScore(-1, 30), RawScore(2, 8), RawScore(11, -20), RawScore(0, -19),
        RawScore(3, -25), RawScore(-4, 5), RawScore(1, 21), RawScore(8, 42), RawScore(10, 16), RawScore(-5, 17), RawScore(-4, -3), RawScore(-4, -8),
        RawScore(-20, -8), RawScore(3, 9), RawScore(6, 31), RawScore(44, 18), RawScore(44, 30), RawScore(17, 26), RawScore(1, 20), RawScore(3, -1),
        RawScore(-24, 10), RawScore(27, 13), RawScore(17, 17), RawScore(44, 15), RawScore(-9, -6), RawScore(28, 32), RawScore(36, 14), RawScore(5, -7),
        RawScore(-29, -13), RawScore(-2, 6), RawScore(-51, 12), RawScore(-36, -11), RawScore(20, 8), RawScore(49, -2), RawScore(-37, 9), RawScore(-63, -31),
        RawScore(-27, -15), RawScore(3, -20), RawScore(-94, -14), RawScore(-49, -9), RawScore(-32, -13), RawScore(-60, -13), RawScore(7, -21), RawScore(-10, -24),
    ],
    [
        RawScore(-29, -18), RawScore(-28, -7), RawScore(-18, 3), RawScore(-4, 4), RawScore(-1, -4), RawScore(-15, -15), RawScore(-49, -8), RawScore(-30, -47),
        RawScore(-68, -43), RawScore(-41, -23), RawScore(-44, -11), RawScore(-44, -6), RawScore(-30, -7), RawScore(-5, -13), RawScore(-19, -32), RawScore(-64, -10),
        RawScore(-51, -15), RawScore(-46, -1), RawScore(-42, -28), RawScore(-52, -17), RawScore(-17, -13), RawScore(-24, -24), RawScore(-4, -15), RawScore(-44, -30),
        RawScore(-34, -15), RawScore(-22, 8), RawScore(-18, 8), RawScore(-12, 1), RawScore(6, -11), RawScore(-34, 6), RawScore(18, -22), RawScore(-29, -17),
        RawScore(-39, 6), RawScore(-16, 6), RawScore(10, 18), RawScore(29, 8), RawScore(19, 1), RawScore(35, 9), RawScore(-4, 3), RawScore(0, -7),
        RawScore(-8, 22), RawScore(35, 4), RawScore(17, 16), RawScore(53, 13), RawScore(33, 17), RawScore(75, -5), RawScore(71, 8), RawScore(28, -2),
        RawScore(37, 18), RawScore(26, 26), RawScore(54, 31), RawScore(75, 26), RawScore(94, 12), RawScore(71, 14), RawScore(32, 20), RawScore(54, 11),
        RawScore(41, 13), RawScore(52, 8), RawScore(25, 26), RawScore(32, 29), RawScore(41, 24), RawScore(2, 12), RawScore(37, 19), RawScore(49, 9),
    ],
    [
        RawScore(-12, -47), RawScore(-40, -34), RawScore(-20, -36), RawScore(4, -43), RawScore(-29, -22), RawScore(-36, -39), RawScore(-42, -25), RawScore(-48, -44),
        RawScore(-39, -25), RawScore(-29, -29), RawScore(6, -30), RawScore(-5, -30), RawScore(1, -16), RawScore(11, -51), RawScore(-29, -48), RawScore(-13, -39),
        RawScore(-26, -28), RawScore(-4, -34), RawScore(-18, 13), RawScore(-11, 4), RawScore(-18, 0), RawScore(-5, 12), RawScore(14, -7), RawScore(-24, 2),
        RawScore(-3, -34), RawScore(-17, 23), RawScore(-11, 19), RawScore(-6, 62), RawScore(-6, 30), RawScore(-22, 42), RawScore(3, 34), RawScore(-6, 19),
        RawScore(-32, 0), RawScore(-14, 29), RawScore(-9, 26), RawScore(-9, 54), RawScore(11, 60), RawScore(23, 41), RawScore(-6, 60), RawScore(1, 41),
        RawScore(-30, -24), RawScore(-12, 3), RawScore(12, 15), RawScore(20, 52), RawScore(44, 41), RawScore(68, 40), RawScore(71, 35), RawScore(55, 0),
        RawScore(-23, -21), RawScore(-30, 27), RawScore(-7, 41), RawScore(5, 45), RawScore(-14, 61), RawScore(73, 39), RawScore(24, 25), RawScore(52, 2),
        RawScore(-21, -3), RawScore(17, 24), RawScore(28, 18), RawScore(19, 36), RawScore(69, 45), RawScore(44, 21), RawScore(47, 6), RawScore(47, 16),
    ],
    [
        RawScore(-63, -72), RawScore(19, -33), RawScore(-16, -16), RawScore(-100, -19), RawScore(-22, -35), RawScore(-68, -16), RawScore(60, -41), RawScore(32, -69),
        RawScore(-20, -29), RawScore(-25, -11), RawScore(-42, 5), RawScore(-101, 9), RawScore(-78, 17), RawScore(-26, 2), RawScore(35, -17), RawScore(26, -36),
        RawScore(-19, -27), RawScore(-13, -2), RawScore(-24, 12), RawScore(-60, 18), RawScore(-68, 33), RawScore(-30, 16), RawScore(-21, 8), RawScore(-34, -17),
        RawScore(-49, -15), RawScore(-1, -2), RawScore(-23, 27), RawScore(-44, 36), RawScore(-36, 32), RawScore(-48, 29), RawScore(-45, 18), RawScore(-49, -12),
        RawScore(-14, -8), RawScore(-21, 8), RawScore(-8, 20), RawScore(-25, 20), RawScore(-26, 35), RawScore(-16, 46), RawScore(-5, 41), RawScore(-34, 18),
        RawScore(-10, 8), RawScore(24, 23), RawScore(7, 8), RawScore(-13, 29), RawScore(-19, 28), RawScore(14, 46), RawScore(23, 41), RawScore(-18, 14),
        RawScore(29, -15), RawScore(2, 20), RawScore(-20, 19), RawScore(-3, 29), RawScore(-3, 36), RawScore(2, 43), RawScore(-34, 44), RawScore(-27, 19),
        RawScore(-66, -77), RawScore(23, -30), RawScore(16, -18), RawScore(-14, -15), RawScore(-55, -7), RawScore(-33, 21), RawScore(2, 9), RawScore(11, -25),
    ],
];

/// Creates the initial piece-square table for White and Black, with an extra
/// table of 0's at the end to allow [`Piece::NONE`] to index into it.
#[allow(clippy::similar_names)]
pub const fn create_piece_square_tables() -> [[Score; Square::TOTAL]; Piece::TOTAL + 1] {
    let mut psqt = [[Score(Evaluation(0), Evaluation(0)); Square::TOTAL]; Piece::TOTAL + 1];
    cfor!(let mut piece = 0; piece < PieceType::TOTAL; piece += 1; {
        let w_piece = Piece::from_piecetype(PieceType(piece as u8), Side::WHITE);
        let b_piece = Piece::from_piecetype(PieceType(piece as u8), Side::BLACK);
        cfor!(let mut square = 0; square < Square::TOTAL; square += 1; {
            let flipped_square = square ^ 56;
            let RawScore(mut mg, mut eg) = BASE_PIECE_VALUES[piece];
            let RawScore(mg_psq, eg_psq) = INITIAL_PIECE_SQUARE_TABLES[piece][square];
            mg += mg_psq;
            eg += eg_psq;
            psqt[w_piece.to_index()][square] = Score(Evaluation(mg), Evaluation(eg));
            psqt[b_piece.to_index()][flipped_square] = Score(Evaluation(-mg), Evaluation(-eg));
        });
    });
    psqt
}
