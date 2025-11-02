#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Color {
    White = 0,
    Black = 1,
}

use std::ops::Not;

use Color::*;

impl Color {
    pub const fn from_bits(bits: u8) -> Self {
        match bits {
            0 => White,
            1 => Black,
            _ => unreachable!(),
        }
    }

    pub const fn into_bits(self) -> u8 {
        self as u8
    }

    pub const fn as_array() -> [Color; 2] {
        [White, Black]
    }
}

impl From<Color> for char {
    fn from(val: Color) -> Self {
        match val {
            White => 'w',
            Black => 'b',
        }
    }
}

impl TryFrom<char> for Color {
    type Error = ();

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'w' => Ok(White),
            'b' => Ok(Black),
            _ => Err(()),
        }
    }
}

impl Not for Color {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            White => Black,
            Black => White,
        }
    }
}
