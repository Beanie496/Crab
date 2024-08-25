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

//! Creates all static lookup tables and places them in `binaries/`.

use std::{
    fs, io,
    mem::{size_of, transmute},
    path::Path,
};

/// The difference between the leaf node and the current node.
type Depth = i16;
/// A table of base late move reductions.
///
/// Indexed by the depth then number of legal moves.
type BaseReductions = [[Depth; 128]; 64];
/// A table of rays between two squares;
type RaysBetween = [[u64; 64]; 64];

fn main() -> io::Result<()> {
    if !Path::new("binaries").exists() {
        fs::create_dir("binaries")?;
    }
    create_base_reductions()?;
    create_rays_between()?;

    println!("cargo::rerun-if-changed=build.rs");

    Ok(())
}

/// Creates the file `binaries/base_reductions.bin`.
///
/// The file has structure [`BaseReductions`].
#[allow(clippy::missing_docs_in_private_items, clippy::items_after_statements)]
fn create_base_reductions() -> io::Result<()> {
    let mut base_reductions: BaseReductions = [[0; 128]; 64];
    const SIZE: usize = size_of::<BaseReductions>();

    for (depth, move_table) in base_reductions.iter_mut().enumerate() {
        for (move_idx, depth_entry) in move_table.iter_mut().enumerate() {
            let ln_depth = f32::ln(depth as f32);
            let ln_move_idx = f32::ln(move_idx as f32);
            *depth_entry = (ln_depth * ln_move_idx / 2.0) as Depth;
        }
    }

    // SAFETY: there are no invalid bit patterns for a `u8`
    let reductions_bytes = unsafe { transmute::<BaseReductions, [u8; SIZE]>(base_reductions) };
    fs::write("binaries/base_reductions.bin", reductions_bytes)
}

/// Creates a table of bitboard rays between two squares (exclusive).
///
/// If the squares are not orthogonal or diagonal to each other, the bitboard
/// will be empty.
#[allow(clippy::missing_docs_in_private_items, clippy::items_after_statements)]
fn create_rays_between() -> io::Result<()> {
    let mut rays_between: RaysBetween = [[0; 64]; 64];
    const SIZE: usize = size_of::<RaysBetween>();

    for start in 0..64 {
        for end in (start + 1)..64 {
            if let Some(direction) = direction_of_dest(start, end) {
                let ray = flood_fill(start, end, direction);
                rays_between[usize::from(start)][usize::from(end)] = ray;
                rays_between[usize::from(end)][usize::from(start)] = ray;
            }
        }
    }

    // SAFETY: there are no invalid bit patterns for a `u8`
    let rays_bytes = unsafe { transmute::<RaysBetween, [u8; SIZE]>(rays_between) };
    fs::write("binaries/rays_between.bin", rays_bytes)
}

/// Calculates which direction `dest` is from `square` as an offset.
///
/// If `dest` is not orthogonal or diagonal from `square`, it will return
/// `None`.
const fn direction_of_dest(square: u8, dest: u8) -> Option<u8> {
    // north: difference is divisible by 8
    if (dest - square) % 8 == 0 {
        return Some(8);
    }

    // east: both on same rank
    if square >> 3 == dest >> 3 {
        return Some(1);
    }

    // north-west
    let mut current = square;
    loop {
        if is_illegal_step(current, current + 7) {
            break;
        }
        current += 7;
        if current == dest {
            return Some(7);
        }
    }

    // north-east
    let mut current = square;
    loop {
        if is_illegal_step(current, current + 9) {
            break;
        }
        current += 9;
        if current == dest {
            return Some(9);
        }
    }

    None
}

/// Returns a ray in the given direction from `start` until it reaches `end` or
/// a side of the board.
const fn flood_fill(mut start: u8, end: u8, direction: u8) -> u64 {
    let mut ret = 0;

    loop {
        let next = start + direction;
        if is_illegal_step(start, next) || next == end {
            break;
        }
        start = next;
        ret |= 1 << start;
    }

    ret
}

/// Checks if `step` is a valid king move from `current`.
const fn is_illegal_step(current: u8, step: u8) -> bool {
    // if step is outside the board or has wrapped round the side
    step >= 64 || (step & 7).abs_diff(current & 7) > 1
}
