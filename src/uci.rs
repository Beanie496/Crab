use std::{io, process::exit, str::Split};

use crate::engine::Engine;

pub struct Uci;

impl Uci {
    /// Repeatedly waits for a command and executes it according to the UCI
    /// protocol. It is not yet concurrent.
    pub fn main_loop() {
        let mut engine = Engine::new();
        let mut input = String::new();
        loop {
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read from stdin");
            Self::handle_input_line(&input, &mut engine);
            input.clear();
        }
    }
}

impl Uci {
    /// Given an iterator over a FEN string, convert it to a position and
    /// modify the board state of `engine` to reflect that.
    fn handle_fen_string(_line: &Split<'_, char>, _engine: &mut Engine) {
        todo!()
    }

    /// Dissects `line` according to the UCI protocol.
    fn handle_input_line(line: &str, engine: &mut Engine) {
        let mut line = line.trim().split(' ');

        // handle each UCI option
        if let Some(command) = line.next() {
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
                     * - searchmoves: restrict search to one of the
                     *   specified moves
                     * - ponder: start searching in pondering mode.
                     *   Don't implement this.
                     * - wtime: White has x ms left
                     * - btime: Black has x ms left
                     * - winc: White has x ms inc
                     * - binc: Black has x ms inc
                     * - movestogo: x moves until next tc, otherwise
                     *   sudden death
                     * - depth: search x plies only
                     * - nodes: search x nodes only
                     * - mate: search for mate in x
                     * - movetime: search for exactly x ms
                     * - infinite: search until "stop" command
                     *   received. Do not exit search otherwise.
                     */
                    // just depth for now, as making this easily extensible
                    // would take a little time
                    if let Some(string) = line.next() {
                        if string != "depth" {
                            return;
                        }
                        if let Some(depth) = line.next() {
                            match depth.parse::<u8>() {
                                Ok(result) => engine.search(Some(result)),
                                Err(result) => println!("{}; must give 0-255", result),
                            }
                        }
                    } else {
                        engine.search(None);
                    }
                }
                "isready" => {
                    /* Immediately print "readyok" */
                    println!("readyok");
                }
                "position" => {
                    /* Next element should be "fen" or "startpos".
                     * If the next element is "fen", a FEN string
                     * should be given (spanning multiple elements).
                     * The element after that should be "moves",
                     * followed by a series of moves, one per element.
                     * The moves should look like, for example,
                     * "e2e4".
                     */
                    // add FEN only for now - not moves
                    if let Some(string) = line.next() {
                        match string {
                            "fen" => {
                                Self::handle_fen_string(&line, engine);
                            }
                            "startpos" => engine.set_startpos(),
                            _ => (),
                        }
                    }
                }
                "setoption" => {
                    /* Next element of line_iter should be "name".
                     * Element after "name" should be one of the
                     * options specified from "uci" command.
                     */
                }
                "stop" => { /* Stop calculating immediately. */ }
                "uci" => {
                    /* Print ID, all options and "uciok" */
                    println!("uciok");
                }
                "ucinewgame" => { /* What it sounds like. Set pos to start pos, etc. */ }
                "q" | "quit" => {
                    /* Quit as soon as possible */
                    exit(0);
                }

                /* non-standard commands */
                /* "f" - find magic numbers for each square for bishop and rook */
                "f" => {
                    Engine::find_magics::<{ crate::defs::Pieces::BISHOP }>();
                    Engine::find_magics::<{ crate::defs::Pieces::ROOK }>();
                }
                /* "p" - prints current position */
                "p" => {
                    engine.pretty_print_board();
                }
                /* "perft n", where n is a number - run perft to depth n */
                "perft" => {
                    if let Some(depth) = line.next() {
                        match depth.parse::<u8>() {
                            Ok(result) => engine.perft(result),
                            Err(result) => println!("{}; must give 0-255", result),
                        }
                    }
                }

                other => {
                    println!("Unrecognised option \"{other}\".");
                }
            }
        } else {
            println!(
                "Unreachable code reached. (Each line should have at least 1 iterable element.)"
            );
            exit(1);
        }
    }
}
