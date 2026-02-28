use itertools::Itertools;

use crate::{
    Insert,
    collections::Plys,
    color::Color,
    hash::{HashedState, Hasher},
    r#move::{Move, MoveCode, MoveGenerator},
    square::{CastleSide, SquareFinder},
    state::{State, bitboard::BitBoard, chess_board::PieceType, flags::StateFlags},
};

#[derive(Clone)]
pub struct Position<H: Hasher> {
    pub state: HashedState<H>,
    stack: Vec<IrreversibleInfo>,
}

impl<H: Hasher> std::fmt::Debug for Position<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Position")
            .field("state", &self.state.get())
            .finish()
    }
}

impl<H: Hasher + Default> Default for Position<H> {
    fn default() -> Self {
        Self {
            state: Default::default(),
            stack: Default::default(),
        }
    }
}

impl<H: Hasher> Position<H> {
    pub fn new(state: HashedState<H>) -> Self {
        Self {
            state,
            stack: Vec::new(),
        }
    }

    pub fn from_fen(fen: &str, hasher: H) -> Self {
        Self::new(HashedState::from_fen(fen, hasher))
    }

    pub fn pseudo_legal_moves<T: Insert<Move>>(&self, out: &mut T) {
        MoveGenerator::new(self.state.get()).pseudo_legal_moves(out)
    }

    /// Apply a function to all children
    /// Despite self and buf being mutable, they should end up in the same state as before the function call
    pub fn apply_children<P: Plys<Move> + Insert<Move>>(
        &mut self,
        buf: &mut P,
        mut f: impl FnMut(Move, &mut Self, &mut P),
    ) {
        buf.new_ply();
        self.pseudo_legal_moves(buf);
        let ply_number = buf.ply_number();
        let ply_size = buf.ply_size(ply_number);

        for i in 0..ply_size {
            let m = buf.r#move(ply_number, i);
            self.make(m);
            if self.was_move_legal() {
                f(m, self, buf);
            }
            self.unmake(m);
        }
        buf.drop_current_ply();
    }

    pub fn was_move_legal(&self) -> bool {
        self.state.get().was_move_legal()
    }

