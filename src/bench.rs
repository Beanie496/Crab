#![cfg(test)]

use lazy_static::lazy_static;

use crate::engine::Engine;

#[derive(Clone, Copy)]
struct TestPosition<'a> {
    position: &'a str,
    perft_depth: u8,
    perft_result: u64,
}

lazy_static! {
    /// Test positions used to check the correctness of movegen/make/unmake.
    static ref TEST_POSITIONS: Vec<TestPosition<'static>> = vec![
        // startpos. Depth 6 has ep, checks, discovered checks, double checks
        // and checkmates.
        TestPosition::new(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -",
            6,
            119_060_324,
        ),
        // kiwipete. Depth 4 tests everything, but depth 5 to be safe.
        TestPosition::new(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -",
            5,
            193_690_690,
        ),
        // tests ep which would be a discovered attack on own king
        TestPosition::new(
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - -",
            5,
            674_624
        ),
        TestPosition::new(
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq -",
            4,
            422_333,
        ),
        TestPosition::new(
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq -",
            5,
            15_833_292,
        ),
        // tests enemy knight taking own rook, disallowing castling
        TestPosition::new(
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ -",
            5,
            89_941_194,
        ),
        TestPosition::new(
            "r3k2r/2pb1ppp/2pp1q2/p7/1nP1B3/1P2P3/P2N1PPP/R2QK2R w KQkq a6",
            5,
            67_956_855,
        ),
    ];
}

impl<'a> TestPosition<'a> {
    const fn new(position: &'a str, perft_depth: u8, perft_result: u64) -> Self {
        Self {
            position,
            perft_depth,
            perft_result,
        }
    }
}

impl TestPosition<'_> {
    fn run_test(&self, engine: &mut Engine) {
        engine.set_position(self.position, "");
        println!("Position: {}", self.position);
        assert_eq!(
            engine.perft::<false, false>(self.perft_depth),
            self.perft_result
        );
    }
}

#[test]
fn test_positions() {
    let mut engine = Engine::new();
    for position in TEST_POSITIONS.iter() {
        position.run_test(&mut engine);
    }
}
