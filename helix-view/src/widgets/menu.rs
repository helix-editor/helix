pub trait Item: AsRef<&str> {}

pub trait Menu<T: Item> {
    fn score(&mut self, pattern: &str);
    fn move_up(&mut self);
    fn move_down(&mut self);
    fn adjust_scroll(&mut self);
    fn selection(&self) -> Option<&T>;
    fn is_empty(&self) -> bool;
    fn len(&self) -> usize;
}
