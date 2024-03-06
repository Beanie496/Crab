use std::{
    io,
    process::exit,
    str::Split,
    thread::{spawn, JoinHandle},
};

use crate::{board::find_magics, defs::PieceType, engine::Engine};

/// Used to store state for [`Self::main_loop()`].
pub struct Uci {
    /// The engine.
    engine: Engine,
    /// The handle to the thread that prints the information from the search.
    search_handle: Option<JoinHandle<()>>,
}

/// The starting position as a FEN string.
const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

impl Uci {
    /// Creates a new instance of [`Uci`](Self).
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
            search_handle: None,
        }
    }

    /// Repeatedly waits for a command and executes it according to the UCI
    /// protocol.
    ///
    /// # Panics
    ///
    /// Panics if [`read_line()`](`std::io::BufRead::read_line`) returns an
    /// [`Err`].
    #[inline]
    pub fn main_loop(&mut self) -> ! {
        let mut input = String::new();
        loop {
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read from stdin");
            self.handle_input_line(&input);
            input.clear();
        }
    }

    /// Starts the search, given the rest of the tokens after `go`.
    fn go(&mut self, line: &mut Split<'_, char>) {
        let mut depth = None;

        while let Some(token) = line.next() {
            // just depth for now
            #[allow(clippy::single_match)]
            match token {
                "depth" => {
                    if let Some(result) = line.next() {
                        if let Ok(d) = result.parse::<u8>() {
                            if d != 0 {
                                depth = Some(d);
                            }
                        }
                    }
                }
                _ => (),
            }
        }

        let info_rx = self.engine.start_search(depth);
        self.search_handle = Some(spawn(move || {
            for result in info_rx {
                println!("{result}");
            }
        }));
    }

    /// Given an iterator over the remaining space-delimited tokens of a `position`
    /// command, removes all empty strings and concatenate the remaining tokens
    /// into a [`String`] for the FEN and moves each with a space between each
    /// token.
    fn handle_position(&mut self, line: &mut Split<'_, char>) {
        let fen = match line.next() {
            Some("startpos") => {
                // ensure the next token is "moves" if there is one
                if let Some(token) = line.next() {
                    if token != "moves" {
                        return;
                    }
                }
                STARTPOS.to_string()
            }
            Some("fen") => {
                let mut fen = String::new();
                line.take_while(|token| *token != "moves")
                    .filter(|token| !token.is_empty())
                    // I COULD use `map()` then `collect()` but that's an unnecessary heap
                    // allocation for each token
                    .for_each(|token| {
                        fen.push_str(token);
                        fen.push(' ');
                    });
                // remove the trailing space
                fen.pop();
                fen
            }
            _ => return,
        };

        let mut moves = String::new();
        line.filter(|token| !token.is_empty()).for_each(|token| {
            moves.push_str(token);
            moves.push(' ');
        });
        // remove the trailing space
        moves.pop();

        self.engine.set_position(&fen, &moves);
    }

    /// Stops the search.
    fn stop(&mut self) {
        self.engine.stop_search();
        // there's nothing wrong with calling this function even if there's no
        // search going on
        if let Some(handle) = self.search_handle.take() {
            #[allow(clippy::unwrap_used)]
            handle.join().unwrap();
        }
    }

    /// Dissects `line` according to the UCI protocol.
    fn handle_input_line(&mut self, line: &str) {
        let mut line = line.trim().split(' ');

        // handle each UCI option
        if let Some(command) = line.next() {
            if command.is_empty() {
                return;
            }
            #[allow(clippy::match_same_arms)]
            match command {
                // Ignored commands
                "debug" | "ponderhit" => {
                    /* "debug": Sets debug to "on" or "off". Default "off". */
                    /* "ponderhit": The user has played the expected move. */
                }
                "go" => {
                    /* Start calculating from the current position,
                     * as specified by the "position" command.
                     * The next element should be one of the following:
                     * - searchmoves: restrict search to one of the specified moves
                     * - ponder: start searching in pondering mode.  Don't
                     *   implement this.
                     * - wtime: White has x ms left
                     * - btime: Black has x ms left
                     * - winc: White has x ms inc
                     * - binc: Black has x ms inc
                     * - movestogo: x moves until next tc, otherwise sudden death
                     * - depth: search x plies only
                     * - nodes: search x nodes only
                     * - mate: search for mate in x
                     * - movetime: search for exactly x ms
                     * - infinite: search until "stop" command received. Do not
                     * exit search otherwise.
                     */
                    self.go(&mut line);
                }
                "isready" => {
                    /* Immediately print "readyok" */
                    println!("readyok");
                }
                "position" => {
                    /* Next element should be "fen" or "startpos".  If the next
                     * element is "fen", a FEN string should be given (spanning
                     * multiple elements).  The element after that should be
                     * "moves", followed by a series of moves, one per element.
                     * The moves should look like, for example, "e2e4".
                     */
                    self.handle_position(&mut line);
                }
                "setoption" => {
                    /* Next element of line_iter should be "name".  Element after
                     * "name" should be one of the options specified from "uci"
                     * command.
                     */
                }
                "stop" => {
                    /* Stop calculating immediately. */
                    self.stop();
                }
                "uci" => {
                    /* Print ID, all options and "uciok" */
                    println!("uciok");
                }
                "ucinewgame" => { /* What it sounds like. Set pos to start pos, etc. */ }
                "q" | "quit" => {
                    /* Quit as soon as possible */
                    self.stop();
                    exit(0);
                }

                /* non-standard commands */
                /* "f" - find magic numbers for each square for bishop and rook */
                "f" => {
                    find_magics::<{ PieceType::BISHOP.0 }>();
                    find_magics::<{ PieceType::ROOK.0 }>();
                }
                /* "p" - prints current position */
                "p" => {
                    self.engine.pretty_print_board();
                }
                /* "perft n", where n is a number - run perft to depth n */
                "perft" => {
                    if let Some(depth) = line.next() {
                        match depth.parse::<u8>() {
                            Ok(result) => _ = self.engine.perft::<true, true>(result),
                            Err(result) => println!("{result}; must give 0-255"),
                        }
                    }
                }

                other => {
                    println!("Unrecognised option \"{other}\".");
                }
            }
        } else {
            unreachable!("Each line should have at least 1 iterable element.");
        }
    }
}
