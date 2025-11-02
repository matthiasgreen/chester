use lazy_static::lazy_static;
use std::ops::{Index, IndexMut};

use crate::{color::Color, square::Square, state::bitboard::BitBoard};

pub struct MoveMap([BitBoard; 64]);

impl Default for MoveMap {
    fn default() -> Self {
        Self([BitBoard::EMPTY; 64])
    }
}

impl IndexMut<Square> for MoveMap {
    fn index_mut(&mut self, index: Square) -> &mut Self::Output {
        &mut self.0[index.get() as usize]
    }
}

impl Index<Square> for MoveMap {
    type Output = BitBoard;

    fn index(&self, index: Square) -> &Self::Output {
        &self.0[index.get() as usize]
    }
}

#[derive(Clone)]
struct Offset {
    rank: i8,
    file: i8,
}

impl Offset {
    fn new(rank: i8, file: i8) -> Self {
        Self { rank, file }
    }

    fn permute_signs(self) -> [Self; 4] {
        [
            Self::new(-self.rank, -self.file),
            Self::new(-self.rank, self.file),
            Self::new(self.rank, -self.file),
            self,
        ]
        // (2, 1) => (2, 1), (1, 2), (-1, 2), (-2, 1), (-2, -1), (-1, -2), (1, -2)
        // (0, 1) => (0, 1), (1, 0), (-1, 0) (0, -1)
    }

    fn directions(value: i8) -> [Self; 4] {
        [
            Self::new(value, 0),
            Self::new(-value, 0),
            Self::new(0, value),
            Self::new(0, -value),
        ]
    }
}

impl Square {
    fn with_offset(&self, offset: &Offset) -> Option<Self> {
        let rank = u8::try_from(self.rank() as i8 + offset.rank).ok()?;
        let file = u8::try_from(self.file() as i8 + offset.file).ok()?;
        Square::new(rank, file)
    }
}

impl MoveMap {
    fn from_offsets(offsets: Vec<Offset>) -> Self {
        let mut res = MoveMap::default();
        for offset in offsets {
            for source in Square::iter() {
                if let Some(target) = source.with_offset(&offset) {
                    res[source].set(target);
                }
            }
        }
        res
    }

    fn from_direction(direction: Offset) -> Self {
        let mut res = MoveMap::default();
        for source in Square::iter() {
            let mut square = source;
            while let Some(target) = square.with_offset(&direction) {
                res[source].set(target);
                square = target;
            }
        }
        res
    }

    fn double_pawn(color: Color) -> Self {
        let mut res = MoveMap::default();
        let from_rank = match color {
            Color::White => 1,
            Color::Black => 6,
        };
        let to_rank = match color {
            Color::White => 3,
            Color::Black => 4,
        };
        for file in 0..8 {
            let from_square = Square::new(from_rank, file).unwrap();
            let to_square = Square::new(to_rank, file).unwrap();
            res[from_square].set(to_square);
        }
        res
    }
}

pub struct MoveMaps {
    pub knight: MoveMap,
    pub king: MoveMap,
    pub ne_diagonal: MoveMap,
    pub nw_diagonal: MoveMap,
    pub sw_diagonal: MoveMap,
    pub se_diagonal: MoveMap,
    pub e_rank: MoveMap,
    pub w_rank: MoveMap,
    pub n_file: MoveMap,
    pub s_file: MoveMap,

    pub white_pawn_passive: MoveMap,
    pub black_pawn_passive: MoveMap,
    pub white_pawn_double: MoveMap,
    pub black_pawn_double: MoveMap,
    pub white_pawn_attack: MoveMap,
    pub black_pawn_attack: MoveMap,
}

