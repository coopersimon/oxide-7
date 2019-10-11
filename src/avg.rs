// Averager

use std::{
    collections::VecDeque,
    ops::{
        Add,
        Div
    }
};

pub struct Averager<T: Add + Div> {
    queue:      VecDeque<T>,
    max_len:    usize
}

impl Averager<usize> {
    pub fn new(len: usize) -> Self {
        Averager {
            queue:      VecDeque::with_capacity(len),
            max_len:    len
        }
    }

    pub fn add(&mut self, data: usize) {
        self.queue.push_back(data);

        if self.queue.len() > self.max_len {
            self.queue.pop_front();
        }
    }

    pub fn get_avg(&self) -> usize {
        self.queue.iter().sum::<usize>() / self.queue.len()
    }
}