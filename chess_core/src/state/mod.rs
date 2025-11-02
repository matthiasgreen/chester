pub mod bitboard;
pub mod chess_board;
pub mod flags;

use crate::{
    color::Color,
    square::Square,
    state::{
        bitboard::BitBoard,
        chess_board::{ChessBoard, ChessBoardSide},
        flags::StateFlags,
    },
};

#[derive(Clone, PartialEq)]
pub struct State {
    pub boards: ChessBoard,
    pub en_passant: BitBoard,
    pub flags: StateFlags,
    pub halfmove: u8,
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("boards", &self.boards)
            .field(
                "en_passant_file",
                &self.en_passant.get_first_square().map(|s| s.file()),
            )
            .field("flags", &self.flags)
            .field("halfmove", &self.halfmove)
            .finish()
    }
}

impl Default for State {
    fn default() -> Self {
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }
}

impl State {
    pub fn from_fen(fen: &str) -> Self {
        let mut split = fen.split_whitespace();
        let board_str = split.next().unwrap();
        let active_color = split.next().unwrap();
        let castling = split.next().unwrap();
        let en_passant = split.next().unwrap();
        let halfmove = split.next().unwrap_or("0");

        // let _fullmove = split.next().unwrap();
        let boards = ChessBoard::from_fen(board_str);
        let flags = StateFlags::from_fen(active_color.chars().nth(0).unwrap(), castling);
        let en_passant = match en_passant {
            "-" => BitBoard::EMPTY,
            s => BitBoard::from(Square::try_from(s).unwrap()),
        };
        let halfmove: u8 = halfmove.parse().unwrap();
        State {
            boards,
            en_passant,
            flags,
            halfmove,
        }
    }

    pub fn to_fen(&self) -> String {
        let board_str = self.boards.to_fen();
        let flags = self.flags.to_fen();
        let en_passant = match self.en_passant {
            BitBoard::EMPTY => "-".to_string(),
            bb => Square::try_from(bb).unwrap().to_string(),
        };

        format!("{} {} {} {} 1", board_str, flags, en_passant, self.halfmove)
    }

    pub fn inactive_boards(&self) -> &ChessBoardSide {
        match self.flags.active_color() {
            Color::White => &self.boards.black,
            Color::Black => &self.boards.white,
        }
    }

    pub fn active_boards(&self) -> &ChessBoardSide {
        match self.flags.active_color() {
            Color::White => &self.boards.white,
            Color::Black => &self.boards.black,
        }
    }

    pub fn inactive_boards_mut(&mut self) -> &mut ChessBoardSide {
        match self.flags.active_color() {
            Color::White => &mut self.boards.black,
            Color::Black => &mut self.boards.white,
        }
    }

    pub fn active_boards_mut(&mut self) -> &mut ChessBoardSide {
        match self.flags.active_color() {
            Color::White => &mut self.boards.white,
            Color::Black => &mut self.boards.black,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_fen() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let gs = State::from_fen(fen);
        assert_eq!(gs.boards.white.pawn, BitBoard::rank(1));
        assert_eq!(gs.boards.white.knight, 0b0100_0010.into());
        assert_eq!(gs.halfmove, 0);
        assert_eq!(gs.flags.active_color(), Color::White);
        assert!(
            gs.flags.white_king_castle_right()
                && gs.flags.white_queen_castle_right()
                && gs.flags.black_king_castle_right()
                && gs.flags.black_queen_castle_right()
        );
    }

    #[test]
    fn test_to_fen() {
        let fens = [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "rnbqkbnr/pppppppp/4p3/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w kq e3 0 1",
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b Kq e3 0 1",
        ];
        for fen in fens {
            let gs = State::from_fen(fen);
            assert_eq!(gs.to_fen(), fen);
        }
    }
}
