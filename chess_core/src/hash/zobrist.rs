use lazy_static::lazy_static;
use std::array;

use itertools::Itertools;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

use crate::{
    color::Color,
    hash::Hasher,
    square::CastleSide,
    state::{State, chess_board::PieceType},
};

const SEED: u64 = 0xdeadbeef;

struct ZobristSide {
    pawn: [u64; 64],
    knight: [u64; 64],
    bishop: [u64; 64],
    rook: [u64; 64],
    queen: [u64; 64],
    king: [u64; 64],
}

impl ZobristSide {
    fn get(&self, piece: PieceType) -> &[u64; 64] {
        match piece {
            PieceType::Pawn => &self.pawn,
            PieceType::Knight => &self.knight,
            PieceType::Bishop => &self.bishop,
            PieceType::Rook => &self.rook,
            PieceType::Queen => &self.queen,
            PieceType::King => &self.king,
        }
    }

    fn new(rng: &mut ChaCha20Rng) -> Self {
        Self {
            pawn: array::from_fn(|_| rng.r#gen()),
            knight: array::from_fn(|_| rng.r#gen()),
            bishop: array::from_fn(|_| rng.r#gen()),
            rook: array::from_fn(|_| rng.r#gen()),
            queen: array::from_fn(|_| rng.r#gen()),
            king: array::from_fn(|_| rng.r#gen()),
        }
    }
}

struct ZobristBoard {
    white: ZobristSide,
    black: ZobristSide,
}

impl ZobristBoard {
    fn new(rng: &mut ChaCha20Rng) -> Self {
        Self {
            white: ZobristSide::new(rng),
            black: ZobristSide::new(rng),
        }
    }

    fn get_color(&self, color: Color) -> &ZobristSide {
        match color {
            Color::White => &self.white,
            Color::Black => &self.black,
        }
    }

    fn get(&self, color: Color, piece: PieceType) -> &[u64; 64] {
        self.get_color(color).get(piece)
    }
}

struct ZobristFlags {
    active_color: u64,
    white_king_castle_right: u64,
    white_queen_castle_right: u64,
    black_king_castle_right: u64,
    black_queen_castle_right: u64,
}

impl ZobristFlags {
    fn new(rng: &mut ChaCha20Rng) -> Self {
        Self {
            active_color: rng.r#gen(),
            white_king_castle_right: rng.r#gen(),
            white_queen_castle_right: rng.r#gen(),
            black_king_castle_right: rng.r#gen(),
            black_queen_castle_right: rng.r#gen(),
        }
    }

    fn get_castle(&self, color: Color, side: CastleSide) -> u64 {
        match (color, side) {
            (Color::White, CastleSide::King) => self.white_king_castle_right,
            (Color::White, CastleSide::Queen) => self.white_queen_castle_right,
            (Color::Black, CastleSide::King) => self.black_king_castle_right,
            (Color::Black, CastleSide::Queen) => self.black_queen_castle_right,
        }
    }
}

struct ZobristNumbers {
    pub board: ZobristBoard,
    pub flags: ZobristFlags,
    pub en_passant_file: [u64; 8],
}

impl ZobristNumbers {
    fn new() -> Self {
        let rng = &mut ChaCha20Rng::seed_from_u64(SEED);
        ZobristNumbers {
            flags: ZobristFlags::new(rng),
            board: ZobristBoard::new(rng),
            en_passant_file: array::from_fn(|_| rng.r#gen()),
        }
    }
}

lazy_static! {
    static ref ZOBRIST_NUMBERS: ZobristNumbers = ZobristNumbers::new();
}

#[derive(Clone)]
pub struct ZobristHasher {
    hash: u64,
}

impl ZobristHasher {
    pub fn new() -> Self {
        Self { hash: 0 }
    }
}

impl Default for ZobristHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for ZobristHasher {
    fn init(&mut self, state: &State) {
        self.hash = 0;
        for (color, piece) in Color::as_array()
            .into_iter()
            .cartesian_product(PieceType::as_array())
        {
            let mut b = state.boards[color][piece].clone();
            while let Some(lsb) = b.pop_first_square() {
                self.hash ^= ZOBRIST_NUMBERS.board.get(color, piece)[lsb.get() as usize]
            }
        }

        if !state.flags.active_color() == Color::White {
            self.hash ^= ZOBRIST_NUMBERS.flags.active_color;
        }

        for (color, side) in Color::as_array()
            .into_iter()
            .cartesian_product(CastleSide::as_array())
            .filter(|(color, side)| state.flags.castle_right(*color, *side))
        {
            self.hash ^= ZOBRIST_NUMBERS.flags.get_castle(color, side);
        }

        // En passant
        if let Some(lsb) = state.en_passant.get_first_square() {
            self.hash ^= ZOBRIST_NUMBERS.en_passant_file[lsb.file() as usize];
        }
    }

    fn consume_piece(&mut self, color: Color, piece: PieceType, square: crate::square::Square) {
        self.hash ^= ZOBRIST_NUMBERS.board.get(color, piece)[square.get() as usize]
    }

    fn get(&self) -> u64 {
        self.hash
    }

    fn consume_castle(&mut self, color: Color, side: CastleSide) {
        self.hash ^= ZOBRIST_NUMBERS.flags.get_castle(color, side)
    }

    fn consume_color(&mut self) {
        self.hash ^= ZOBRIST_NUMBERS.flags.active_color
    }

    fn consume_en_passant(&mut self, square: crate::square::Square) {
        self.hash ^= ZOBRIST_NUMBERS.en_passant_file[square.file() as usize]
    }
}
