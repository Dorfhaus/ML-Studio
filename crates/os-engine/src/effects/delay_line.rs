// SPDX-License-Identifier: GPL-3.0-or-later

/// A resizable delay with an immediate logical reset. Samples from an earlier
/// length are never read after reset, so changes need no buffer-sized clear.
pub(super) struct DelayLine<T> {
    buffer: Vec<T>,
    length: usize,
    cursor: usize,
    filled: usize,
}

impl<T: Copy + Default> DelayLine<T> {
    pub fn new(capacity: usize) -> Self {
        Self { buffer: vec![T::default(); capacity], length: 0, cursor: 0, filled: 0 }
    }

    pub fn prepare(&mut self, capacity: usize) {
        if capacity > self.buffer.len() {
            self.buffer.resize(capacity, T::default());
        }
    }

    pub fn set_length(&mut self, length: usize) {
        self.prepare(length);
        self.length = length;
        self.clear();
    }

    pub fn clear(&mut self) {
        self.cursor = 0;
        self.filled = 0;
    }

    pub fn len(&self) -> usize {
        self.length
    }

    #[inline]
    pub fn push(&mut self, input: T) -> T {
        if self.length == 0 {
            return input;
        }
        let old = std::mem::replace(&mut self.buffer[self.cursor], input);
        self.cursor += 1;
        if self.cursor == self.length {
            self.cursor = 0;
        }
        if self.filled < self.length {
            self.filled += 1;
            T::default()
        } else {
            old
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resets_and_resizes_in_place_without_reading_old_samples() {
        let mut line = DelayLine::new(4);
        let buffer = line.buffer.as_ptr();
        line.set_length(2);
        assert_eq!([1, 2, 3, 4].map(|v| line.push(v)), [0, 0, 1, 2]);
        line.set_length(3);
        assert_eq!([5, 6, 7, 8].map(|v| line.push(v)), [0, 0, 0, 5]);
        line.clear();
        assert_eq!(line.push(9), 0);
        line.set_length(0);
        assert_eq!(line.push(10), 10);
        assert_eq!(line.buffer.as_ptr(), buffer);
    }
}
