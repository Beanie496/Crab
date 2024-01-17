use super::Gui;

use backend::defs::{Piece, Side, Square};

impl Gui {
    pub fn piece_on(&self, square: Square) -> Piece {
        self.piece_mailbox[square.to_index()]
    }

    pub fn side_of(&self, square: Square) -> Side {
        self.side_mailbox[square.to_index()]
    }
}
