#![cfg(test)]

use std::{
    sync::{mpsc, Arc, Mutex},
    thread::{available_parallelism, spawn},
};

use crate::engine::Engine;

struct TestPosition {
    position: String,
    perft_depth: u8,
    perft_result: u64,
}

static TEST_POSITIONS: &str = include_str!("../test_positions.epd");

impl TestPosition {
    const fn new(position: String, perft_depth: u8, perft_result: u64) -> Self {
        Self {
            position,
            perft_depth,
            perft_result,
        }
    }
}

impl TestPosition {
    fn run_test(&self, engine: &mut Engine) {
        engine.set_position(&self.position, "");
        println!("Position: {}", self.position);
        assert_eq!(
            engine.perft::<false, false>(self.perft_depth),
            self.perft_result
        );
    }
}

#[test]
fn test_positions() {
    let engine = Engine::new();
    let (tx, rx) = mpsc::channel();
    let rx = Arc::new(Mutex::new(rx));
    let mut handles = Vec::new();

    // add all test positions to the queue
    for position in TEST_POSITIONS.lines() {
        let mut tokens = position.split(' ');

        let result = tokens
            .next_back()
            .and_then(|result| result.parse::<u64>().ok())
            .unwrap();

        let mut fen = String::new();
        for token in tokens.take(6) {
            fen.push_str(&token);
            fen.push(' ');
        }
        fen.pop();

        // each position is just to depth 4
        let depth = 4;

        let test_pos = TestPosition::new(fen, depth, result);
        tx.send(test_pos).unwrap();
    }

    // create as many threads as is optimal. If no threads available, the test
    // positions won't be able to be run, so panic.
    for _ in 0..available_parallelism().unwrap().get() {
        // I'm manually doing `.clone()` because deriving `Copy` for `Engine`
        // (and by extension `Board`) results in a noticeable slowdown in
        // `perft`, for some goddamn reason.
        let mut engine = engine.clone();
        let rx = Arc::clone(&rx);
        // Spawn a thread that dequeues and runs the test positions from the
        // receiver until there are no positions left
        handles.push(spawn(move || loop {
            let test_pos = rx.lock().unwrap().try_recv();
            if let Ok(test_pos) = test_pos {
                test_pos.run_test(&mut engine)
            } else {
                return;
            }
        }));
    }

    for handle in handles {
        handle.join().expect("A position has failed!");
    }
}
