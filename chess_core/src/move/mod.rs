use bitfields::bitfield;
use std::fmt::Debug;
use std::fmt::Display;

mod move_generator;
mod move_maps;

pub use move_generator::MoveGenerator;

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
