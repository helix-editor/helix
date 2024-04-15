use std::collections::{vec_deque, VecDeque};

#[derive(Debug, Default)]
pub struct Ring<T> {
    data: VecDeque<T>,
}

impl<T> Ring<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
        }
    }

    pub fn iter(&self) -> vec_deque::Iter<T> /*VecDeque<T>::Iter*/ {
        self.data.iter()
    }

    pub fn rotate_forward(&mut self, mut shifts: usize) {
        if shifts > self.data.len() {
            shifts %= self.data.len();
        }
        self.data.rotate_right(shifts);
    }

    pub fn rotate_backward(&mut self, mut shifts: usize) {
        if shifts > self.data.len() {
            shifts %= self.data.len();
        }
        self.data.rotate_left(shifts);
    }

    pub fn current(&self) -> Option<&T> {
        self.data.front()
    }

    pub fn push(&mut self, item: T) {
        if self.data.len() == self.data.capacity() {
            self.data.pop_back();
        }
        self.data.push_front(item);
    }
}

pub type YankRing = Ring<Vec<String>>;
