use super::{Depth, Node, OtherNode, Pv, SearchInfo, SearchStatus};
use crate::{
    board::Board,
    defs::MoveType,
    evaluation::{evaluate, Eval, DRAW, INF_EVAL},
    movegen::generate_moves,
};

/// Performs a search on `board`.
///
/// Returns the evaluation of after searching to the given depth. If `NodeType`
/// is `Root`, `pv` will always have at least one legal move in it after the
/// search.
pub fn search<NodeType: Node>(
    search_info: &mut SearchInfo,
    pv: &mut Pv,
    board: &Board,
    depth: Depth,
) -> Eval {
    if depth == 0 {
        return evaluate(board);
    }

    if !NodeType::IS_ROOT && search_info.is_draw(board.halfmoves()) {
        return DRAW;
    }

    let mut best_score = -INF_EVAL;
    let mut new_pv = Pv::new();
    let moves = generate_moves::<{ MoveType::ALL }>(board);

    let mut total_moves = 0;
    for mv in moves {
        let mut copy = *board;
        if !copy.make_move(mv) {
            continue;
        }

        // make sure we always have at least one legal move ready to play
        if NodeType::IS_ROOT && total_moves == 0 {
            pv.enqueue(mv);
        }

        search_info.past_zobrists.push(copy.zobrist());
        let result = -search::<OtherNode>(search_info, &mut new_pv, &copy, depth - 1);
        search_info.past_zobrists.pop();
        search_info.nodes += 1;

        // if the search was stopped early, we can't trust its results
        if search_info.check_status() != SearchStatus::Continue {
            return 0;
        }

        // We've found a better move for us, but not good enough to raise beta.
        if result > best_score {
            best_score = result;
            pv.clear();
            pv.enqueue(mv);
            pv.append_pv(&mut new_pv);
        }

        new_pv.clear();
        total_moves += 1;
    }

    if !NodeType::IS_ROOT && total_moves == 0 {
        -INF_EVAL
    } else {
        DRAW
    };

    best_score
}
