use crate::{
    Insert,
    color::Color,
    r#move::{
        MoveCode,
        move_maps::{Direction, MOVE_MAPS},
    },
    square::{CastleSide, Square, SquareFinder},
    state::{State, bitboard::BitBoard, chess_board::PieceType},
};

use super::Move;

pub struct MoveGenerator<'a>(&'a State);

impl<'a> MoveGenerator<'a> {
    pub fn new(state: &'a State) -> Self {
        Self(state)
    }

    pub fn pseudo_legal_moves<T: Insert<Move>>(&self, out: &mut T) {
        self.knight_moves(out);
        self.king_moves(out);
        self.diagonal_moves(out, PieceType::Bishop);
        self.diagonal_moves(out, PieceType::Queen);
        self.rank_file_moves(out, PieceType::Rook);
        self.rank_file_moves(out, PieceType::Queen);
        self.pawn_moves(out);
        self.castle_moves(out);
    }

    fn knight_moves<T: Insert<Move>>(&self, out: &mut T) {
        let mut temp_knights = self.0.active_boards()[PieceType::Knight].clone();
        let friendly_occupation = self.0.active_boards().union();
        let enemy_occupation = self.0.inactive_boards().union();

        while let Some(knight) = temp_knights.pop_first_square() {
            let to_board = MOVE_MAPS.knight[knight] & !friendly_occupation;
            // Remove any moves that are occupied by friendly pieces
            // Check for captures
            let mut to_capture = to_board & enemy_occupation;
            let mut to_quiet = to_board & !enemy_occupation;

            while let Some(enemy) = to_capture.pop_first_square() {
                out.insert(Move::new(knight, enemy, MoveCode::Capture));
            }
            while let Some(to) = to_quiet.pop_first_square() {
                out.insert(Move::new(knight, to, MoveCode::QuietMove));
            }
        }
    }

    fn king_moves<T: Insert<Move>>(&self, out: &mut T) {
        let king = self.0.active_boards()[PieceType::King]
            .get_first_square()
            .unwrap();
        let friendly_occupation = self.0.active_boards().union();
        let enemy_occupation = self.0.inactive_boards().union();

        let to_board = MOVE_MAPS.king[king] & !friendly_occupation;
        let mut to_capture = to_board & enemy_occupation;
        let mut to_quiet = to_board & !enemy_occupation;

        while let Some(to) = to_capture.pop_first_square() {
            out.insert(Move::new(king, to, MoveCode::Capture));
        }
        while let Some(to) = to_quiet.pop_first_square() {
            out.insert(Move::new(king, to, MoveCode::QuietMove));
        }
    }

    fn diagonal_moves<T: Insert<Move>>(&self, out: &mut T, piece: PieceType) {
        let mut board = self.0.active_boards()[piece];
        while let Some(square) = board.pop_first_square() {
            for (move_map, direction) in MOVE_MAPS.diagonals() {
                self.direction_moves(out, square, move_map[square], direction);
            }
        }
    }

    fn rank_file_moves<T: Insert<Move>>(&self, out: &mut T, piece: PieceType) {
        let mut board = self.0.active_boards()[piece].clone();
        while let Some(square) = board.pop_first_square() {
            for (move_map, direction) in MOVE_MAPS.directions() {
                self.direction_moves(out, square, move_map[square], direction);
            }
        }
    }

