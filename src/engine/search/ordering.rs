use super::SearchInfo;
use crate::{
    board::{Move, Moves, MAX_LEGAL_MOVES},
    evaluation::Eval,
    util::Stack,
};

/// A scored move.
#[derive(Copy, Clone)]
#[allow(clippy::missing_docs_in_private_items)]
struct ScoredMove {
    mv: Move,
    /// A score.
    ///
    /// This is currently +INF for PV moves and 0 otherwise, but is subject to
    /// change.
    score: Eval,
}

/// A stack of scored moves.
#[allow(clippy::missing_docs_in_private_items)]
pub struct ScoredMoves {
    moves: Stack<ScoredMove, MAX_LEGAL_MOVES>,
}

impl Iterator for ScoredMoves {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop().map(|scored_move| scored_move.mv)
    }
}

impl ScoredMove {
    /// Creates a new [`ScoredMove`] from a [`Move`] and a score ([`Eval`]).
    const fn new(mv: Move, score: Eval) -> Self {
        Self { mv, score }
    }
}

impl ScoredMoves {
    /// Scores the moves in `moves` given the information in `search_info` and
    /// the current height.
    pub fn score_moves(search_info: &SearchInfo, moves: &mut Moves, height: u8) -> Self {
        let mut scored_moves = Self::new();

        for mv in moves {
            // always search the PV first
            if search_info.history.get(usize::from(height)) == mv {
                scored_moves.push(ScoredMove::new(mv, Eval::MAX));
            } else {
                scored_moves.push(ScoredMove::new(mv, 0));
            }
        }

        scored_moves
    }

    /// Sorts the scored moves in `self` based on their score.
    pub fn sort(&mut self) {
        self.moves.get_mut_slice().sort_by(|mv1, mv2| {
            // SAFETY: the slice we're sorting contains only initialised
            // elements
            unsafe { mv1.assume_init_read() }
                .score
                // SAFETY: ditto
                .cmp(&unsafe { mv2.assume_init_read() }.score)
        });
    }

}

impl ScoredMoves {
    /// Creates a new, uninitialised stack of [`ScoredMove`]s.
    const fn new() -> Self {
        Self {
            moves: Stack::new(),
        }
    }

    /// Pushes a [`ScoredMove`] onto the stack.
    fn push(&mut self, mv: ScoredMove) {
        self.moves.push(mv);
    }

    /// Pops a [`ScoredMove`] off the stack.
    fn pop(&mut self) -> Option<ScoredMove> {
        self.moves.pop()
    }
}
