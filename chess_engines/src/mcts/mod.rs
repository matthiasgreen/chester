use std::{
    fmt::Display,
    time::{Duration, Instant},
};

use chess_core::{
    color::Color,
    hash::{HashedState, Hasher, zobrist::ZobristHasher},
    r#move::Move,
    position::Position,
};
use hashbrown::HashMap;
use itertools::Itertools;
use ordered_float::OrderedFloat; // For max by key of float
use rand::{rngs::ThreadRng, seq::IteratorRandom};

use crate::engine::Engine;

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

/// Alias type to repesent a count of selections.
pub type Count = u64;

/// Node of the MCTS graph
struct Node {
    /// Board of the node
    // position: Position<ZobristHasher>,
    /// Numer of times this node has been selected
    count: Count,
    /// *All* valid actions available on the board, together with the number of times they have been selected (potentially 0)
    /// and the last known evaluation of the result board.
    /// The actions define the outgoing edges (the target nodes can be computed by applying the action on the board)
    out_edges: Vec<OutEdge>,
    /// Evaluation given by the initial rollout on expansion
    initial_eval: f32,
    /// Q(s): complete evaluation of the node (to be updated after each playout)
    eval: f32,
}

impl Node {
    /// Creates the node with a single evaluation from a rollout
    pub fn init(mut position: Position<ZobristHasher>, initial_eval: f32) -> Node {
        // create one outgoing edge per valid action
        let mut out_edges = Vec::new();
        let mut moves = Vec::new();
        position.pseudo_legal_moves(&mut moves);
        for m in moves {
            position.make(m);
            if position.was_move_legal() {
                out_edges.push(OutEdge::new(m));
            }
            position.unmake(m);
        }

        Node {
            // position,
            count: 1,
            out_edges,
            initial_eval,
            eval: initial_eval,
        }
    }

