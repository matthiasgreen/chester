use std::time::Instant;

use chess_core::{hash::zobrist::ZobristHasher, r#move::Move, position::Position};

pub trait Engine {
    fn select(&mut self, pos: &mut Position<ZobristHasher>, deadline: Instant) -> Option<Move>;
    fn clear(&mut self);
}
