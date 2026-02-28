use chrono::{Duration, Local};

use chess_core::{
    r#move::{Move, MoveGenerator, MoveList},
    state::{game_state::State, make_unmake::MakeUnmaker},
};

use super::transposition_table::{TranspositionTable, TtEntry};

pub struct SearchContext<'a> {
    pub make_unmaker: MakeUnmaker<'a>,
    pub move_generator: MoveGenerator,
    pub move_list: MoveList,
    pub transpos: TranspositionTable,
    pub max_depth: u8,
}

impl SearchContext<'_> {
    const MIN_SCORE: i32 = i32::MIN + 1;
    const MAX_SCORE: i32 = i32::MAX;

    pub fn new(state: &mut State, max_depth: Option<u8>) -> SearchContext<'_> {
        SearchContext {
            make_unmaker: MakeUnmaker::new(state),
            move_generator: MoveGenerator::new(),
            move_list: MoveList::new(),
            transpos: TranspositionTable::new(),
            max_depth: max_depth.unwrap_or(1),
        }
    }

    pub fn iterative_deepen(&mut self, max_time: Duration) -> (i32, Vec<Move>) {
        let mut time_taken = Duration::new(0, 0).unwrap();
        let prev_depth = self.max_depth;

        let (mut score, pv) = (0, &mut Vec::new());

        while time_taken < max_time {
            let start_time = Local::now();
            let prev_pv = pv.clone();
            (score, *pv) = self.search(prev_pv);
            time_taken = Local::now() - start_time;
            self.max_depth += 1;
        }
        self.max_depth = prev_depth;
        (score, pv.clone())
    }

    pub fn search(&mut self, prev_pv: Vec<Move>) -> (i32, Vec<Move>) {
        let mut prev_pv = prev_pv;
        let mut pv = Vec::new();
        let score =
            self.alpha_beta_search(Self::MIN_SCORE, Self::MAX_SCORE, 0, &mut pv, &mut prev_pv);
        (score, pv)
    }

    /// Add pseudo legal moves to move list and returns number and size of ply
    fn add_moves_to_list(&mut self, prev_pv: &mut Vec<Move>) -> (usize, usize) {
        self.move_list.new_ply();
        self.move_generator
            .pseudo_legal_moves(self.make_unmaker.state, &mut self.move_list);
        self.move_list.order_ply(prev_pv.pop());

        let ply_number = self.move_list.ply_number();
        (ply_number, self.move_list.ply_size(ply_number))
    }

    fn alpha_beta_search(
        &mut self,
        alpha: i32,
        beta: i32,
        depth: u8,
        pv: &mut Vec<Move>,
        prev_pv: &mut Vec<Move>,
    ) -> i32 {
        let mut alpha = alpha;
        if depth == self.max_depth {
            return self.quiesce(alpha, beta, depth, pv, prev_pv);
        }

        let (ply_number, ply_size) = self.add_moves_to_list(prev_pv);

        let mut best_score = i32::MIN + 1;
        let mut best_move = None;
        let mut line: Vec<Move> = Vec::new();

        for i in 0..ply_size {
            let m = self.move_list.r#move(ply_number, i);

            self.make_unmaker.make_move(m);
            if !self.move_generator.was_move_legal(self.make_unmaker.state) {
                self.make_unmaker.unmake_move(m);
                continue;
            }
            let score = -self.alpha_beta_search(-beta, -alpha, depth + 1, &mut line, prev_pv);
            // if let Some(tt_entry) = self.transpos.get(self.make_unmaker.zobrist_hash) {
            //     score = -tt_entry.score;
            //     line.push(tt_entry.best_move);
            // }
            self.make_unmaker.unmake_move(m);

            if score > best_score {
                best_score = score;
                best_move = Some(m);
                if score > alpha {
                    alpha = score;
                    pv.clear();
                    pv.append(&mut line.clone());
                    pv.push(m);
                }
            }
            if score >= beta {
                break;
            }
        }

        self.move_list.drop_current_ply();

        if let Some(best_move) = best_move {
            self.transpos.store(TtEntry {
                hash: self.make_unmaker.zobrist_hash,
                depth,
                score: best_score,
                best_move,
            });
        } else {
            // If best move if still 0, either stalemate or checkmate
            // Evaluation function will catch this
            best_score = self.evaluate();
        }

        best_score
    }

    fn quiesce(
        &mut self,
        alpha: i32,
        beta: i32,
        depth: u8,
        pv: &mut Vec<Move>,
        prev_pv: &mut Vec<Move>,
    ) -> i32 {
        if depth >= self.max_depth + 4 {
            pv.clear();
            return self.evaluate();
        }
        let mut alpha = alpha;
        let (ply_number, ply_size) = self.add_moves_to_list(prev_pv);

        let static_score = self.evaluate();
        let mut best_score = static_score;
        let mut best_move = None;

        if static_score >= beta {
            pv.clear();
            self.move_list.drop_current_ply();
            return static_score;
        }

        if static_score > alpha {
            alpha = static_score;
            // Not sure what to do here
            best_move = None;
        }

        let mut line: Vec<Move> = Vec::new();

        for i in 0..ply_size {
            let m = self.move_list.r#move(ply_number, i);
            if m.code().is_quiet() {
                continue;
            }
            // println!("{}Exploring {}", "  ".repeat(depth as usize), m.to_pretty_string());
            self.make_unmaker.make_move(m);
            if !self.move_generator.was_move_legal(self.make_unmaker.state) {
                self.make_unmaker.unmake_move(m);
                continue;
            }
            let score = -self.quiesce(-beta, -alpha, depth + 1, &mut line, prev_pv);
            // println!("{}{} scored {}", "  ".repeat(depth as usize), m.to_pretty_string(), score);
            // if let Some(tt_entry) = self.transpos.get(self.make_unmaker.zobrist_hash) {
            //     score = -tt_entry.score;
            //     line.push(tt_entry.best_move);
            // }
            self.make_unmaker.unmake_move(m);
            if score > best_score {
                best_score = score;
                best_move = Some(m);
                if score > alpha {
                    alpha = score;
                    pv.clear();
                    pv.append(&mut line.clone());
                    pv.push(m);
                }
            }
            if score >= beta {
                break;
            }
        }
        self.move_list.drop_current_ply();

        if let Some(best_move) = best_move {
            self.transpos.store(TtEntry {
                hash: self.make_unmaker.zobrist_hash,
                depth: 0,
                score: best_score,
                best_move,
            });
        } else {
            // No moves found, stop quiescence
            pv.clear();
            return static_score;
        }

        best_score
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader, Write},
        process::{Command, Stdio},
    };

    use chrono::TimeDelta;

    use chess_core::{color::Color, r#move::MoveCode, square::Square};

    use super::*;

    #[test]
    fn test_quiesce() {
        let cases = [
            // starting position
            (
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                (0, 0, vec![]),
            ),
            // white is up by a pawn, black has 4 more mobility
            (
                "rnbqkbnr/ppppppp1/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                (80, 80, vec![]),
            ),
            // white is up by a knight, black to play
            (
                "rnbqkb1r/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
                (-350, -300, vec![]),
            ),
            // One capture + lots of extra mobility
            (
                "8/8/8/8/8/8/qQ/5k1K w - - 0 1",
                (
                    900,
                    1050,
                    vec![Move::new(Square(9), Square(8), MoveCode::Capture)],
                ),
            ),
            // Two captures
            (
                "8/8/8/8/8/1p6/qR6/5k1K w - - 0 1",
                (
                    -150,
                    -50,
                    vec![
                        Move::new(Square(17), Square(8), MoveCode::Capture),
                        Move::new(Square(9), Square(8), MoveCode::Capture),
                    ],
                ),
            ),
            // Capture rook with queen but get taken or capture pawn with no capture
            (
                "8/8/8/8/1p6/8/rQ6/r4k1K w - - 0 1",
                (
                    -150,
                    -50,
                    vec![Move::new(Square(9), Square(25), MoveCode::Capture)],
                ),
            ),
            // Capture + promotion sequence resulting in gain for white
            // Black is not forced to make second capture. Static eval can be considered best move.
            (
                "k7/pp5r/6P1/3p4/4P3/8/6PP/7K w - - 0 1",
                (
                    50,
                    150,
                    vec![
                        Move::new(Square(46), Square(55), MoveCode::Capture),
                        // Move::new(Square(35), Square(28), MoveCode::Capture),
                        // Move::new(Square(55), Square(63), MoveCode::QueenPromotion)
                    ],
                ),
            ),
        ];
        for (fen, (lower_bound, upper_bound, expected_pv)) in cases {
            let state = &mut State::from_fen(fen.to_string());
            let prev_pv = &mut Vec::new();
            let pv = &mut Vec::new();
            let mut context = SearchContext::new(state, None);
            let score = context.quiesce(
                SearchContext::MIN_SCORE,
                SearchContext::MAX_SCORE,
                0,
                pv,
                prev_pv,
            );
            assert_eq!(*prev_pv, vec![]);
            assert_eq!(*pv, expected_pv, "State: {:?}", state);
            assert!(score >= lower_bound, "{} < {}", score, lower_bound);
            assert!(score <= upper_bound, "{} > {}", score, upper_bound);
        }
    }

    #[test]
    fn test_vs_stockfish() {
        // 27/01: current estimated elo: 2000
        // Lacking transpo table, good move ordering, and simple eval function
        let mut stockfish_cli = Command::new("stockfish")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to start stockfish");
        let mut stockfish_stdin = stockfish_cli.stdin.take().unwrap();
        let mut bufreader = BufReader::new(stockfish_cli.stdout.as_mut().unwrap());

        let mut state =
            State::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string());
        let mut search_ctx = SearchContext::new(&mut state, None);

        stockfish_stdin
            .write_all("setoption name UCI_LimitStrength value true\n".as_bytes())
            .unwrap();
        stockfish_stdin
            .write_all("setoption name UCI_Elo value 2000\n".as_bytes())
            .unwrap();
        stockfish_stdin.flush().unwrap();

        let mut time_sum = TimeDelta::zero();
        let mut time_count = 0;

        while !search_ctx.is_checkmate() && search_ctx.make_unmaker.state.halfmove < 200 {
            if search_ctx.make_unmaker.state.flags.active_color() == Color::White {
                let start_time = Local::now();
                let (score, pv) =
                    search_ctx.iterative_deepen(Duration::new(0, 500_000_000).unwrap());
                let time_taken = Local::now() - start_time;
                time_sum += time_taken;
                time_count += 1;
                for m in pv.iter() {
                    println!("{}", m);
                }
                dbg!(score);
                let m = pv.last().unwrap();
                dbg!(m);
                search_ctx.make_unmaker.make_move(*m);
                println!("{}", search_ctx.make_unmaker.state.to_fen());
            } else {
                stockfish_stdin
                    .write_all(
                        format!("position fen {}\n", search_ctx.make_unmaker.state.to_fen())
                            .as_bytes(),
                    )
                    .unwrap();
                stockfish_stdin
                    .write_all("go movetime 2000\n".as_bytes())
                    .unwrap();
                stockfish_stdin.flush().unwrap();
                let mut buf = String::new();
                while !buf.contains("bestmove") {
                    buf.clear();
                    bufreader.read_line(&mut buf).unwrap();
                }
                let move_str = buf.split_whitespace().nth(1).unwrap();
                dbg!(move_str);
                search_ctx.move_list.new_ply();
                search_ctx
                    .move_generator
                    .pseudo_legal_moves(search_ctx.make_unmaker.state, &mut search_ctx.move_list);
                let m = search_ctx
                    .move_list
                    .current_ply()
                    .into_iter()
                    .find(|m| m.matches_perft_string(move_str))
                    .unwrap()
                    .clone();
                search_ctx.move_list.drop_current_ply();
                search_ctx.make_unmaker.make_move(m);
                println!("{}", search_ctx.make_unmaker.state.to_fen());
            }
        }

        println!(
            "Average time taken: {:?}",
            (time_sum / time_count).num_milliseconds()
        );

        if search_ctx.is_checkmate()
            && search_ctx.make_unmaker.state.flags.active_color() == Color::White
        {
            println!("Black wins");
        } else if search_ctx.is_checkmate()
            && search_ctx.make_unmaker.state.flags.active_color() == Color::Black
        {
            println!("White wins");
        } else {
            println!("Draw");
        }

        stockfish_cli.kill().unwrap();
    }
}
