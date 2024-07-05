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

/// The difference between the root or leaf node (for height or depth
/// respectively) and the current node.
type Depth = u8;
/// A table of base late move reductions.
///
/// Indexed by the depth then number of legal moves.
type BaseReductions = [[Depth; 128]; 64];

fn main() -> io::Result<()> {
    create_base_reductions()?;

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
            *depth_entry = (0.2 + ln_depth * ln_move_idx / 2.0) as Depth;
        }
    }

    // SAFETY: there are no invalid bit patterns for a `u8`
    let reductions_bytes = unsafe { transmute::<BaseReductions, [u8; SIZE]>(base_reductions) };
    if !Path::new("binaries").exists() {
        fs::create_dir("binaries")?;
    }
    fs::write("binaries/base_reductions.bin", reductions_bytes)
}
