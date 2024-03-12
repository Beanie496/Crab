use std::time::Duration;

use super::{Limits, SearchInfo};

impl Limits {
    /// The maximum number of effective moves to go until the next time
    /// control.
    ///
    /// In other words, the time manager treats the moves until the next time
    /// control as `moves_to_go.min(MAX_MOVES_TO_GO)`.
    const MAX_MOVES_TO_GO: u8 = 40;
}

impl SearchInfo {
    /// Calculates the maximum window of time that should be used for the next
    /// iterative deepening loop.
    ///
    /// Any kind of explanation here as to what I'm doing would immediately
    /// become outdated, so read the comments of the following code instead.
    pub fn calculate_time_window(&self) -> Duration {
        if let Limits::Timed {
            time,
            inc,
            moves_to_go,
        } = self.limits
        {
            // prioritise a low number of moves to go, but if it's sudden death
            // (let's say), we set a maximum on the apparent moves to go, in order
            // to avoid allocating too little time
            let moves_to_go = moves_to_go.min(Limits::MAX_MOVES_TO_GO);

            (time / u32::from(moves_to_go) + inc).saturating_sub(self.time_start.elapsed())
        } else {
            Duration::MAX
        }
    }
}
