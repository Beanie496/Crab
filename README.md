This is a chess engine made in Rust. It was originally made for my Year 2 Computer Science A-level project.

To compile, [make sure you have Rust installed](https://rustup.rs). Then, run `cargo run --release` from the root directory.

Features:
- Bitboard-based representation and move generation (with a redundant mailbox)
- Magic bitboards
- UCI compatibility
- Aspiration windows
- Principle variation search
- A transposition table
- Null move pruning
- Staged movegen
- Move ordering:
  - TT-move
  - MVV + SEE captures
  - Killer moves
  - Counter moves
- Histories:
  - Butterfly history
- Futility pruning
- Extensions:
  - Check extensions
- Late move reductions
- Quiescence
- Piece-square tables
