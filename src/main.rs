use std::{
    io,
    process::exit,
};

use crate::board::*;

mod board;
mod defs;
mod util;

fn main() {
    let board = Board::new();
    uci_main_loop();
}

fn uci_main_loop() {
    let mut input = String::new();
    loop {
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read from stdin");
        handle_input_line(&input);
        input.clear();
    }
}

fn handle_input_line(line: &String) {
    let mut line = line.trim().split(' ');

    // handle each UCI option
    if let Some(command) = line.next() {
        match command {
            "debug" => {
                 /* Sets debug to "on" or "off". Default "off". */
                 /* This can be ignored. */
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
                 * - perft [unofficial]: run perft to x plies
                 */
            }
            "isready" => {
                /* Immediately print "readyok" */
                println!("readyok");
            }
            "ponderhit" => {
                /* The user has played the expected move. */
                /* Don't implement. */
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
            }
            "setoption" => {
                /* Next element of line_iter should be "name".
                 * Element after "name" should be one of the
                 * options specified from "uci" command.
                 */
            }
            "stop" => {
                /* Stop calculating immediately. */
            }
            "uci" => {
                /* Print ID, all options and "uciok" */
                println!("uciok");
            }
            "ucinewgame" => {
                /* What it sounds like. Set pos to start pos, etc. */
            }
            "quit" => {
                /* Quit as soon as possible */
                exit(0);
            }
            /* non-standard commands */
            "p" => {
                // pretty print
            }
            other => {
                println!("Unrecognised option \"{other}\".");
            }
        }
    } else {
        println!("Unreachable code reached. (Each line should have at least 1 iterable element.)");
        exit(1);
    }
}
