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

use std::mem::transmute;

use crate::{bitboard::Bitboard, defs::Square, search::Depth, util::get_unchecked};

/// A table of base late move reductions.
///
/// Indexed by the depth then number of legal moves.
type BaseReductions = [[Depth; 128]; 64];

/// A table of base late move reductions.
///
/// Generated by the build script.
static BASE_REDUCTIONS: BaseReductions =
    unsafe { transmute(*include_bytes!("../binaries/base_reductions.bin")) };
/// A table of rays between two squares, excluding the squares themselves.
///
/// If an orthogonal or diagonal ray cannot be drawn between the two squares,
/// the bitboard will be empty.
static RAYS_BETWEEN: [[Bitboard; Square::TOTAL]; Square::TOTAL] =
    unsafe { transmute(*include_bytes!("../binaries/rays_between.bin")) };

/// Finds the base late move reduction for the given number of moves and the
/// given depth.
pub fn base_reductions(depth: Depth, total_moves: u8) -> Depth {
    let move_table = get_unchecked(&BASE_REDUCTIONS, usize::from(depth).min(63));
    *get_unchecked(move_table, usize::from(total_moves).min(127))
}

/// Finds the bitboard ray between `start` and `end`.
///
/// It will be an empty bitboard if there cannot be a ray between `start` and
/// `end`.
pub fn ray_between(start: Square, end: Square) -> Bitboard {
    let first_square = get_unchecked(&RAYS_BETWEEN, usize::from(start.0));
    *get_unchecked(first_square, usize::from(end.0))
}