impl MoveMaps {
    pub fn new() -> MoveMaps {
        MoveMaps {
            knight: MoveMap::from_offsets(
                [
                    Offset::new(2, 1).permute_signs(),
                    Offset::new(1, 2).permute_signs(),
                ]
                .concat(),
            ),
            king: MoveMap::from_offsets(
                [Offset::new(1, 1).permute_signs(), Offset::directions(1)].concat(),
            ),
            ne_diagonal: MoveMap::from_direction(Offset::new(1, 1)),
            nw_diagonal: MoveMap::from_direction(Offset::new(1, -1)),
            sw_diagonal: MoveMap::from_direction(Offset::new(-1, -1)),
            se_diagonal: MoveMap::from_direction(Offset::new(-1, 1)),

            e_rank: MoveMap::from_direction(Offset::new(0, 1)),
            w_rank: MoveMap::from_direction(Offset::new(0, -1)),
            n_file: MoveMap::from_direction(Offset::new(1, 0)),
            s_file: MoveMap::from_direction(Offset::new(-1, 0)),

            white_pawn_passive: MoveMap::from_offsets(vec![Offset::new(1, 0)]),
            black_pawn_passive: MoveMap::from_offsets(vec![Offset::new(-1, 0)]),
            white_pawn_double: MoveMap::double_pawn(Color::White),
            black_pawn_double: MoveMap::double_pawn(Color::Black),
            white_pawn_attack: MoveMap::from_offsets(vec![Offset::new(1, -1), Offset::new(1, 1)]),
            black_pawn_attack: MoveMap::from_offsets(vec![Offset::new(-1, -1), Offset::new(-1, 1)]),
        }
    }

    pub fn diagonals(&self) -> [(&MoveMap, Direction); 4] {
        [
            (&self.ne_diagonal, Direction::Increasing),
            (&self.nw_diagonal, Direction::Increasing),
            (&self.sw_diagonal, Direction::Decreasing),
            (&self.se_diagonal, Direction::Decreasing),
        ]
    }

    pub fn directions(&self) -> [(&MoveMap, Direction); 4] {
        [
            (&self.e_rank, Direction::Increasing),
            (&self.n_file, Direction::Increasing),
            (&self.w_rank, Direction::Decreasing),
            (&self.s_file, Direction::Decreasing),
        ]
    }

    pub fn passive_pawn(&self, color: Color) -> &MoveMap {
        match color {
            Color::White => &self.white_pawn_passive,
            Color::Black => &self.black_pawn_passive,
        }
    }

    pub fn double_pawn(&self, color: Color) -> &MoveMap {
        match color {
            Color::White => &self.white_pawn_double,
            Color::Black => &self.black_pawn_double,
        }
    }

    pub fn attack_pawn(&self, color: Color) -> &MoveMap {
        match color {
            Color::White => &self.white_pawn_attack,
            Color::Black => &self.black_pawn_attack,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Increasing,
    Decreasing,
}

lazy_static! {
    pub static ref MOVE_MAPS: MoveMaps = MoveMaps::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_all() {
        let move_maps = MoveMaps::new();
        let index = Square::new(1, 1).unwrap();
        println!("knight:\n{}\n", move_maps.knight[index]);
        println!("king:\n{}\n", move_maps.king[index]);
        println!("ne_diagonal:\n{}\n", move_maps.ne_diagonal[index]);
        println!("nw_diagonal:\n{}\n", move_maps.nw_diagonal[index]);
        println!("sw_diagonal:\n{}\n", move_maps.sw_diagonal[index]);
        println!("se_diagonal:\n{}\n", move_maps.se_diagonal[index]);
        println!("e_rank:\n{}\n", move_maps.e_rank[index]);
        println!("w_rank:\n{}\n", move_maps.w_rank[index]);
        println!("n_file:\n{}\n", move_maps.n_file[index]);
        println!("s_file:\n{}\n", move_maps.s_file[index]);
        println!(
            "white_passive_pawn:\n{}\n",
            move_maps.white_pawn_passive[index]
        );
        println!(
            "black_passive_pawn:\n{}\n",
            move_maps.black_pawn_passive[index]
        );
        println!(
            "white_double_pawn:\n{}\n",
            move_maps.white_pawn_double[index]
        );
        println!(
            "black_double_pawn:\n{}\n",
            move_maps.black_pawn_double[index]
        );
        println!(
            "white_attack_pawn:\n{}\n",
            move_maps.white_pawn_attack[index]
        );
        println!(
            "black_attack_pawn:\n{}\n",
            move_maps.black_pawn_attack[index]
        );
    }
}