    fn direction_moves<T: Insert<Move>>(
        &self,
        out: &mut T,
        from: Square,
        direction: BitBoard,
        direction_type: Direction,
    ) {
        let friendly_occupation = self.0.active_boards().union();
        let enemy_occupation = self.0.inactive_boards().union();

        let get_square = |b: BitBoard| -> Option<Square> {
            match direction_type {
                Direction::Increasing => b.get_first_square(),
                Direction::Decreasing => b.get_last_square(),
            }
        };
        let friendly_block = get_square(friendly_occupation & direction);
        let enemy_sb = get_square(enemy_occupation & direction);

        // Blocking sb is the index up to which we can add quiet moves
        let (blocking_sb, capture_square) = match (friendly_block, enemy_sb) {
            (None, None) => (None, None),
            (None, Some(enemy)) => (Some(enemy), Some(enemy)),
            (Some(friendly), None) => (Some(friendly), None),
            (Some(friendly), Some(enemy)) => {
                if (direction_type == Direction::Increasing && enemy < friendly)
                    || (direction_type == Direction::Decreasing && enemy > friendly)
                {
                    (Some(enemy), Some(enemy))
                } else {
                    (Some(friendly), None)
                }
            }
        };
        if let Some(capture) = capture_square {
            out.insert(Move::new(from, capture, MoveCode::Capture));
        }

        // to_board is the board of all moves in the direction that haven't been added yet.
        let mut to_board = direction;

        // While there are still moves to add and we haven't reached the blocking piece
        match direction_type {
            Direction::Increasing => {
                while let Some(to) = to_board.pop_first_square()
                    && (blocking_sb.is_none_or(|b| to < b))
                {
                    out.insert(Move::new(from, to, MoveCode::QuietMove));
                }
            }
            Direction::Decreasing => {
                while let Some(to) = to_board.pop_last_square()
                    && (blocking_sb.is_none_or(|b| to > b))
                {
                    out.insert(Move::new(from, to, MoveCode::QuietMove));
                }
            }
        }
    }

    fn pawn_moves<T: Insert<Move>>(&self, out: &mut T) {
        let mut pawns = self.0.active_boards()[PieceType::Pawn];
        let color = self.0.flags.active_color();
        let friendly_occupation = self.0.active_boards().union();
        let enemy_occupation = self.0.inactive_boards().union();

        let unoccupied = !(friendly_occupation | enemy_occupation);

        while let Some(from) = pawns.pop_first_square() {
            let will_promote = match color {
                Color::White => from.rank() == 6,
                Color::Black => from.rank() == 1,
            };
            let mut passive_board = MOVE_MAPS.passive_pawn(color)[from] & unoccupied;
            let mut double_board = MOVE_MAPS.double_pawn(color)[from] & unoccupied;

            // Cannot double push pawn if can't single push
            double_board &= if color == Color::White {
                passive_board << 8
            } else {
                passive_board >> 8
            };

            let mut attack_board =
                MOVE_MAPS.attack_pawn(color)[from] & (enemy_occupation | self.0.en_passant);

            if let Some(to) = passive_board.pop_first_square() {
                if will_promote {
                    out.insert(Move::new(from, to, MoveCode::QueenPromotion));
                    out.insert(Move::new(from, to, MoveCode::RookPromotion));
                    out.insert(Move::new(from, to, MoveCode::BishopPromotion));
                    out.insert(Move::new(from, to, MoveCode::KnightPromotion));
                } else {
                    out.insert(Move::new(from, to, MoveCode::QuietMove));
                }
            }

            if let Some(to) = double_board.pop_first_square() {
                out.insert(Move::new(from, to, MoveCode::DoublePawnPush));
            }

            while let Some(to) = attack_board.pop_first_square() {
                if will_promote {
                    out.insert(Move::new(from, to, MoveCode::QueenPromotionCapture));
                    out.insert(Move::new(from, to, MoveCode::RookPromotionCapture));
                    out.insert(Move::new(from, to, MoveCode::BishopPromotionCapture));
                    out.insert(Move::new(from, to, MoveCode::KnightPromotionCapture));
                } else if self.0.en_passant.get_first_square() == Some(to) {
                    out.insert(Move::new(from, to, MoveCode::EnPassant));
                } else {
                    out.insert(Move::new(from, to, MoveCode::Capture));
                }
            }
        }
    }

    fn castle_moves<T: Insert<Move>>(&self, out: &mut T) {
        let color = self.0.flags.active_color();
        let friendly_occupation = self.0.active_boards().union();
        let enemy_occupation = self.0.inactive_boards().union();
        let occupation = friendly_occupation | enemy_occupation;

        for side in CastleSide::as_array() {
            let finder = SquareFinder(color);
            if self.0.flags.castle_right(color, side)
                && (occupation & finder.castle_empty(side)).is_empty()
                && finder
                    .castle_check(side)
                    .iter()
                    .all(|s| !self.0.is_square_attacked(*s, !color))
            {
                out.insert(Move::new(
                    finder.source(PieceType::King).unwrap(),
                    finder.castle_king_target(side),
                    MoveCode::from_castle(side),
                ));
            }
        }
    }
}

