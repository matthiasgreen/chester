use crate::r#move::Move;

pub trait Insert<T> {
    fn insert(&mut self, value: T);
}

pub trait Plys<T> {
    fn new_ply(&mut self);
    fn ply_size(&self) -> usize;
    fn get(&self, index: usize) -> T;
    fn drop_ply(&mut self);
}

pub struct MoveList {
    moves: [Move; 2048],
    ply_first_move: [usize; 128], // Index of the first move for a given ply
    current_ply: usize,
    total_count: usize,
}

impl Insert<Move> for MoveList {
    fn insert(&mut self, m: Move) {
        assert!(self.current_ply != 0);
        self.moves[self.total_count] = m;
        self.total_count += 1;
    }
}

impl Plys<Move> for MoveList {
    fn new_ply(&mut self) {
        self.current_ply += 1;
        self.ply_first_move[self.current_ply] = self.total_count;
    }

    fn ply_size(&self) -> usize {
        assert!(self.current_ply != 0);
        self.total_count - self.ply_first_move[self.current_ply]
    }

    fn get(&self, index: usize) -> Move {
        self.current_ply()[index]
    }

    fn drop_ply(&mut self) {
        assert!(self.current_ply != 0);
        self.total_count = self.ply_first_move[self.current_ply];
        self.current_ply -= 1;
    }
}

impl MoveList {
    pub fn new() -> MoveList {
        MoveList {
            moves: [Move::from_bits(0); 2048],
            ply_first_move: [0; 128],
            current_ply: 0,
            total_count: 0,
        }
    }

    pub fn ply_number(&self) -> usize {
        self.current_ply
    }

    pub fn current_ply(&self) -> &[Move] {
        assert!(self.current_ply != 0);
        let first_move_index = self.ply_first_move[self.current_ply];
        &self.moves[first_move_index..self.total_count]
    }

    pub fn current_ply_mut(&mut self) -> &mut [Move] {
        assert!(self.current_ply != 0);
        let first_move_index = self.ply_first_move[self.current_ply];
        &mut self.moves[first_move_index..self.total_count]
    }

    /// "Sorts" the ply in place
    ///
    /// Optional first move is placed first in the ply
    ///
    /// Loud moves are placed before quiet moves
    pub fn order_ply(&mut self, first: Option<Move>) {
        // Selection sort
        let ply = self.current_ply_mut();

        let mut sorted_index = 0;

        // Place first move at the start
        if let Some(first) = first {
            for i in 0..ply.len() {
                if ply[i] == first {
                    ply.swap(i, 0);
                    sorted_index += 1;
                    break;
                }
            }
        }

        // Put loud moves before quiet moves
        let range = sorted_index..ply.len();
        for i in range {
            if !ply[i].code().is_quiet() {
                ply.swap(i, sorted_index);
                sorted_index += 1;
            }
        }
    }
}

impl<T> Insert<T> for Vec<T> {
    fn insert(&mut self, value: T) {
        self.push(value);
    }
}

impl<T: Copy> Plys<T> for Vec<T> {
    fn new_ply(&mut self) {
        debug_assert!(self.is_empty(), "Vec can only be used for a single ply.");
    }

    fn ply_size(&self) -> usize {
        self.len()
    }

    fn get(&self, index: usize) -> T {
        self[index]
    }

    fn drop_ply(&mut self) {
        self.clear();
    }
}
