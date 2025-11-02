use crate::{
    color::Color,
    square::{CastleSide, Square},
    state::{State, bitboard::BitBoard, chess_board::PieceType},
};

pub mod zobrist;

/// Hasher holds a hash of the current game state.
pub trait Hasher {
    fn init(&mut self, state: &State);
    fn consume_piece(&mut self, color: Color, piece: PieceType, square: Square);
    fn consume_castle(&mut self, color: Color, side: CastleSide);
    fn consume_color(&mut self);
    fn consume_en_passant(&mut self, square: Square);
    fn get(&self) -> u64;
}

#[derive(Default)]
pub struct NoopHasher {}

impl Hasher for NoopHasher {
    fn init(&mut self, _state: &State) {}

    fn get(&self) -> u64 {
        unimplemented!()
    }

    fn consume_piece(&mut self, _color: Color, _piece: PieceType, _square: Square) {}

    fn consume_castle(&mut self, _color: Color, _side: CastleSide) {}

    fn consume_color(&mut self) {}

    fn consume_en_passant(&mut self, _square: Square) {}
}

/// This struct allows read access to state, but protects state mutation with hashing
pub struct HashedState<H: Hasher> {
    state: State,
    hasher: H,
}

impl<H: Hasher + Default> Default for HashedState<H> {
    fn default() -> Self {
        Self {
            state: Default::default(),
            hasher: Default::default(),
        }
    }
}

impl<H: Hasher> HashedState<H> {
    pub fn from_fen(fen: &str, mut hasher: H) -> Self {
        let state = State::from_fen(fen);
        hasher.init(&state);
        Self { state, hasher }
    }

    pub fn new(state: State, mut hasher: H) -> Self {
        hasher.init(&state);
        Self { state, hasher }
    }

    pub fn get(&self) -> &State {
        &self.state
    }

    pub fn get_hash(&self) -> u64 {
        self.hasher.get()
    }

    /// Remove a piece and update the hash.
    pub fn remove_piece(&mut self, square: Square, piece: PieceType, color: Color) {
        self.state.boards[color][piece].unset(square);
        self.hasher.consume_piece(color, piece, square);
    }

    /// Add a piece and update the hash.
    pub fn add_piece(&mut self, square: Square, piece: PieceType, color: Color) {
        self.state.boards[color][piece].set(square);
        self.hasher.consume_piece(color, piece, square);
    }

    /// Move a piece and update the hash.
    pub fn move_piece(&mut self, from: Square, to: Square, piece: PieceType, color: Color) {
        self.state.boards[color][piece].r#move(from, to);
        self.hasher.consume_piece(color, piece, from);
        self.hasher.consume_piece(color, piece, to);
    }

    pub fn set_castle_right(&mut self, color: Color, side: CastleSide, value: bool) {
        if self.state.flags.castle_right(color, side) != value {
            self.state.flags.set_castle_right(color, side, value);
            self.hasher.consume_castle(color, side);
        }
    }

    pub fn toggle_color(&mut self) {
        // Change active color
        self.state.flags.toggle_color();
        self.hasher.consume_color();
    }

    pub fn remove_en_passant(&mut self) {
        if let Some(square) = self.state.en_passant.pop_first_square() {
            self.hasher.consume_en_passant(square);
        }
    }

    pub fn add_en_passant(&mut self, square: Square) {
        debug_assert_eq!(self.state.en_passant, BitBoard::EMPTY);
        self.state.en_passant.set(square);
        self.hasher.consume_en_passant(square);
    }

    pub fn increment_halfmove(&mut self) {
        self.state.halfmove += 1;
    }

    pub fn decrement_halfmove(&mut self) {
        self.state.halfmove -= 1;
    }
}
