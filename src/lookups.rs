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

use crate::{search::Depth, util::get_unchecked};

/// A table of base late move reductions.
///
/// Indexed by the depth then number of legal moves.
type BaseReductions = [[Depth; 128]; 64];

/// A table of base late move reductions.
///
/// Generated by the build script.
static BASE_REDUCTIONS: BaseReductions =
    unsafe { transmute(*include_bytes!("../binaries/base_reductions.bin")) };

/// Finds the base late move reduction for the given number of moves and the
/// given depth.
pub fn base_reductions(depth: Depth, total_moves: u8) -> Depth {
    let move_table = get_unchecked(&BASE_REDUCTIONS, usize::from(depth).min(63));
    *get_unchecked(move_table, usize::from(total_moves).min(127))
}