    pub fn make(&mut self, r#move: Move) {
        let color = self.state.get().flags.active_color();

        let moved_piece = PieceType::as_array()
            .into_iter()
            .find(|piece| {
                !(self.state.get().active_boards()[*piece].clone() & BitBoard::from(r#move.from()))
                    .is_empty()
            })
            .unwrap();

        let captured_piece = if r#move.code() == MoveCode::EnPassant {
            Some(PieceType::Pawn)
        } else if r#move.code().is_capture() {
            PieceType::as_array().into_iter().find(|piece| {
                !(self.state.get().inactive_boards()[*piece].clone() & BitBoard::from(r#move.to()))
                    .is_empty()
            })
        } else {
            None
        };

        // Push irreversible info to stack
        self.stack.push(IrreversibleInfo::from_state(
            self.state.get(),
            captured_piece,
        ));

        // Discard en passant
        self.state.remove_en_passant();

        // Castles need to be handled seperately since two pieces move
        if let Some(side) = r#move.code().as_castle() {
            self.castle(side);
        } else {
            // Move friendly piece and take capture
            self.state
                .move_piece(r#move.from(), r#move.to(), moved_piece, color);

            // Remove castling rights if moved piece is a king or a rook
            if moved_piece == PieceType::King {
                for side in CastleSide::as_array() {
                    self.state.set_castle_right(color, side, false);
                }
            }
            if moved_piece == PieceType::Rook {
                for side in CastleSide::as_array() {
                    if r#move.from() == SquareFinder(color).castle_rook_target(side) {
                        self.state.set_castle_right(color, side, false);
                    }
                }
            }

            if r#move.code() == MoveCode::EnPassant {
                self.state.remove_piece(
                    SquareFinder(color).en_passant_capture(r#move.to().file()),
                    PieceType::Pawn,
                    !color,
                );
            } else if let Some(piece) = captured_piece {
                self.state.remove_piece(r#move.to(), piece, !color);
            }
        }

        if r#move.code() == MoveCode::DoublePawnPush {
            self.state
                .add_en_passant(SquareFinder(color).en_passant_marker(r#move.from().file()));
        }

        if let Some(piece) = r#move.code().as_promotion() {
            self.state.remove_piece(r#move.to(), moved_piece, color);
            self.state.add_piece(r#move.to(), piece, color);
        }

        self.state.toggle_color();
        self.state.increment_halfmove();
    }

    pub fn unmake(&mut self, r#move: Move) {
        // Let's immediately switch color so we act on active color
        self.state.toggle_color();
        let color = self.state.get().flags.active_color();

        // Restoring castling rights and en passant
        // should have no effect on the rest of the function
        let info = self.stack.pop().unwrap();
        self.restore_castle_rights(info.flags);

        self.state.remove_en_passant();
        if let Some(square) = info.en_passant.get_first_square() {
            self.state.add_en_passant(square);
        }

        let moved_piece = if r#move.code().as_promotion().is_some() {
            PieceType::Pawn
        } else {
            PieceType::as_array()
                .into_iter()
                .find(|piece| {
                    !(self.state.get().active_boards()[*piece].clone()
                        & BitBoard::from(r#move.to()))
                    .is_empty()
                })
                .unwrap()
        };

        // If promotion, replace promoted piece with pawn
        if let Some(piece) = r#move.code().as_promotion() {
            self.state.remove_piece(r#move.to(), piece, color);
            self.state.add_piece(r#move.to(), PieceType::Pawn, color);
        }

        if let Some(side) = r#move.code().as_castle() {
            self.uncastle(side);
        } else {
            // Move active piece back to position
            self.state
                .move_piece(r#move.to(), r#move.from(), moved_piece, color);

            // Uncapture piece
            if r#move.code() == MoveCode::EnPassant {
                self.state.add_piece(
                    SquareFinder(color).en_passant_capture(r#move.to().file()),
                    PieceType::Pawn,
                    !color,
                );
            } else if let Some(captured_piece) = info.captured_piece {
                self.state.add_piece(r#move.to(), captured_piece, !color);
            }
        }
        self.state.decrement_halfmove();
    }

    fn castle(&mut self, side: CastleSide) {
        use PieceType::{King, Rook};
        let color = self.state.get().flags.active_color();

        let king_square = SquareFinder(color).source(King).unwrap();
        let king_target = SquareFinder(color).castle_king_target(side);
        self.state.move_piece(king_square, king_target, King, color);

        let rook_square = SquareFinder(color).castle_rook_source(side);
        let rook_target = SquareFinder(color).castle_rook_target(side);
        self.state.move_piece(rook_square, rook_target, Rook, color);

        // Remove castle rights
        for side in CastleSide::as_array() {
            self.state.set_castle_right(color, side, false);
        }
    }

    fn uncastle(&mut self, side: CastleSide) {
        use PieceType::{King, Rook};
        let color = self.state.get().flags.active_color();

        let king_target = SquareFinder(color).source(King).unwrap();
        let king_square = SquareFinder(color).castle_king_target(side);
        self.state.move_piece(king_square, king_target, King, color);

        let rook_target = SquareFinder(color).castle_rook_source(side);
        let rook_square = SquareFinder(color).castle_rook_target(side);
        self.state.move_piece(rook_square, rook_target, Rook, color);

        // Castling rights are restored seperately
    }

    fn restore_castle_rights(&mut self, flags: StateFlags) {
        for (color, side) in Color::as_array()
            .into_iter()
            .cartesian_product(CastleSide::as_array())
        {
            self.state
                .set_castle_right(color, side, flags.castle_right(color, side));
        }
    }
}

/// Irreversible information needed to unmake a move
#[derive(Clone)]
struct IrreversibleInfo {
    #[allow(unused)]
    halfmove: u16,
    en_passant: BitBoard,
    flags: StateFlags,
    captured_piece: Option<PieceType>,
}

impl IrreversibleInfo {
    fn from_state(state: &State, captured_piece: Option<PieceType>) -> Self {
        IrreversibleInfo {
            halfmove: state.halfmove,
            en_passant: state.en_passant,
            flags: state.flags.clone(),
            captured_piece: captured_piece,
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::{
        hash::{NoopHasher, zobrist::ZobristHasher},
        r#move::MoveList,
        square::Square,
    };
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_make_move() {
        let mut pos = Position::<NoopHasher>::default();
        let m = Move::new(
            Square::new(0, 1).unwrap(),
            Square::new(2, 0).unwrap(),
            MoveCode::QuietMove,
        );
        pos.make(m);
        assert!(
            pos.state.get().boards[Color::White][PieceType::Knight].get(Square::new(2, 0).unwrap())
        );
        pos.unmake(m);
        assert!(
            !pos.state.get().boards[Color::White][PieceType::Knight]
                .get(Square::new(2, 0).unwrap())
        );

        let mut pos = Position::from_fen(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R b KQkq - 0 1",
            NoopHasher {},
        );
        dbg!(&pos.state.get());

        let mut move_list = MoveList::new();
        move_list.new_ply();
        pos.pseudo_legal_moves(&mut move_list);

        for m in move_list.current_ply().iter()
        // .filter(|m| m.code() == MoveCode::Capture)
        {
            dbg!(m);
        }

        pos.make(Move::new(
            Square::new(5, 1).unwrap(),
            Square::new(4, 3).unwrap(),
            MoveCode::Capture,
        ));
        dbg!(pos.state.get());
    }

    #[test]
    fn test_make_unmake_move() {
        let fens = [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        ];
        for fen in fens {
            let mut state = Position::from_fen(fen, ZobristHasher::new());
            recursize_test_make_unmake_move(&mut state, &mut MoveList::new(), 3);
        }
    }

    fn recursize_test_make_unmake_move(
        position: &mut Position<ZobristHasher>,
        move_list: &mut MoveList,
        depth: u8,
    ) {
        if depth == 0 {
            return;
        }
        move_list.new_ply();
        position.pseudo_legal_moves(move_list);
        let current_ply = move_list.ply_number();
        let ply_size = move_list.ply_size(current_ply);

        for i in 0..ply_size {
            let m = move_list.r#move(current_ply, i);

            let original_gs = position.state.get().clone();
            position.make(m);

            if position.was_move_legal() {
                let moved_gs = position.state.get().clone();
                let mut new_hasher = ZobristHasher::new();
                new_hasher.init(&moved_gs);
                assert_eq!(
                    position.state.get_hash(),
                    new_hasher.get(),
                    "Move: {}\nBoard: {:#?}",
                    m,
                    moved_gs
                );
                recursize_test_make_unmake_move(position, move_list, depth - 1);
                position.unmake(m);
                assert_eq!(
                    original_gs,
                    *position.state.get(),
                    "Unmade move different from original\nMove: {:?}\nAfter move: {:#?}\nBefore fen: {}",
                    m,
                    moved_gs,
                    original_gs.to_fen()
                );

                let mut new_hasher = ZobristHasher::new();
                new_hasher.init(&original_gs);
                assert_eq!(
                    new_hasher.get(),
                    position.state.get_hash(),
                    "\nMove: {}\nMade move: {:#?}",
                    m,
                    moved_gs
                );
            } else {
                position.unmake(m);
            }
        }
        move_list.drop_current_ply();
    }
}