    /// Gets the OutEdge corresponding to the action
    fn get_action_edge(&self, r#move: Move) -> &OutEdge {
        self.out_edges
            .iter()
            .find(|edge| edge.action == r#move)
            .unwrap()
    }

    /// Gets the OutEdge corresponding to the action (mutable)
    fn get_action_edge_mut(&mut self, r#move: Move) -> &mut OutEdge {
        self.out_edges
            .iter_mut()
            .find(|edge| edge.action == r#move)
            .unwrap()
    }

    /// Returns the best action according to the number of visits (N(s,a))
    fn get_best_action(&self) -> Option<Move> {
        self.out_edges
            .iter()
            .max_by_key(|edge| edge.visits)
            .map(|edge| edge.action.clone())
    }
}

/// Edge of the MCTS graph.
///
/// An `OutEdge` is attached to a node (source) and target can be computed by applying the action to the source.
struct OutEdge {
    // action of the edge
    action: Move,
    // N(s,a): number of times this edge was selected
    visits: Count,
    // Q(s,a): Last known evaluation of the board resulting from the action
    eval: f32,
}

impl OutEdge {
    /// Initializes a new edge for this actions (with a count and eval at 0)
    pub fn new(action: Move) -> OutEdge {
        OutEdge {
            action,
            visits: 0,
            eval: 0.,
        }
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // write!(f, "\n{}", self.board)?;
        writeln!(f, "Q: {}    (N: {})", self.eval, self.count)?;
        // display edges by decreasing number of samples
        for OutEdge {
            action,
            visits,
            eval,
        } in self.out_edges.iter().sorted_by_key(|e| u64::MAX - e.visits)
        {
            writeln!(f, "{visits:>8} {action}   [{eval}]")?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct MctsStats {
    total_playouts: u32,
    total_playout_depth: u32,
    total_runtime: Duration,
    total_eval: f32,
    total_eval_count: u32,
}

impl MctsStats {
    /// Returns the number of playouts performed per second on average since the last reset.
    pub fn playouts_per_second(&self) -> f32 {
        if self.total_runtime.as_secs() == 0 {
            return 0.;
        }
        self.total_playouts as f32 / self.total_runtime.as_secs_f32()
    }

    /// Returns the average playout depth since the last reset.
    pub fn average_playout_depth(&self) -> f32 {
        if self.total_playouts == 0 {
            return 0.;
        }
        self.total_playout_depth as f32 / self.total_playouts as f32
    }

    pub fn average_eval(&self) -> f32 {
        if self.total_eval_count == 0 {
            return 0.;
        }
        self.total_eval / self.total_eval_count as f32
    }
}

pub struct MctsEngine {
    /// Graph structure
    nodes: HashMap<HashedState<ZobristHasher>, Node>,
    /// weight given to the exploration term in UCB1
    pub exploration_weight: f32,
    /// Number of rollouts performed
    pub n_rollouts: u32,
    /// Weight given to the heuristic compared to the rollout result (between 0 and 1)
    pub heuristic_weight: f32,

    pub stats: MctsStats,
}
impl MctsEngine {
    pub fn new(exploration_weight: f32, n_rollouts: u32, heuristic_weight: f32) -> MctsEngine {
        MctsEngine {
            nodes: HashMap::new(),
            exploration_weight,
            stats: MctsStats::default(),
            n_rollouts,
            heuristic_weight,
        }
    }
}

impl MctsEngine {
    /// Selects the best action according to UCB1, or `None` if no action is available.
    /// Doesn't modify pos despite mut ref
    pub fn select_ucb1(&self, mut pos: Position<ZobristHasher>) -> Option<Move> {
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

    fn evaluate(&mut self, pos: &Position<ZobristHasher>) -> f32 {
        let eval = rollout(pos);
        self.stats.total_eval += eval;
        self.stats.total_eval_count += 1;
        eval
    }

    /// Performs a playout for this board (s) and returns the (updated) evaluation of the board (Q(s))
    fn playout(&mut self, pos: &mut Position<ZobristHasher>, depth: &mut u32) -> f32 {
        if !self.nodes.contains_key(&pos.state) {
            let eval = self.evaluate(pos);
            self.nodes
                .insert(pos.state.clone(), Node::init(pos.clone(), eval));
            eval
        } else if let Some(m) = self.select_ucb1(pos.clone()) {
            *depth += 1;
            pos.make(m);
            let score = self.playout(pos, depth);
            pos.unmake(m);
            self.update_eval(pos, m, score)
        } else {
            self.nodes.get(&pos.state).unwrap().eval
        }
    }

    /// Updates the evaluation (Q(s)) of the board (s), after selected the action (a) for a new playout
    /// which yieled an evaluation of `action_eval` (Q(s,a))
    fn update_eval(&mut self, pos: &Position<ZobristHasher>, r#move: Move, move_eval: f32) -> f32 {
        debug_assert!(self.nodes.contains_key(&pos.state));
        let node = self.nodes.get_mut(&pos.state).unwrap();
        let out_edge = node.get_action_edge_mut(r#move);
        out_edge.visits += 1;
        out_edge.eval = move_eval;
        node.count += 1;
        node.eval = node.initial_eval / node.count as f32
            + node.out_edges.iter().fold(0., |acc, x| {
                acc + x.visits as f32 / node.count as f32 * x.eval
            });
        node.eval
    }

    fn reset_stats(&mut self) {
        self.stats = MctsStats::default();
    }
}

impl Engine for MctsEngine {
    fn select(&mut self, pos: &mut Position<ZobristHasher>, deadline: Instant) -> Option<Move> {
        let mut curr_time = Instant::now();
        let start_time = curr_time;
        while curr_time < deadline {
            let depth = &mut 1;
            self.playout(pos, depth);
            self.stats.total_playouts += 1;
            self.stats.total_playout_depth += *depth;
            curr_time = Instant::now();
        }
        self.stats.total_runtime += start_time.elapsed();
        self.nodes.get(&pos.state).unwrap().get_best_action()
    }

    fn clear(&mut self) {
        self.nodes.clear();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use super::{MctsEngine, rollout};

    macro_rules! assert_approx_eq {
        ($a:expr, $b:expr, $c:expr) => {
            let diff = ($a - $b).abs();
            assert!(diff < $c, "Expected {} to be close to {}", $a, $b);
        };
    }

    #[test]
    fn test_rollout() {
        const N_ITER: usize = 1000;
        let board = Position::default();
        let mut scores: [f32; N_ITER] = [0.; N_ITER];
        for score in scores.iter_mut() {
            *score = rollout(&board);
        }
        let sum = scores.iter().fold(0., |x, y| x + y);
        assert_approx_eq!(sum / N_ITER as f32, 0.5, 0.05);
    }

    #[test]
    fn test_mcts() {
        let mut pos = Position::default();
        let mut mcts = MctsEngine::new(1., 1, 0.);

        for _ in 1..=100000 {
            mcts.playout(&mut pos, &mut 0);
        }
        println!("After 1000 playouts: \n{}", mcts.nodes[&pos.state]);
    }
}
