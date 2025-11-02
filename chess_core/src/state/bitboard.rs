use std::fmt::Debug;
use std::fmt::Display;

use derive_more::{BitAnd, BitAndAssign, BitOr, BitOrAssign, From, Not, Shl, Shr};

use crate::square::Square;

/// A bitboard is a 64-bit integer that represents a set of pieces on a chess board.
/// Import the BitBoardExt trait to use some convenient methods.
#[derive(
    Copy,
    Clone,
    From,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Not,
    BitOr,
    BitAnd,
    BitOrAssign,
    BitAndAssign,
    Shl,
    Shr,
)]
pub struct BitBoard(u64);

impl BitBoard {
    pub const EMPTY: Self = Self(0);
    pub const FULL: Self = Self(0xFFFF_FFFF_FFFF_FFFF);

    pub fn is_empty(&self) -> bool {
        *self == BitBoard::EMPTY
    }

    pub fn set(&mut self, square: Square) {
        self.0 |= 1 << square.get()
    }

    pub fn unset(&mut self, square: Square) {
        // debug_assert_ne!(self.0 & (1 << square.get()), 0);
        self.0 &= !(1 << square.get())
    }

    pub fn r#move(&mut self, from: Square, to: Square) {
        self.unset(from);
        self.set(to);
    }

    pub fn toggle(&mut self, square: Square) {
        self.0 ^= 1 << square.get()
    }

    pub fn get(&self, square: Square) -> bool {
        self.0 & (1 << square.get()) != 0
    }

    pub fn pop_first_square(&mut self) -> Option<Square> {
        if self.is_empty() {
            None
        } else {
            let lsb = self.0.trailing_zeros() as u8;
            self.0 &= !(1 << lsb);
            Some(Square::try_from(lsb).unwrap())
        }
    }

    pub fn pop_last_square(&mut self) -> Option<Square> {
        if self.is_empty() {
            None
        } else {
            let msb = 63 - self.0.leading_zeros() as u8;
            self.0 &= !(1 << msb);
            Some(Square::try_from(msb).unwrap())
        }
    }

    pub fn get_first_square(&self) -> Option<Square> {
        if self.is_empty() {
            None
        } else {
            Some(Square::try_from(self.0.trailing_zeros() as u8).unwrap())
        }
    }

    pub fn get_last_square(&self) -> Option<Square> {
        if self.is_empty() {
            None
        } else {
            Some(Square::try_from(63 - self.0.leading_zeros() as u8).unwrap())
        }
    }

    pub fn file(number: u8) -> Self {
        debug_assert!(number < 8);
        BitBoard(0x0101_0101_0101_0101_u64 << number)
    }

    pub fn rank(number: u8) -> Self {
        debug_assert!(number < 8);
        BitBoard(0xFF_u64 << (number * 8))
    }

    pub fn count_ones(&self) -> u32 {
        self.0.count_ones()
    }

    pub fn trailing_zeros(&self) -> u32 {
        self.0.trailing_zeros()
    }
}

impl Display for BitBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        let flipped = self.0.reverse_bits();
        for i in 0..8 {
            let shift = 8 * i;
            let rank = (flipped & (0xFF << shift)) >> shift;
            write!(f, "{}", &format!("{:#010b}\n", rank)[2..11])?;
        }
        Ok(())
    }
}

impl Debug for BitBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl TryFrom<BitBoard> for Square {
    type Error = &'static str;

    fn try_from(value: BitBoard) -> Result<Self, Self::Error> {
        if value.count_ones() != 1 {
            Err("Bitboard must have exactly 1 one to convert to square.")
        } else {
            Ok(value.get_first_square().unwrap())
        }
    }
}

impl From<Square> for BitBoard {
    fn from(value: Square) -> Self {
        let mut res = BitBoard::EMPTY;
        res.set(value);
        res
    }
}

// impl Shl<u8> for BitBoard {
//     type Output = BitBoard;

//     fn shl(self, rhs: u8) -> Self::Output {
//         BitBoard(self.0 << rhs)
//     }
// }

// impl Shr<u8> for BitBoard {
//     type Output;

//     fn shr(self, rhs: u8) -> Self::Output {
//         todo!()
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pop_lsb() {
        let mut bb = BitBoard(0b1100);
        assert_eq!(
            bb.pop_first_square().unwrap(),
            Square::try_from(2_u8).unwrap()
        );
        assert_eq!(bb, 0b1000.into());
    }

    #[test]
    fn test_pop_msb() {
        let mut bb = BitBoard(0b1100);
        assert_eq!(
            bb.pop_last_square().unwrap(),
            Square::try_from(3_u8).unwrap()
        );
        assert_eq!(bb, 0b0100.into());
    }

    #[test]
    fn test_get_lsb() {
        let bb = BitBoard(0b1100);
        println!("{}", 0_u64.trailing_zeros());
        assert_eq!(
            bb.get_first_square().unwrap(),
            Square::try_from(2_u8).unwrap()
        );
        // assert_eq!(0.get_lsb(), 64);
    }

    #[test]
    fn test_get_msb() {
        let bb = BitBoard(0b1100);
        assert_eq!(
            bb.get_last_square().unwrap(),
            Square::try_from(3_u8).unwrap()
        );
        assert_eq!(
            BitBoard(1).get_last_square().unwrap(),
            Square::try_from(0_u8).unwrap()
        );
    }
}
