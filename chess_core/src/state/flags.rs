use bitfields::bitfield;
use derive_more::BitXor;
use itertools::Itertools;

use crate::{color::Color, square::CastleSide};

#[bitfield(u8, debug = false)]
#[derive(Clone, Eq, PartialEq, BitXor)]
pub struct StateFlags {
    #[bits(1, default = Color::White)]
    active_color: Color,

    #[bits(3)]
    _padding: u8,

    #[bits(1, default = true)]
    white_king_castle_right: bool,

    #[bits(1, default = true)]
    white_queen_castle_right: bool,

    #[bits(1, default = true)]
    black_king_castle_right: bool,

    #[bits(1, default = true)]
    black_queen_castle_right: bool,
}

impl std::fmt::Debug for StateFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateFlags")
            .field("active_color", &self.active_color())
            .field(
                "castle_rights",
                &Color::as_array()
                    .into_iter()
                    .cartesian_product(CastleSide::as_array())
                    .filter(|(color, side)| self.castle_right(*color, *side))
                    .map(|(color, side)| format!("{:?}{:?}", color, side))
                    .collect_vec(),
            )
            .finish()
    }
}

impl StateFlags {
    pub fn set_castle_right(&mut self, color: Color, side: CastleSide, value: bool) {
        match (color, side) {
            (Color::White, CastleSide::King) => self.set_white_king_castle_right(value),
            (Color::White, CastleSide::Queen) => self.set_white_queen_castle_right(value),
            (Color::Black, CastleSide::King) => self.set_black_king_castle_right(value),
            (Color::Black, CastleSide::Queen) => self.set_black_queen_castle_right(value),
        }
    }

    pub fn castle_right(&self, color: Color, side: CastleSide) -> bool {
        match (color, side) {
            (Color::White, CastleSide::King) => self.white_king_castle_right(),
            (Color::White, CastleSide::Queen) => self.white_queen_castle_right(),
            (Color::Black, CastleSide::King) => self.black_king_castle_right(),
            (Color::Black, CastleSide::Queen) => self.black_queen_castle_right(),
        }
    }

    pub fn toggle_color(&mut self) {
        self.set_active_color(!self.active_color());
    }

    pub fn from_fen(active_color: char, castling_rights: &str) -> StateFlags {
        let mut flags = StateFlags::new();
        flags.set_active_color(active_color.try_into().unwrap());
        if !castling_rights.contains('K') {
            flags.set_white_king_castle_right(false);
        }
        if !castling_rights.contains('Q') {
            flags.set_white_queen_castle_right(false);
        }
        if !castling_rights.contains('k') {
            flags.set_black_king_castle_right(false);
        }
        if !castling_rights.contains('q') {
            flags.set_black_queen_castle_right(false);
        }
        flags
    }

    pub fn to_fen(&self) -> String {
        let mut castle_string = String::new();
        if self.white_king_castle_right() {
            castle_string.push('K');
        }
        if self.white_queen_castle_right() {
            castle_string.push('Q');
        }
        if self.black_king_castle_right() {
            castle_string.push('k');
        }
        if self.black_queen_castle_right() {
            castle_string.push('q');
        }
        if castle_string.is_empty() {
            castle_string = "-".to_string();
        }
        format!("{} {}", char::from(self.active_color()), castle_string)
    }
}
