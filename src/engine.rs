use crate::{
    board::{find_magics, Board, CastlingRights},
    defs::{File, Piece, Rank, Side, Square},
};

mod perft;
mod search;

/// Master object that contains all the other major objects.
pub struct Engine {
    board: Board,
}

impl Engine {
    /// Creates a new [`Engine`] with each member struct initialised to their
    /// default values.
    pub fn new() -> Self {
        Self {
            board: Board::new(),
        }
    }

    /// Wrapper for [`find_magics`].
    pub fn find_magics<const PIECE: u8>() {
        find_magics::<PIECE>();
    }
}

impl Engine {
    /// Clears the board.
    pub fn clear_board(&mut self) {
        self.board.clear_board();
    }

    /// Takes a sequence of moves and feeds them to the board. Will stop and
    /// return if any of the moves are incorrect. Not implemented yet.
    pub fn play_moves(&self, _moves: &str) {}

    /// Pretty-prints the current state of the board.
    pub fn pretty_print_board(&self) {
        self.board.pretty_print();
    }

    /// Adds the given right to the castling rights of the board.
    pub fn add_castling_right(&mut self, right: CastlingRights) {
        self.board.add_castling_right(right);
    }

    /// Sets the default castling right/s `right`.
    pub fn set_default_castling_rights(&mut self) {
        self.board.set_default_castling_rights();
    }

    /// Sets default side to move.
    pub fn set_default_side_to_move(&mut self) {
        self.board.set_default_side_to_move();
    }

    /// Sets the ep square.
    pub fn set_ep_square(&mut self, square: Square) {
        self.board.set_ep_square(square);
    }

    /// Sets the fullmove counter.
    pub fn set_fullmoves(&mut self, count: u32) {
        self.board.set_fullmoves(count);
    }

    /// Sets the halfmove counter.
    pub fn set_halfmoves(&mut self, count: u32) {
        self.board.set_halfmoves(count);
    }

    /// Sets `self.board` to the given FEN. It will check for basic errors,
    /// like the board having too many ranks, but not many more.
    pub fn set_pos_to_fen(&mut self, position: &str) {
        self.clear_board();

        let mut iter = position.split(' ');
        let board = if let Some(x) = iter.next() {
            x
        } else {
            self.set_startpos();
            return eprintln!("Need to pass a board");
        };
        let side_to_move = iter.next();
        let castling_rights = iter.next();
        let ep_square = iter.next();
        let halfmoves = iter.next();
        let fullmoves = iter.next();

        // 1. the board itself. 1 of each king isn't checked. Hey, garbage in,
        // garbage out!
        // split into 2 to check for overflow easily
        let mut rank_idx = 8u8;
        let mut file_idx = 0;
        let ranks = board.split('/');
        for rank in ranks {
            // if the board has too many ranks, this would eventually underflow
            // and panic, so wrapping sub needed
            rank_idx = rank_idx.wrapping_sub(1);
            for piece in rank.chars() {
                // if it's a number, skip over that many files
                if ('0'..='8').contains(&piece) {
                    file_idx += piece.to_digit(10).unwrap() as u8;
                } else {
                    let piece_num = Piece::from_char(piece.to_ascii_lowercase());
                    let piece_num = if let Some(pn) = piece_num {
                        pn
                    } else {
                        self.set_startpos();
                        return eprintln!("Error: \"{piece}\" is not a valid piece.");
                    };
                    // 1 if White, 0 if Black
                    let side = Side::from(piece.is_ascii_uppercase() as u8);

                    self.board.add_piece(
                        side,
                        Square::from_pos(Rank::from(rank_idx), File::from(file_idx)),
                        piece_num,
                    );

                    file_idx += 1;
                }
            }
            // if there are too few/many files in that rank, reset and return
            if file_idx != 8 {
                self.set_startpos();
                return eprintln!("Error: FEN is invalid");
            }

            file_idx = 0;
        }
        // if there are too many/few ranks in the board, reset and return
        if rank_idx != 0 {
            self.set_startpos();
            return eprintln!("Error: FEN is invalid (incorrect number of ranks)");
        }

        // 2. side to move
        if let Some(stm) = side_to_move {
            if stm == "w" {
                self.set_side_to_move(Side::WHITE);
            } else if stm == "b" {
                self.set_side_to_move(Side::BLACK);
            } else {
                self.set_startpos();
                return eprintln!("Error: Side to move \"{stm}\" is not \"w\" or \"b\"");
            }
        } else {
            // I've decided that everything apart from the board can be omitted
            // and guessed, so if there's nothing given, default to White to
            // move.
            self.set_default_side_to_move();
        }

        // 3. castling rights
        if let Some(cr) = castling_rights {
            for right in cr.chars() {
                match right {
                    'K' => self.add_castling_right(CastlingRights::CASTLE_FLAGS_K),
                    'Q' => self.add_castling_right(CastlingRights::CASTLE_FLAGS_Q),
                    'k' => self.add_castling_right(CastlingRights::CASTLE_FLAGS_k),
                    'q' => self.add_castling_right(CastlingRights::CASTLE_FLAGS_q),
                    '-' => (),
                    _ => {
                        self.set_startpos();
                        return eprintln!("Error: castling right \"{right}\" is not valid");
                    }
                }
            }
        } else {
            // KQkq if nothing is given.
            self.set_default_castling_rights();
        }

        // 4. en passant
        self.set_ep_square(if let Some(ep) = ep_square {
            if ep == "-" {
                Square::NONE
            } else if let Some(square) = Square::from_string(ep) {
                square
            } else {
                self.set_startpos();
                return eprintln!("Error: En passant square \"{ep}\" is not a valid square");
            }
        } else {
            Square::NONE
        });

        // 5. halfmoves
        self.set_halfmoves(if let Some(hm) = halfmoves {
            if let Ok(hm) = hm.parse::<u32>() {
                hm
            } else {
                self.set_startpos();
                return eprintln!("Error: Invalid number (\"hm\") given for halfmove counter");
            }
        } else {
            0
        });

        // 6. fullmoves
        self.set_fullmoves(if let Some(fm) = fullmoves {
            if let Ok(fm) = fm.parse::<u32>() {
                fm
            } else {
                return eprintln!("Error: Invalid number (\"fm\") given for fullmove counter");
            }
        } else {
            0
        });
    }

    /// Sets `self.board` to the given FEN and moves, as given by the
    /// `position` UCI command. Unexpected/incorrect tokens will be ignored.
    pub fn set_position(&mut self, position: &str, moves: &str) {
        self.set_pos_to_fen(position);
        self.play_moves(moves);
    }

    /// Sets side to move to `side`.
    pub fn set_side_to_move(&mut self, side: Side) {
        self.board.set_side_to_move(side);
    }

    /// Resets `self.board`.
    pub fn set_startpos(&mut self) {
        self.board.set_startpos();
    }
}
