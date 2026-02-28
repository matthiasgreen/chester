use chess_core::{
    color::Color,
    hash::{Hasher, zobrist::ZobristHasher},
    r#move::Move,
    position::Position,
};
use rand::{rngs::ThreadRng, seq::IteratorRandom};

/// Function that evaluates a final board.
/// No legal moves should be left.
fn white_score<H: Hasher>(pos: Position<H>) -> f32 {
    if pos.state.get().is_check() {
        match pos.state.get().flags.active_color() {
            Color::White => 0.0,
            Color::Black => 1.0,
        }
    } else {
        0.5
    }
}

fn rollout_policy<H: Hasher>(
    pos: &mut Position<H>,
    move_list: &mut Vec<Move>,
    rng: &mut ThreadRng,
) -> Option<Move> {
    move_list.clear();
    pos.pseudo_legal_moves(move_list);

    std::iter::from_fn(|| {
        let (i, &mut m) = move_list.into_iter().enumerate().choose(rng)?;
        move_list.remove(i);
        Some(m)
    })
    .find(|&m| {
        pos.make(m);
        let res = pos.was_move_legal();
        pos.unmake(m);
        res
    })
}

/// Performs a single rollout and returns the evaluation of the final state.
/// Position is cloned since unmaking all rollout moves is innefficient
fn rollout(mut pos: Position<ZobristHasher>) -> f32 {
    // TODO: stop after some max_iter and return simple eval
    let mut buf = Vec::new();
    let rng = &mut rand::rng();

    while let Some(m) = rollout_policy(&mut pos, &mut buf, rng)
        && !pos.state.get().insufficient_material()
    {
        pos.make(m);
    }
    white_score(pos)
}

struct Node {
    untried_moves: Vec<Move>,
    causing_move: Move,
    q: f32,
    n: f32,
}

impl Node {
    fn uct_best_child(c: f32) -> Option<Move> {
        let t = match pos.state.get().flags.active_color() {
            Color::White => 1.,
            Color::Black => -1.,
        };
        let old_node = self.nodes.get(&pos.state).unwrap();
        let mut move_list = Vec::new();
        pos.pseudo_legal_moves(&mut move_list);

        move_list
            .into_iter()
            .filter(|m| {
                pos.make(*m);
                let res = pos.was_move_legal();
                pos.unmake(*m);
                res
            })
            .max_by_key(|m| {
                let out_edge = old_node.get_action_edge(*m);
                OrderedFloat(
                    t * out_edge.eval
                        + self.exploration_weight
                            * (2. * (old_node.count as f32).log2() / out_edge.visits as f32).sqrt(),
                )
            })
    }
}