impl State {
    pub fn was_move_legal(&self) -> bool {
        !self.is_square_attacked(
            self.inactive_boards().king.get_first_square().unwrap(),
            self.flags.active_color(),
        )
    }

    /// Checks if the king of the active player is in check
    pub fn is_check(&self) -> bool {
        !self.is_square_attacked(
            self.active_boards().king.get_first_square().unwrap(),
            !self.flags.active_color(),
        )
    }

    /// Check if active color attacking a square.
    /// There may be a piece on the square, but it will not consider en passant for pawns
    pub fn is_square_attacked(&self, square: Square, attacking_color: Color) -> bool {
        let attacking_pieces = &self.boards[attacking_color];
        let defending_pieces = &self.boards[!attacking_color];
        let defending_occupation = defending_pieces.union();

        let blocking_bishop_queen_rook = defending_occupation
            | attacking_pieces.pawn
            | attacking_pieces.knight
            | attacking_pieces.king;
        let blocking_bishop_queen = blocking_bishop_queen_rook | attacking_pieces.rook;
        let blocking_rook_queen = blocking_bishop_queen_rook | attacking_pieces.bishop;

        // Start with bishops and queens
        let attacking_bishops_and_queens = attacking_pieces.bishop | attacking_pieces.queen;

        if MOVE_MAPS.diagonals().iter().any(|(map, direction)| {
            capture_in_direction(
                map[square],
                *direction,
                attacking_bishops_and_queens,
                blocking_bishop_queen,
            )
        }) {
            return true;
        }

        // Rooks and queens
        let attacking_rooks_and_queens = attacking_pieces.rook | attacking_pieces.queen;

        if MOVE_MAPS.directions().iter().any(|(map, direction)| {
            capture_in_direction(
                map[square],
                *direction,
                attacking_rooks_and_queens,
                blocking_rook_queen,
            )
        }) {
            return true;
        }

        // Pawns
        // FIXME: Not sure about color here.
        if !(attacking_pieces.pawn & MOVE_MAPS.attack_pawn(!attacking_color)[square]).is_empty() {
            return true;
        }

        // Knights
        if !(MOVE_MAPS.knight[square] & attacking_pieces.knight).is_empty() {
            return true;
        }

        // Kings
        if !(MOVE_MAPS.king[square] & attacking_pieces.king).is_empty() {
            return true;
        }

        false
    }
}

fn capture_in_direction(
    direction: BitBoard,
    direction_type: Direction,
    targets: BitBoard,
    blocking: BitBoard,
) -> bool {
    match direction_type {
        Direction::Increasing => capture_in_increasing_direction(direction, targets, blocking),
        Direction::Decreasing => capture_in_decreasing_direction(direction, targets, blocking),
    }
}

fn capture_in_increasing_direction(
    direction: BitBoard,
    targets: BitBoard,
    blocking: BitBoard,
) -> bool {
    let friendly_sb = (direction & blocking).get_first_square();
    let target_sb = (direction & targets).get_first_square();
    match (friendly_sb, target_sb) {
        (None, Some(_)) => true,
        (Some(f), Some(t)) if t < f => true,
        _ => false,
    }
}

fn capture_in_decreasing_direction(
    direction: BitBoard,
    targets: BitBoard,
    blocking: BitBoard,
) -> bool {
    let friendly_sb = (direction & blocking).get_last_square();
    let target_sb = (direction & targets).get_last_square();
    match (friendly_sb, target_sb) {
        (None, Some(_)) => true,
        (Some(f), Some(t)) if t > f => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        collections::{MoveList, Plys},
        hash::NoopHasher,
        position::Position,
    };

    #[test]
    fn test_pseudo_legal_moves_from_starting_position() {
        let position = Position::<NoopHasher>::default();
        let mut move_list = MoveList::new();
        move_list.new_ply();
        position.pseudo_legal_moves(&mut move_list);
        let n_moves = move_list
            .current_ply()
            .iter()
            .inspect(|m| println!("{:?}", m))
            .count();
        assert_eq!(n_moves, 20);
    }
}
