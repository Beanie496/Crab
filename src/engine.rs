use crate::board::Board;

/// For perft, as it's counting leaf nodes, not searching.
mod perft;
/// For search-related code.
mod search;

/// Master object that contains all the other major objects.
#[non_exhaustive]
#[derive(Clone)]
pub struct Engine {
    /// The internal board.
    ///
    /// See [`Board`].
    pub board: Board,
}

impl Engine {
    /// Creates a new [`Engine`] with each member struct initialised to their
    /// default values.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            board: Board::new(),
        }
    }

    /// Sets `self.board` to the given FEN and moves, as given by the
    /// `position` UCI command. Unexpected/incorrect tokens will be ignored.
    #[inline]
    pub fn set_position(&mut self, position: &str, moves: &str) {
        self.board.set_pos_to_fen(position);
        self.board.play_moves(moves);
    }
}
