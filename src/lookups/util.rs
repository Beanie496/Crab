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

use super::magic::MAX_BLOCKERS;
use crate::{
    bitboard::Bitboard,
    defs::{File, PieceType, Square},
};

/// Generates all combinations of attacks from `square` and puts them in
/// `attacks`.
///
/// It starts with a full blocker board that goes from the square to the edge
/// exclusive and uses the Carry-Rippler trick to generate each subsequent
/// attack.
pub fn gen_all_sliding_attacks<const PIECE: u8>(
    square: Square,
    attacks: &mut [Bitboard; MAX_BLOCKERS],
) {
    let edges = Bitboard::edges_without(square);
    let mask = sliding_attacks::<PIECE>(square, Bitboard::empty()) & !edges;

    let mut blockers = mask;
    let mut first_empty = 0;
    while !blockers.is_empty() {
        attacks[first_empty] = sliding_attacks::<PIECE>(square, blockers);
        first_empty += 1;
        blockers = Bitboard(blockers.wrapping_sub(1)) & mask;
    }
    attacks[first_empty] = sliding_attacks::<PIECE>(square, Bitboard::empty());
}

/// Generates the attack set for `PIECE` on `square` up to and including the
/// given blockers and/or the edge.
///
/// Will panic if `PIECE` is not the piece type of a bishop or rook.
#[allow(clippy::similar_names)]
pub const fn sliding_attacks<const PIECE: u8>(square: Square, blockers: Bitboard) -> Bitboard {
    let not_a_file = 0xfefe_fefe_fefe_fefe;
    let not_h_file = 0x7f7f_7f7f_7f7f_7f7f;
    let square = square.0;
    let square_bb = bitboard_from_square(square);
    let blockers = blockers.0;

    let mut attacks = 0x0;
    let mut ray = square_bb;
    let mut free = !blockers;

    // Kogge-stone algorithm. This code is only ever ran once at compilation
    // time, so I'm optimising here for an interpreter of MIR. This results in
    // ugly code just to get half-decent compilation times.
    if PIECE == PieceType::BISHOP.0 {
        // north-east
        free &= not_a_file;
        ray |= free & (ray << 9);
        free &= free << 9;
        ray |= free & (ray << 18);
        free &= free << 18;
        ray |= free & (ray << 36);
        ray <<= 9;
        attacks |= ray & not_a_file;

        // south-east
        ray = square_bb;
        free = !blockers & not_a_file;
        ray |= free & (ray >> 7);
        free &= free >> 7;
        ray |= free & (ray >> 14);
        free &= free >> 14;
        ray |= free & (ray >> 28);
        ray >>= 7;
        attacks |= ray & not_a_file;

        // south-west
        ray = square_bb;
        free = !blockers & not_h_file;
        ray |= free & (ray >> 9);
        free &= free >> 9;
        ray |= free & (ray >> 18);
        free &= free >> 18;
        ray |= free & (ray >> 36);
        ray >>= 9;
        attacks |= ray & not_h_file;

        // north-west
        ray = square_bb;
        free = !blockers & not_h_file;
        ray |= free & (ray << 7);
        free &= free << 7;
        ray |= free & (ray << 14);
        free &= free << 14;
        ray |= free & (ray << 28);
        ray <<= 7;
    } else if PIECE == PieceType::ROOK.0 {
        // north
        ray |= free & (ray << 8);
        free &= free << 8;
        ray |= free & (ray << 16);
        free &= free << 16;
        ray |= free & (ray << 32);
        ray <<= 8;
        attacks |= ray;

        // east
        ray = square_bb;
        free = !blockers & not_a_file;
        ray |= free & (ray << 1);
        free &= free << 1;
        ray |= free & (ray << 2);
        free &= free << 2;
        ray |= free & (ray << 4);
        ray <<= 1;
        attacks |= ray & not_a_file;

        // south
        ray = square_bb;
        free = !blockers;
        ray |= free & (ray >> 8);
        free &= free >> 8;
        ray |= free & (ray >> 16);
        free &= free >> 16;
        ray |= free & (ray >> 32);
        ray >>= 8;
        attacks |= ray;

        // west
        ray = square_bb;
        free = !blockers & not_h_file;
        ray |= free & (ray >> 1);
        free &= free >> 1;
        ray |= free & (ray >> 2);
        free &= free >> 2;
        ray |= free & (ray >> 4);
        ray >>= 1;
    } else {
        panic!("Sliding piece type not a bishop or rook");
    };
    attacks |= ray & not_h_file;

    Bitboard(attacks)
}

/// Shifts a bitboard one square north without wrapping.
pub const fn north(num: u64) -> u64 {
    num << 8
}

/// Shifts a bitboard one square east without wrapping.
pub const fn east(num: u64) -> u64 {
    (num << 1) & !Bitboard::file_bb(File::FILE1).0
}

/// Shifts a bitboard one square south without wrapping.
pub const fn south(num: u64) -> u64 {
    num >> 8
}

/// Shifts a bitboard one square west without wrapping.
pub const fn west(num: u64) -> u64 {
    (num >> 1) & !Bitboard::file_bb(File::FILE8).0
}

/// Converts a square to a bitboard with the relevant bit set.
pub const fn bitboard_from_square(square: u8) -> u64 {
    1 << square
}
