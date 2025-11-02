use std::fmt::{Debug, Display};

use derive_more::{Add, Sub};

use crate::{
    color::Color,
    state::{bitboard::BitBoard, chess_board::PieceType},
};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Add, Sub)]
pub struct Square(u8);

impl TryFrom<u8> for Square {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 64 {
            Ok(Square(value))
        } else {
            Err("Value too large")
        }
    }
}

impl TryFrom<i8> for Square {
    type Error = &'static str;

    fn try_from(value: i8) -> Result<Self, Self::Error> {
        match u8::try_from(value) {
            Ok(x) => Square::try_from(x),
            Err(_) => Err("Value too small"),
        }
    }
}

impl Square {
    pub const fn new(rank: u8, file: u8) -> Option<Self> {
        if rank < 8 && file < 8 {
            Some(Self::new_unchecked(rank, file))
        } else {
            None
        }
    }

    pub const fn new_unchecked(rank: u8, file: u8) -> Self {
        debug_assert!(rank < 8 && file < 8);
        Self(rank * 8 + file)
    }

    pub const fn get(&self) -> u8 {
        self.0
    }

    pub const fn rank(&self) -> u8 {
        self.0 / 8
    }

    pub const fn file(&self) -> u8 {
        self.0 % 8
    }

    pub const fn mirror(&self) -> Square {
        Square::new_unchecked(7 - self.rank(), self.file())
    }

    pub fn iter() -> impl Iterator<Item = Square> {
        (0..64).map(|x| Square(x))
    }

    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    pub const fn into_bits(self) -> u8 {
        self.0
    }
}

impl Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const A: u8 = 'a' as u8;
        write!(f, "{}{}", (self.file() + A) as char, self.rank() + 1)
    }
}

impl Debug for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl TryFrom<&str> for Square {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        const A: u8 = 'a' as u8;
        const H: u8 = 'h' as u8;
        const ZERO: u8 = '0' as u8;
        const EIGHT: u8 = '8' as u8;
        match *value.as_bytes() {
            [file @ A..=H, rank @ ZERO..EIGHT] => {
                Ok(Square::new_unchecked(rank - ZERO - 1, file - A))
            }
            _ => Err("Square string malformed."),
        }
    }
}

pub struct SquareFinder(pub Color);

macro_rules! square {
    ($self:ident, $rank:expr, $file:expr) => {
        $self.adapt_to_color(Square::new($rank, $file).unwrap())
    };
}

impl SquareFinder {
    fn adapt_to_color(&self, offset: Square) -> Square {
        match self.0 {
            Color::White => offset,
            Color::Black => offset.mirror(),
        }
    }

    /// Get the square the piece starts on.
    /// Returns None if multiple pieces.
    pub fn source(&self, piece: PieceType) -> Option<Square> {
        match piece {
            PieceType::Queen => Some(square!(self, 0, 3)),
            PieceType::King => Some(square!(self, 0, 4)),
            _ => return None,
        }
    }

    /// Get the square the king goes to during a castle.
    pub fn castle_king_target(&self, side: CastleSide) -> Square {
        match side {
            CastleSide::King => square!(self, 0, 6),
            CastleSide::Queen => square!(self, 0, 2),
        }
    }

    /// Get the square the rook goes to during a castle.
    pub fn castle_rook_target(&self, side: CastleSide) -> Square {
        match side {
            CastleSide::King => square!(self, 0, 5),
            CastleSide::Queen => square!(self, 0, 3),
        }
    }

    pub fn castle_rook_source(&self, side: CastleSide) -> Square {
        match side {
            CastleSide::King => square!(self, 0, 7),
            CastleSide::Queen => square!(self, 0, 0),
        }
    }

    /// Get the squares the king moves through when castling.
    pub fn castle_check(&self, side: CastleSide) -> [Square; 3] {
        match side {
            CastleSide::King => [
                square!(self, 0, 4),
                square!(self, 0, 5),
                square!(self, 0, 6),
            ],
            CastleSide::Queen => [
                square!(self, 0, 4),
                square!(self, 0, 3),
                square!(self, 0, 2),
            ],
        }
    }

    /// Get the squares that must be empty when castling.
    pub fn castle_empty(&self, side: CastleSide) -> BitBoard {
        let squares = match side {
            CastleSide::King => [
                square!(self, 0, 5),
                square!(self, 0, 5),
                square!(self, 0, 6),
            ],
            CastleSide::Queen => [
                square!(self, 0, 3),
                square!(self, 0, 2),
                square!(self, 0, 1),
            ],
        };
        BitBoard::EMPTY | squares[0].into() | squares[1].into() | squares[2].into()
    }

    pub fn en_passant_capture(&self, file: u8) -> Square {
        square!(self, 4, file)
    }

    pub fn en_passant_marker(&self, file: u8) -> Square {
        square!(self, 2, file)
    }
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum CastleSide {
    King,
    Queen,
}

impl CastleSide {
    pub const fn as_array() -> [CastleSide; 2] {
        use CastleSide::*;
        [King, Queen]
    }
}
