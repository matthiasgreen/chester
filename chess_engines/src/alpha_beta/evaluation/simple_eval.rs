use chess_core::state::{State, bitboard::BitBoard};

use super::super::search::SearchContext;

const DOUBLED_PAWN_COEF: i32 = 40;
const ISOLATED_PAWN_COEF: i32 = 40;
const MOBILITY_COEF: i32 = 5;

struct EvaluationContext<'a>(&'a State);

impl EvaluationContext<'_> {
    fn doubled_pawn_number(&self) -> i32 {
        // - number of doubled pawns on active side + number of doubled on passive side
        let (active_boards, passive_boards) = self.0.split_boards();
        let (active_pawns, passive_pawns) = (active_boards.pawn, passive_boards.pawn);

        // For each column, count the number of pawns in that column
        let (mut active_doubled_pawns, mut passive_doubled_pawns) = (0, 0);
        for col in 0..8 {
            let column = BitBoard::file(col);
            let (active_col, passive_col) = (column & active_pawns, column & passive_pawns);
            if !active_col.is_empty() {
                active_doubled_pawns += active_col.count_ones() as i32 - 1;
            }
            if !passive_col.is_empty() {
                passive_doubled_pawns += passive_col.count_ones() as i32 - 1;
            }
        }

        passive_doubled_pawns - active_doubled_pawns
    }

    fn isolated_pawn_number(&self) -> i32 {
        // - number of isolated pawns on active side + number of isolated pawns on passive side
        let (active_boards, passive_boards) = self.0.split_boards();
        let (active_pawns, passive_pawns) = (active_boards.pawn, passive_boards.pawn);

        // For each column, count the number of pawns in the adjacent columns
        // Watch out for doubled isolated pawns, these should only be counted once (?)

        let (mut active_isolated_pawns, mut passive_isolated_pawns) = (0, 0);
        for (counter, pawns) in [
            (&mut active_isolated_pawns, active_pawns),
            (&mut passive_isolated_pawns, passive_pawns),
        ] {
            // Start with edge of board
            if !(pawns & BitBoard::file(0)).is_empty() && (pawns & BitBoard::file(1)).is_empty() {
                *counter += 1;
            }
            if !(pawns & BitBoard::file(7)).is_empty() && (pawns & BitBoard::file(6)).is_empty() {
                *counter += 1;
            }
            for col in 1..7 {
                if !(pawns & BitBoard::file(col)).is_empty()
                    && (pawns & BitBoard::file(col - 1)).is_empty()
                    && (pawns & BitBoard::file(col + 1)).is_empty()
                {
                    *counter += 1;
                }
            }
        }
        passive_isolated_pawns - active_isolated_pawns
    }

    fn pawn_structure_score(&self) -> i32 {
        DOUBLED_PAWN_COEF * self.doubled_pawn_number()
            + ISOLATED_PAWN_COEF * self.isolated_pawn_number()
    }

    fn board_material(active_pieces: BitBoard, passive_pieces: BitBoard, coef: i32) -> i32 {
        let active_material = active_pieces.count_ones() as i32;
        let passive_material = passive_pieces.count_ones() as i32;
        coef * (active_material - passive_material)
    }

    fn material_score(&self) -> i32 {
        let (active_pieces, passive_pieces) = self.0.split_boards();
        Self::board_material(active_pieces.pawn, passive_pieces.pawn, 100)
            + Self::board_material(active_pieces.knight, passive_pieces.knight, 300)
            + Self::board_material(active_pieces.bishop, passive_pieces.bishop, 300)
            + Self::board_material(active_pieces.rook, passive_pieces.rook, 500)
            + Self::board_material(active_pieces.queen, passive_pieces.queen, 900)
    }
}

