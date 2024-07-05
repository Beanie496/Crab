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

use std::mem::size_of;

use super::{
    alpha_beta_search::search, print_report, Depth, Eval, Pv, RootNode, SearchReferences,
    SearchStatus,
};
use crate::{board::Board, evaluation::INF_EVAL};

/// An aspiration window: a set of bounds used for [`search()`] and updated if
/// the returned score fails high or low.
pub struct AspirationWindow {
    /// The lower bound.
    alpha: Eval,
    /// The upper bound.
    beta: Eval,
    /// How much higher or lower the bounds should be set above the returned
    /// score (if the score is a bound).
    margin: Eval,
}

impl AspirationWindow {
    /// The highest a margin can be before it increases.
    const MARGIN_LIMIT: Eval = 700;
}

impl AspirationWindow {
    /// Returns a new [`AspirationWindow`] with infinite bounds and no margin.
    pub const fn new() -> Self {
        Self {
            alpha: -INF_EVAL,
            beta: INF_EVAL,
            margin: 0,
        }
    }

    /// Adjusts the aspiration window around the score.
    pub fn adjust_around(&mut self, score: Eval, depth: Depth) {
        // let very small depths have a higher margin and high scores have a
        // much higher margin
        // `widening_mul()` is still nightly unfortunately
        assert!(
            size_of::<Eval>() * 2 == size_of::<i32>(),
            "an Eval must be half the size of an i32 or the following calculation could overflow"
        );
        let unbounded_margin =
            50 / i32::from(depth).min(5) + i32::from(score) * i32::from(score) / 3_000;

        self.margin = Eval::try_from(unbounded_margin).unwrap_or(INF_EVAL);
        self.alpha = score.saturating_sub(self.margin);
        self.beta = score.saturating_add(self.margin);
    }

    /// Returns the lower bound.
    const fn alpha(&self) -> Eval {
        self.alpha
    }

    /// Returns the upper bound.
    const fn beta(&self) -> Eval {
        self.beta
    }

    /// Increases the upper bound to above the given score.
    fn widen_up(&mut self, score: Eval) {
        if self.margin > Self::MARGIN_LIMIT {
            self.beta = INF_EVAL;
            return;
        }
        self.margin *= 2;

        self.beta = score.saturating_add(self.margin);
    }

    /// Checks if the upper bound can be increased.
    const fn can_widen_up(&self) -> bool {
        self.beta() < INF_EVAL
    }

    /// Lowers the lower bound to below the given score.
    fn widen_down(&mut self, score: Eval) {
        if self.margin > Self::MARGIN_LIMIT {
            self.alpha = -INF_EVAL;
            return;
        }
        self.margin *= 2;

        self.beta = (self.alpha + self.beta) / 2;
        // same as `score.saturating_sub(self.margin)`, but saturates at
        // `-Eval::MAX` instead of `Eval::MIN`
        self.alpha = -(-score).saturating_add(self.margin);
    }

    /// Checks if the lower bound can be lowered.
    const fn can_widen_down(&self) -> bool {
        self.alpha() > -INF_EVAL
    }
}

/// Runs the aspiration loop on the given board.
///
/// See <https://www.chessprogramming.org/Aspiration_Windows>.
/// `pv` does not need to be empty.
pub fn aspiration_loop(
    search_refs: &mut SearchReferences<'_>,
    pv: &mut Pv,
    board: &Board,
    asp_window: &mut AspirationWindow,
    depth: Depth,
) -> Eval {
    loop {
        let score = search::<RootNode>(
            search_refs,
            pv,
            board,
            asp_window.alpha(),
            asp_window.beta(),
            depth,
            0,
        );

        let time = search_refs.start.elapsed();
        print_report(search_refs, time, score, pv, depth);

        if search_refs.check_status() != SearchStatus::Continue {
            break score;
        }

        // fail-low
        if score <= asp_window.alpha() && asp_window.can_widen_down() {
            asp_window.widen_down(score);
            continue;
        }

        // fail-high
        if score >= asp_window.beta() && asp_window.can_widen_up() {
            asp_window.widen_up(score);
            continue;
        }

        // exact score
        break score;
    }
}
