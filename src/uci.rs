use std::{
    io,
    process::exit,
    str::{FromStr, Split},
    time::Duration,
};

use crate::{board::find_magics, defs::PieceType, engine::Engine};

/// Used to store state for [`Self::main_loop()`].
pub struct Uci {
    /// The engine.
    engine: Engine,
}

/// The limits of a search: how much time is allocated, etc.
#[derive(Default)]
pub struct Limits {
    /// White's time left.
    pub wtime: Option<Duration>,
    /// Black's time left.
    pub btime: Option<Duration>,
    /// White's increment.
    pub winc: Option<Duration>,
    /// Black's increment.
    pub binc: Option<Duration>,
    /// Moves until the next time control, otherwise sudden death.
    pub movestogo: Option<u8>,
    /// Maximum search depth.
    pub depth: Option<u8>,
    /// Maximum node count.
    pub nodes: Option<u64>,
    /// Exact thinking time.
    pub movetime: Option<Duration>,
}

/// The starting position as a FEN string.
const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

impl Uci {
    /// Creates a new instance of [`Uci`](Self).
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
        }
    }

    /// Repeatedly waits for a command and executes it according to the UCI
    /// protocol.
    ///
    /// # Panics
    ///
    /// Panics if [`read_line()`](`std::io::BufRead::read_line`) returns an
    /// [`Err`].
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
        let mut limits = Limits::default();

        while let Some(token) = line.next() {
            match token {
                "wtime" => limits.wtime = parse_next_num(line).map(Duration::from_millis),
                "btime" => limits.btime = parse_next_num(line).map(Duration::from_millis),
                "winc" => limits.winc = parse_next_num(line).map(Duration::from_millis),
                "binc" => limits.binc = parse_next_num(line).map(Duration::from_millis),
                "movestogo" => limits.movestogo = parse_next_num(line),
                "depth" => limits.depth = parse_next_num(line),
                "nodes" => limits.nodes = parse_next_num(line),
                "movetime" => limits.movetime = parse_next_num(line).map(Duration::from_millis),
                // if depth is specified and then `infinite` is give, the
                // latter should override the former
                "infinite" => limits.depth = None,
                _ => (),
            }
        }

        self.engine.start_search(limits);
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
                     * - searchmoves: restrict search to the specified moves
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
                    self.engine.stop_search();
                }
                "uci" => {
                    /* Print ID, all options and "uciok" */
                    println!("uciok");
                }
                "ucinewgame" => { /* What it sounds like. Set pos to start pos, etc. */ }
                "q" | "quit" => {
                    /* Quit as soon as possible */
                    self.engine.stop_search();
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

/// Parses the next unsigned integer.
///
/// If the next token is a valid number, is not 0, and fits within `T`, it
/// returns `Some(T)`; otherwise, it returns `None`.
fn parse_next_num<T: TryFrom<u128> + FromStr>(line: &mut Split<'_, char>) -> Option<T> {
    if let Some(result) = line.next() {
        if let Ok(result) = result.parse::<u128>() {
            if result != 0 {
                return result.try_into().ok();
            }
        }
    }
    None
}
