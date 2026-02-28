pub mod collections;
pub mod color;
pub mod hash;
pub mod r#move;
pub mod position;
pub mod square;
pub mod state;

pub trait Insert<T> {
    fn insert(&mut self, value: T);
}

impl<T> Insert<T> for Vec<T> {
    fn insert(&mut self, value: T) {
        self.push(value);
    }
}
