#[derive(Debug)]
pub enum AccumulatorFeedError {
    InputTooLarge,
}
#[derive(Debug)]
pub enum AccumulatorYieldError {
    DecodingError,
    NotACobsFrame,
}

pub const RX_BUF_SIZE: usize = 1024;
pub struct Accumulator<const N: usize> {
    buf: [u8; N],
    start_idx: usize,
    end_idx: usize,
}

impl<const N: usize> Accumulator<N> {
    pub const fn new() -> Self {
        Self {
            buf: [0u8; N],
            start_idx: 0,
            end_idx: 0,
        }
    }

    pub fn reset(&mut self) {
        self.start_idx = 0;
        self.end_idx = 0;
    }

    pub fn feed(&mut self, input: &[u8]) -> Result<(), AccumulatorFeedError> {
        let current_size = self.end_idx - self.start_idx;
        if input.len() + current_size > self.buf.len() {
            return Err(AccumulatorFeedError::InputTooLarge);
        }

        if self.start_idx > 0 {
            self.buf.copy_within(self.start_idx..self.end_idx, 0);
            self.start_idx = 0;
            self.end_idx = current_size;
        }

        self.extend_unchecked(input);
        Ok(())
    }

    /// Extend the internal buffer with the given input.
    ///
    /// # Panics
    ///
    /// Will panic if the input does not fit in the internal buffer.
    fn extend_unchecked(&mut self, input: &[u8]) {
        let new_end = self.end_idx + input.len();
        self.buf[self.end_idx..new_end].copy_from_slice(input);
        self.end_idx = new_end;
    }

    pub fn yield_frame(&mut self) -> Result<&[u8], AccumulatorYieldError> {
        let zero_pos = self.buf[self.start_idx..self.end_idx]
            .iter()
            .position(|&i| i == 0);
        match zero_pos {
            Some(pos) => {
                let cobs_encoded_frame = &mut self.buf[self.start_idx..self.start_idx + pos];
                let retval = match cobs::decode_in_place(cobs_encoded_frame) {
                    Ok(used) => Ok(&self.buf[self.start_idx..used]),
                    Err(_) => Err(AccumulatorYieldError::DecodingError),
                };
                self.start_idx = self.start_idx + pos + 1;
                retval
            }
            None => Err(AccumulatorYieldError::NotACobsFrame),
        }
    }
}

impl<const N: usize> Default for Accumulator<N> {
    fn default() -> Self {
        Self::new()
    }
}
