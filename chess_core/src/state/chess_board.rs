use std::ops::{Index, IndexMut};

use crate::{color::Color, square::Square, state::bitboard::BitBoard};

/// Enum representing the type of a piece.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

use PieceType::*;

impl PieceType {
    pub const fn as_array() -> [PieceType; 6] {
        [Pawn, Knight, Bishop, Rook, Queen, King]
    }
}

impl From<PieceType> for char {
    fn from(value: PieceType) -> Self {
        match value {
            Pawn => 'P',
            Knight => 'N',
            Bishop => 'B',
            Rook => 'R',
            Queen => 'Q',
            King => 'K',
        }
    }
}

/// A struct that gathers all the bitboards for each piece type for one color.
#[derive(Clone, PartialEq, Debug)]
pub struct ChessBoardSide {
    pub pawn: BitBoard,
    pub knight: BitBoard,
    pub bishop: BitBoard,
    pub rook: BitBoard,
    pub queen: BitBoard,
    pub king: BitBoard,
}

impl IndexMut<PieceType> for ChessBoardSide {
    fn index_mut(&mut self, index: PieceType) -> &mut Self::Output {
        match index {
            PieceType::Pawn => &mut self.pawn,
            PieceType::Knight => &mut self.knight,
            PieceType::Bishop => &mut self.bishop,
            PieceType::Rook => &mut self.rook,
            PieceType::Queen => &mut self.queen,
            PieceType::King => &mut self.king,
        }
    }
}

impl Index<PieceType> for ChessBoardSide {
    type Output = BitBoard;

    fn index(&self, index: PieceType) -> &Self::Output {
        match index {
            PieceType::Pawn => &self.pawn,
            PieceType::Knight => &self.knight,
            PieceType::Bishop => &self.bishop,
            PieceType::Rook => &self.rook,
            PieceType::Queen => &self.queen,
            PieceType::King => &self.king,
        }
    }
}

impl ChessBoardSide {
    pub const EMPTY: Self = Self {
        pawn: BitBoard::EMPTY,
        knight: BitBoard::EMPTY,
        bishop: BitBoard::EMPTY,
        rook: BitBoard::EMPTY,
        queen: BitBoard::EMPTY,
        king: BitBoard::EMPTY,
    };

    pub fn union(&self) -> BitBoard {
        self.pawn | self.knight | self.bishop | self.rook | self.queen | self.king
    }

    pub fn as_array(&self) -> [(&BitBoard, PieceType); 6] {
        [
            (&self.pawn, PieceType::Pawn),
            (&self.knight, PieceType::Knight),
            (&self.bishop, PieceType::Bishop),
            (&self.rook, PieceType::Rook),
            (&self.queen, PieceType::Queen),
            (&self.king, PieceType::King),
        ]
    }

    pub fn as_array_mut(&mut self) -> [(&mut BitBoard, PieceType); 6] {
        [
            (&mut self.pawn, PieceType::Pawn),
            (&mut self.knight, PieceType::Knight),
            (&mut self.bishop, PieceType::Bishop),
            (&mut self.rook, PieceType::Rook),
            (&mut self.queen, PieceType::Queen),
            (&mut self.king, PieceType::King),
        ]
    }
}

/// A struct that gathers all the bitboards for each piece type for both colors.
#[derive(Clone, PartialEq)]
pub struct ChessBoard {
    pub white: ChessBoardSide,
    pub black: ChessBoardSide,
}

impl IndexMut<Color> for ChessBoard {
    fn index_mut(&mut self, index: Color) -> &mut Self::Output {
        match index {
            Color::White => &mut self.white,
            Color::Black => &mut self.black,
        }
    }
}

impl Index<Color> for ChessBoard {
    type Output = ChessBoardSide;

    fn index(&self, index: Color) -> &Self::Output {
        match index {
            Color::White => &self.white,
            Color::Black => &self.black,
        }
    }
}

impl ChessBoard {
    pub const EMPTY: ChessBoard = ChessBoard {
        white: ChessBoardSide::EMPTY,
        black: ChessBoardSide::EMPTY,
    };

    pub fn from_fen(board: &str) -> Self {
        let mut boards = ChessBoard::EMPTY;

        for (line, rank) in board.split('/').rev().zip(0_u8..) {
            let mut file = 0_u8;
            for c in line.chars() {
                if c.is_ascii_digit() {
                    file += c.to_digit(10).unwrap() as u8;
                } else {
                    let color_board = if c.is_uppercase() {
                        &mut boards.white
                    } else {
                        &mut boards.black
                    };
                    let bb = match c.to_ascii_lowercase() {
                        'p' => &mut color_board.pawn,
                        'n' => &mut color_board.knight,
                        'b' => &mut color_board.bishop,
                        'r' => &mut color_board.rook,
                        'q' => &mut color_board.queen,
                        'k' => &mut color_board.king,
                        _ => panic!("Invalid piece type"),
                    };
                    bb.set(Square::new_unchecked(rank, file));
                    file += 1;
                }
            }
        }
        boards
    }

    pub fn to_fen(&self) -> String {
        let mut board_str = String::new();
        for i in (0..8).rev() {
            let mut empty = 0;
            for j in 0..8 {
                let mut found = false;
                for (board, piece) in self.white.as_array() {
                    if board.get(Square::new_unchecked(i, j)) {
                        if empty > 0 {
                            board_str.push_str(&empty.to_string());
                            empty = 0;
                        }
                        board_str.push(piece.into());
                        found = true;
                        break;
                    }
                }
                for (board, piece) in self.black.as_array() {
                    if board.get(Square::new_unchecked(i, j)) {
                        if empty > 0 {
                            board_str.push_str(&empty.to_string());
                            empty = 0;
                        }
                        board_str.push(char::from(piece).to_ascii_lowercase());
                        found = true;
                        break;
                    }
                }
                if !found {
                    empty += 1;
                }
            }
            if empty > 0 {
                board_str.push_str(&empty.to_string());
            }
            if i > 0 {
                board_str.push('/');
            }
        }
        board_str
    }
}

impl std::fmt::Debug for ChessBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let white_board = self.white.as_array();
        let black_board = self.black.as_array();
        let mut board_str = String::new();
        board_str.push('\n');
        for i in (0..8).rev() {
            for j in 0..8 {
                let mut found = false;
                for (board, piece) in white_board {
                    if board.get(Square::new_unchecked(i, j)) {
                        board_str.push(piece.into());
                        found = true;
                        break;
                    }
                }
                for (board, piece) in black_board {
                    if board.get(Square::new_unchecked(i, j)) {
                        board_str.push(char::from(piece).to_ascii_lowercase());
                        found = true;
                        break;
                    }
                }
                if !found {
                    board_str.push('.');
                }
            }
            board_str.push('\n');
        }
        f.write_str(board_str.as_str())
    }
}