impl SearchContext<'_> {
    /// Mutable due to move list use but does not modify the state
    pub fn is_checkmate(&mut self) -> bool {
        if !self.move_generator.is_check(self.make_unmaker.state) {
            return false;
        }
        self.move_list.new_ply();
        let mut move_found = false;
        self.move_generator
            .pseudo_legal_moves(self.make_unmaker.state, &mut self.move_list);
        for m in self.move_list.current_ply() {
            self.make_unmaker.make_move(*m);
            if self.move_generator.was_move_legal(self.make_unmaker.state) {
                move_found = true;
                self.make_unmaker.unmake_move(*m);
                break;
            }
            self.make_unmaker.unmake_move(*m);
        }
        self.move_list.drop_current_ply();
        !move_found
    }

    fn active_side_move_number(&mut self) -> i32 {
        // TODO: use safe mobility?
        self.move_list.new_ply();
        // let mut move_number = 0;
        self.move_generator
            .pseudo_legal_moves(self.make_unmaker.state, &mut self.move_list);
        let res = self.move_list.ply_size(self.move_list.ply_number());
        self.move_list.drop_current_ply();
        res as i32
        // for m in self.move_list.get_current_ply() {
        //     // FIXME: issue with same side en passant, this is a hack
        //     if m.is_en_passant() {continue};
        //     self.make_unmaker.make_move(*m);
        //     if self.move_generator.was_move_legal(self.make_unmaker.state) {
        //         move_number += 1;
        //     }
        //     self.make_unmaker.unmake_move(*m);
        // }
        // self.move_list.drop_current_ply();
        // move_number
    }

    /// Mutable due to move list use but does not modify the state
    fn mobility_score(&mut self) -> i32 {
        // active mobility - passive mobility
        let active_mobility = self.active_side_move_number();
        self.make_unmaker.state.flags.toggle_active_color();
        let passive_mobility = self.active_side_move_number();
        self.make_unmaker.state.flags.toggle_active_color();
        MOBILITY_COEF * (active_mobility - passive_mobility)
    }

    /// Mutable due to move list use but does not modify the state
    pub fn evaluate(&mut self) -> i32 {
        if self.is_checkmate() {
            return -100000;
        }
        let eval = EvaluationContext(&self.make_unmaker.state);
        eval.pawn_structure_score() + eval.material_score() + self.mobility_score()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_is_checkmate() {
        for (fen, result) in [
            // starting position
            (
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                false,
            ),
            // mate
            // ("8/8/8/8/8/8/5KQ1/7k b - - 0 1", true),
        ] {
            let state = &mut State::from_fen(fen.to_string());
            let mut search_context = SearchContext::new(state, None);
            assert_eq!(search_context.is_checkmate(), result);
        }
    }

    #[test]
    fn test_material_evaluation() {
        for (fen, result) in [
            // starting position
            (
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                0,
            ),
            // white is up by a pawn
            (
                "rnbqkbnr/ppppppp1/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                100,
            ),
            // white is up by a knight, black to play
            (
                "rnbqkb1r/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
                -300,
            ),
            // Black is up by a knight
            (
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKB1R w KQkq - 0 1",
                -300,
            ),
        ] {
            let state = &mut State::from_fen(fen.to_string());
            let eval = EvaluationContext(state);
            let score = eval.material_score();
            assert_eq!(score, result);
        }
    }

    #[test]
    fn test_doubled_pawn_evaluation() {
        for (fen, result) in [
            // starting position
            (
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                0,
            ),
            // white has doubled pawns, white to move
            (
                "rnbqkbnr/pppppppp/8/8/8/1P6/1PPPPPPP/RNBQKBNR w KQkq - 0 1",
                -DOUBLED_PAWN_COEF,
            ),
            // black has doubled pawns, white to move
            (
                "rnbqkbnr/1ppppppp/1p6/8/8/8/PPPPPPPP/RNBQKB1R w KQkq - 0 1",
                DOUBLED_PAWN_COEF,
            ),
            // white has tripled pawns, black to move
            (
                "rnbqkbnr/pppppppp/8/8/2P5/2P5/11PPPPPP/RNBQKBNR b KQkq - 0 1",
                2 * DOUBLED_PAWN_COEF,
            ),
        ] {
            let state = &mut State::from_fen(fen.to_string());
            let eval = EvaluationContext(state);
            let score = eval.doubled_pawn_number() * DOUBLED_PAWN_COEF;
            assert_eq!(score, result, "FEN: {}", fen);
        }
    }

    #[test]
    fn test_isolated_pawn_evaluation() {
        for (fen, result) in [
            // starting position
            (
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                0,
            ),
            // white has isolated pawns on the side, white to move
            (
                "rnbqkbnr/pppppppp/8/8/8/8/P1PPPPPP/RNBQKBNR w KQkq - 0 1",
                -ISOLATED_PAWN_COEF,
            ),
            // white has isolated pawns on the middle, black to move
            (
                "rnbqkbnr/pppppppp/8/8/8/8/1P1PPPPP/RNBQKBNR b KQkq - 0 1",
                ISOLATED_PAWN_COEF,
            ),
        ] {
            let state = &mut State::from_fen(fen.to_string());
            let eval = EvaluationContext(state);
            let score = eval.isolated_pawn_number() * ISOLATED_PAWN_COEF;
            assert_eq!(score, result, "FEN: {}", fen);
        }
    }

    #[test]
    fn test_mobility_evaluation() {
        for (fen, result) in [
            // starting position
            (
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                0,
            ),
            // white has more mobility, white to move
            (
                "rnbqkbnr/pppppppp/8/8/8/8/1PPPPPPP/RNBQKBNR w KQkq - 0 1",
                4 * MOBILITY_COEF,
            ),
        ] {
            let state = &mut State::from_fen(fen.to_string());
            let mut search_context = SearchContext::new(state, None);
            let score = search_context.mobility_score();
            assert_eq!(score, result, "FEN: {}", fen);
        }
    }
}
