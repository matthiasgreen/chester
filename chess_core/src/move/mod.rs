use bitfields::bitfield;
use std::fmt::Debug;
use std::fmt::Display;

mod move_generator;
mod move_maps;

pub use move_generator::MoveGenerator;

use crate::Insert;
use crate::square::CastleSide;
use crate::{square::Square, state::chess_board::PieceType};

#[bitfield(u16, new = false, debug = false)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Move {
    #[bits(6)]
    pub from: Square,

    #[bits(6)]
    pub to: Square,

    #[bits(4, default = MoveCode::QuietMove)]
    pub code: MoveCode,
}

impl Move {
    pub fn new(from: Square, to: Square, code: MoveCode) -> Move {
        MoveBuilder::new()
            .with_from(from)
            .with_to(to)
            .with_code(code)
            .build()
    }

    pub fn matches_perft_string(self, string: &str) -> bool {
        format!("{}{}", self.from(), self.to()) == string
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format: source, target, promotion (a7b8Q)
        if let Some(promotion) = self.code().as_promotion() {
            write!(f, "{}{}{}", self.from(), self.to(), char::from(promotion))
        } else {
            write!(f, "{}{}", self.from(), self.to())
        }
    }
}

impl Debug for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Move")
            .field("from", &self.from())
            .field("to", &self.to())
            .field("code", &self.code())
            .finish()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MoveCode {
    QuietMove = 0,
    DoublePawnPush = 1,
    KingCastle = 2,
    QueenCastle = 3,
    Capture = 4,
    EnPassant = 5,
    KnightPromotion = 6,
    BishopPromotion = 7,
    RookPromotion = 8,
    QueenPromotion = 9,
    KnightPromotionCapture = 10,
    BishopPromotionCapture = 11,
    RookPromotionCapture = 12,
    QueenPromotionCapture = 13,
}

use MoveCode::*;

impl MoveCode {
    const fn from_bits(bits: u8) -> Self {
        match bits {
            0 => QuietMove,
            1 => DoublePawnPush,
            2 => KingCastle,
            3 => QueenCastle,
            4 => Capture,
            5 => EnPassant,
            6 => KnightPromotion,
            7 => BishopPromotion,
            8 => RookPromotion,
            9 => QueenPromotion,
            10 => KnightPromotionCapture,
            11 => BishopPromotionCapture,
            12 => RookPromotionCapture,
            13 => QueenPromotionCapture,
            _ => unreachable!(),
        }
    }

    const fn into_bits(self) -> u8 {
        self as u8
    }

    pub fn is_capture(&self) -> bool {
        match *self {
            Capture
            | EnPassant
            | KnightPromotionCapture
            | BishopPromotionCapture
            | RookPromotionCapture
            | QueenPromotionCapture => true,
            _ => false,
        }
    }

    pub fn as_promotion(&self) -> Option<PieceType> {
        use PieceType::*;
        match *self {
            KnightPromotion | KnightPromotionCapture => Some(Knight),
            BishopPromotion | BishopPromotionCapture => Some(Bishop),
            RookPromotion | RookPromotionCapture => Some(Rook),
            QueenPromotion | QueenPromotionCapture => Some(Queen),
            _ => None,
        }
    }

    pub fn as_castle(&self) -> Option<CastleSide> {
        match *self {
            KingCastle => Some(CastleSide::King),
            QueenCastle => Some(CastleSide::Queen),
            _ => None,
        }
    }

    pub fn is_quiet(&self) -> bool {
        match *self {
            QuietMove | DoublePawnPush | KingCastle | QueenCastle => true,
            _ => false,
        }
    }

    pub fn from_castle(side: CastleSide) -> MoveCode {
        match side {
            CastleSide::King => MoveCode::KingCastle,
            CastleSide::Queen => MoveCode::QueenCastle,
        }
    }
}

pub trait AddMove {
    fn add_move_to_ply(&mut self, m: Move);
}

pub struct MoveList {
    moves: [Move; 2048],
    ply_first_move: [usize; 128], // Index of the first move for a given ply
    current_ply: usize,
    total_count: usize,
}

impl Insert<Move> for MoveList {
    fn insert(&mut self, m: Move) {
        assert!(self.current_ply != 0);
        self.moves[self.total_count] = m;
        self.total_count += 1;
    }
}

impl AddMove for Vec<Move> {
    fn add_move_to_ply(&mut self, m: Move) {
        self.push(m);
    }
}

