use chess_core::{
    r#move::{MoveGenerator, MoveList},
    state::{game_state::State, make_unmake::MakeUnmaker},
};
use chess_engines::alpha_beta::search::SearchContext;
use chrono::Duration;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct EvaluationResult {
    pub score: i32,
    pub best_move: String, // TODO: change to pv
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FullGameState {
    pub fen: String,
    pub pgn: String,
}

pub fn evaluate(fgs: FullGameState) -> EvaluationResult {
    let state = &mut State::from_fen(fgs.fen);
    let search_ctx = &mut SearchContext::new(state, None);
    let (score, pv) = search_ctx.iterative_deepen(Duration::new(1, 0).unwrap());

    EvaluationResult {
        score,
        best_move: format!("{}", pv.last().unwrap()),
    }
}

/// Does not account for promotion
pub fn is_move_legal(fen: String, r#move: String) -> bool {
    let state = &mut State::from_fen(fen);
    let move_generator = &MoveGenerator::new();
    let make_unmaker = &mut MakeUnmaker::new(state);
    let move_list = &mut MoveList::new();
    move_list.new_ply();
    move_generator.pseudo_legal_moves(make_unmaker.state, move_list);
    let pseudo_legal_move = move_list
        .current_ply()
        .into_iter()
        .find(|m| m.matches_perft_string(r#move.split_at(4).0));
    if let Some(pseudo_legal_move) = pseudo_legal_move {
        make_unmaker.make_move(*pseudo_legal_move);
        move_generator.was_move_legal(state)
    } else {
        false
    }
}

pub fn needs_promotion(fen: String, r#move: String) -> bool {
    let state = &mut State::from_fen(fen);
    let move_generator = &MoveGenerator::new();
    let make_unmaker = &mut MakeUnmaker::new(state);
    let move_list = &mut MoveList::new();
    move_list.new_ply();
    move_generator.pseudo_legal_moves(make_unmaker.state, move_list);
    move_list
        .current_ply()
        .into_iter()
        .find(|m| m.matches_perft_string(r#move.split_at(4).0))
        .unwrap()
        .code()
        .as_promotion()
        .is_some()
}

pub fn make_move(fgs: FullGameState, r#move: String) -> FullGameState {
    let state = &mut State::from_fen(fgs.fen);
    let move_generator = &MoveGenerator::new();
    let make_unmaker = &mut MakeUnmaker::new(state);
    let mut move_list = Vec::new();
    move_generator.pseudo_legal_moves(make_unmaker.state, &mut move_list);
    let pseudo_legal_move = move_list
        .into_iter()
        .find(|m| m.matches_perft_string(r#move.as_str()))
        .unwrap();
    make_unmaker.make_move(pseudo_legal_move);
    FullGameState {
        fen: state.to_fen(),
        pgn: "".to_string(),
    }
}

pub fn respond(fgs: FullGameState) -> FullGameState {
    let state = &mut State::from_fen(fgs.fen);
    let search_ctx = &mut SearchContext::new(state, None);
    let (_, m) = search_ctx.iterative_deepen(Duration::new(0, 300_000_000).unwrap());
    let make_unmaker = &mut MakeUnmaker::new(state);
    make_unmaker.make_move(*m.last().unwrap());
    FullGameState {
        fen: state.to_fen(),
        pgn: "".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate() {
        let fgs = FullGameState {
            fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string(),
            pgn: String::new(),
        };
        let res = evaluate(fgs);
        println!("{}", res.best_move);
        assert_eq!(res.score, 35);
    }

    #[test]
    fn test_evaluate_bug() {
        let fen = "r1bqk1nr/pppp1ppp/2B5/4p2Q/4P3/8/PPPP1bPP/RNB1K1NR w KQkq - 0 5";
        let fgs = FullGameState {
            fen: fen.to_string(),
            pgn: String::new(),
        };
        let _res = evaluate(fgs);
        dbg!(_res.best_move);
    }
}
