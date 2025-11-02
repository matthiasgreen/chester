pub mod color;
pub mod hash;
pub mod r#move;
pub mod position;
pub mod square;
pub mod state;

pub trait Insert<T> {
    fn insert(&mut self, value: T);
}
