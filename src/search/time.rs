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

use super::{CompressedDepth, Limits, Worker};

impl Limits {
    /// The maximum number of effective moves to go until the next time
    /// control.
    ///
    /// In other words, the time manager treats the moves until the next time
    /// control as `moves_to_go.min(MAX_MOVES_TO_GO)`.
    const MAX_MOVES_TO_GO: CompressedDepth = CompressedDepth(20);
}

impl Worker<'_> {
    /// Calculates the maximum window of time that should be used for the next
    /// iterative deepening loop.
    pub(super) fn calculate_time_window(&mut self) {
        if let Limits::Timed {
            ref mut time,
            inc,
            moves_to_go,
        } = self.limits
        {
            // prioritise a low number of moves to go, but if it's sudden death
            // (let's say), we set a maximum on the apparent moves to go, in order
            // to avoid allocating too little time
            let moves_to_go = moves_to_go.min(Limits::MAX_MOVES_TO_GO);
            let total_overhead = self.start.elapsed() + self.move_overhead;
            let total_time = *time;
            // if an iterative deepening loop takes way longer than expected,
            // make sure we don't run out of time
            *time = total_time.saturating_sub(total_overhead);

            self.allocated = (total_time / moves_to_go.0.into() + inc)
                // don't allocate more time than we actually have
                .min(total_time)
                .saturating_sub(total_overhead);
        }
    }
}
