use super::Gui;

use backend::{
    board::Move,
    defs::{Piece, Side, Square},
};

impl Gui {
    /// Remove the [`Piece`] and [`Side`] on `square` in the mailboxes of
    /// `self`.
    pub fn clear_square(&mut self, square: Square) {
        self.piece_mailbox[square.to_index()] = Piece::NONE;
        self.side_mailbox[square.to_index()] = Side::NONE;
    }

    /// Add a [`Piece`] on [`Side`] to `square` to the mailboxes of `self`.
    pub fn add_piece(&mut self, square: Square, piece: Piece, side: Side) {
        self.piece_mailbox[square.to_index()] = piece;
        self.side_mailbox[square.to_index()] = side;
    }

    /// Attempts to move a piece from `start` to `end`. Returns `true` if the
    /// given move is legal; false otherwise.
    // there are many ways to do this. The method I've chosen is
    // - hold a list of legal moves in `self`
    // - when this function is called, create the `Move`
    // - check if it's in the list (linear search), returning if it isn't
    // - play it on a copy, returning if it's illegal
    // - set the current board to the copy
    // - regenerate the legal move list
    // It's not very efficient, but I doubt it takes more than a few hundred
    // nanoseconds, so who cares.
    pub fn move_piece(&mut self, start: Square, end: Square) -> bool {
        let mv = Move::new::<{ Move::NO_FLAG }>(start, end);
        if !self.legal_moves.contains(mv) {
            return false;
        }

        let mut copy = self.engine.board.clone();
        if !copy.make_move(mv) {
            return false;
        }

        self.legal_moves.clear();
        copy.generate_moves(&mut self.legal_moves);
        self.engine.board = copy;

        self.move_piece(start, end);
        self.clear_square(start);
        let piece = self.engine.board.piece_on(end);
        let side = self.engine.board.side_of(end);
        self.add_piece(end, piece, side);

        true
    }

    /// Finds the piece on `square`.
    pub fn piece_on(&self, square: Square) -> Piece {
        self.piece_mailbox[square.to_index()]
    }

    /// Finds the side of the piece on `square`. If there is no piece on
    /// `square`, it returns [`Side::NONE`].
    pub fn side_of(&self, square: Square) -> Side {
        self.side_mailbox[square.to_index()]
    }
}