impl MoveList {
    pub fn new() -> MoveList {
        MoveList {
            moves: [Move(0); 2048],
            ply_first_move: [0; 128],
            current_ply: 0,
            total_count: 0,
        }
    }

    pub fn ply_number(&self) -> usize {
        self.current_ply
    }

    pub fn ply_size(&self, ply: usize) -> usize {
        assert!(ply != 0);
        let first_move_index = self.ply_first_move[ply];
        let next_ply_first_move = if ply == self.current_ply {
            self.total_count
        } else {
            self.ply_first_move[ply + 1]
        };
        next_ply_first_move - first_move_index
    }

    pub fn r#move(&self, ply: usize, index: usize) -> Move {
        assert!(ply != 0);
        assert!(ply <= self.current_ply);
        assert!(index < self.ply_size(ply));
        let first_move_index = self.ply_first_move[ply];
        self.moves[first_move_index + index]
    }

    pub fn new_ply(&mut self) {
        self.current_ply += 1;
        self.ply_first_move[self.current_ply] = self.total_count;
    }

    pub fn current_ply(&self) -> &[Move] {
        assert!(self.current_ply != 0);
        let first_move_index = self.ply_first_move[self.current_ply];
        &self.moves[first_move_index..self.total_count]
    }

    pub fn current_ply_mut(&mut self) -> &mut [Move] {
        assert!(self.current_ply != 0);
        let first_move_index = self.ply_first_move[self.current_ply];
        &mut self.moves[first_move_index..self.total_count]
    }

    pub fn drop_current_ply(&mut self) {
        assert!(self.current_ply != 0);
        self.total_count = self.ply_first_move[self.current_ply];
        self.current_ply -= 1;
    }

    /// "Sorts" the ply in place
    ///
    /// Optional first move is placed first in the ply
    ///
    /// Loud moves are placed before quiet moves
    pub fn order_ply(&mut self, first: Option<Move>) {
        // Selection sort
        let ply = self.current_ply_mut();

        let mut sorted_index = 0;

        // Place first move at the start
        if let Some(first) = first {
            for i in 0..ply.len() {
                if ply[i] == first {
                    ply.swap(i, 0);
                    sorted_index += 1;
                    break;
                }
            }
        }

        // Put loud moves before quiet moves
        let range = sorted_index..ply.len();
        for i in range {
            if !ply[i].code().is_quiet() {
                ply.swap(i, sorted_index);
                sorted_index += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_list() {
        let first_ply_moves = [
            Move::new(
                Square::new(0, 0).unwrap(),
                Square::new(0, 1).unwrap(),
                MoveCode::QuietMove,
            ),
            Move::new(
                Square::new(0, 0).unwrap(),
                Square::new(0, 2).unwrap(),
                MoveCode::QuietMove,
            ),
            Move::new(
                Square::new(0, 0).unwrap(),
                Square::new(0, 3).unwrap(),
                MoveCode::QuietMove,
            ),
        ];
        let second_ply_moves = [
            Move::new(
                Square::new(0, 0).unwrap(),
                Square::new(0, 4).unwrap(),
                MoveCode::QuietMove,
            ),
            Move::new(
                Square::new(0, 0).unwrap(),
                Square::new(0, 5).unwrap(),
                MoveCode::Capture,
            ),
            Move::new(
                Square::new(0, 0).unwrap(),
                Square::new(0, 6).unwrap(),
                MoveCode::QuietMove,
            ),
        ];

        let mut move_list = MoveList::new();
        move_list.new_ply();
        for m in first_ply_moves {
            move_list.insert(m);
        }
        assert_eq!(move_list.current_ply(), &first_ply_moves);
        assert_eq!(move_list.moves[0], first_ply_moves[0]);
        assert_eq!(move_list.current_ply, 1);
        assert_eq!(move_list.total_count, 3);

        move_list.new_ply();
        for m in second_ply_moves {
            move_list.insert(m);
        }
        assert_eq!(move_list.current_ply(), &second_ply_moves);
        assert_eq!(move_list.moves[3], second_ply_moves[0]);
        assert_eq!(move_list.current_ply, 2);
        assert_eq!(move_list.total_count, 6);

        move_list.order_ply(Some(second_ply_moves[2]));
        assert_eq!(
            move_list.current_ply(),
            &[
                second_ply_moves[2],
                second_ply_moves[1],
                second_ply_moves[0]
            ]
        );

        move_list.drop_current_ply();
        assert_eq!(move_list.current_ply(), &first_ply_moves);
        assert_eq!(move_list.current_ply, 1);
        assert_eq!(move_list.total_count, 3);
    }
}
