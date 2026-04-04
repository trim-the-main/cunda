#![no_std]

#[derive(Debug)]
pub enum FeedError {
    InputTooLarge,
}

#[derive(Debug)]
pub enum DecodeError {
    DecodingError,
    Incomplete,
    NoData,
}

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

    pub fn feed(&mut self, input: &[u8]) -> Result<(), FeedError> {
        let current_size = self.end_idx - self.start_idx;
        if input.len() + current_size > self.buf.len() {
            return Err(FeedError::InputTooLarge);
        }

        if self.start_idx > 0 {
            self.buf.copy_within(self.start_idx..self.end_idx, 0);
            self.start_idx = 0;
            self.end_idx = current_size;
        }

        self.extend_unchecked(input);
        Ok(())
    }

    pub fn yield_frame(&mut self) -> Result<&[u8], DecodeError> {
        if self.start_idx == self.end_idx {
            return Err(DecodeError::NoData);
        }
        let zero_pos = self.buf[self.start_idx..self.end_idx]
            .iter()
            .position(|&i| i == 0);
        match zero_pos {
            Some(pos) => {
                let cobs_encoded_frame = &mut self.buf[self.start_idx..self.start_idx + pos];
                let retval = match cobs::decode_in_place(cobs_encoded_frame) {
                    Ok(used) => Ok(&self.buf[self.start_idx..self.start_idx + used]),
                    Err(_) => Err(DecodeError::DecodingError),
                };
                self.start_idx = self.start_idx + pos + 1;
                retval
            }
            None => Err(DecodeError::Incomplete),
        }
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
}

impl<const N: usize> Default for Accumulator<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec::Vec;
    use super::*;

    /// COBS-encode `data` and append a zero-byte delimiter.
    fn make_frame(data: &[u8]) -> Vec<u8> {
        let mut frame = cobs::encode_vec(data);
        frame.push(0x00);
        frame
    }

    // --- yield_frame error cases ---

    #[test]
    fn yield_frame_on_empty_buffer_returns_no_data() {
        let mut acc = Accumulator::<64>::new();
        assert!(matches!(acc.yield_frame(), Err(DecodeError::NoData)));
    }

    #[test]
    fn yield_frame_without_delimiter_returns_incomplete() {
        let mut acc = Accumulator::<64>::new();
        acc.feed(&[0x01, 0x02, 0x03]).unwrap();
        assert!(matches!(acc.yield_frame(), Err(DecodeError::Incomplete)));
    }

    #[test]
    fn yield_frame_with_invalid_cobs_returns_decoding_error() {
        let mut acc = Accumulator::<64>::new();
        // Overhead byte 0x05 claims the next code is 5 bytes away,
        // but only 1 byte follows before the zero delimiter.
        acc.feed(&[0x05, 0x01, 0x00]).unwrap();
        assert!(matches!(acc.yield_frame(), Err(DecodeError::DecodingError)));
    }

    // --- successful decode ---

    #[test]
    fn yield_frame_decodes_valid_frame() {
        let mut acc = Accumulator::<64>::new();
        let data: &[u8] = &[1, 2, 3];
        acc.feed(&make_frame(data)).unwrap();
        assert_eq!(acc.yield_frame().unwrap(), data);
    }

    #[test]
    fn yield_frame_decodes_frame_containing_zeros() {
        let mut acc = Accumulator::<64>::new();
        let data: &[u8] = &[1, 0, 2, 0, 3];
        acc.feed(&make_frame(data)).unwrap();
        assert_eq!(acc.yield_frame().unwrap(), data);
    }

    // --- multi-frame ---

    #[test]
    fn two_frames_fed_at_once_are_yielded_independently() {
        let mut acc = Accumulator::<64>::new();
        let mut combined = make_frame(&[1, 2]);
        combined.extend_from_slice(&make_frame(&[3, 4]));
        acc.feed(&combined).unwrap();
        assert_eq!(acc.yield_frame().unwrap(), &[1, 2]);
        assert_eq!(acc.yield_frame().unwrap(), &[3, 4]);
        assert!(matches!(acc.yield_frame(), Err(DecodeError::NoData)));
    }

    #[test]
    fn frame_split_across_two_feeds_is_assembled() {
        let mut acc = Accumulator::<64>::new();
        let frame = make_frame(&[10, 20, 30]);
        let (first, second) = frame.split_at(frame.len() / 2);
        acc.feed(first).unwrap();
        assert!(matches!(acc.yield_frame(), Err(DecodeError::Incomplete)));
        acc.feed(second).unwrap();
        assert_eq!(acc.yield_frame().unwrap(), &[10, 20, 30]);
    }

    // --- feed errors ---

    #[test]
    fn feed_exceeding_capacity_returns_error() {
        let mut acc = Accumulator::<8>::new();
        assert!(matches!(acc.feed(&[0u8; 9]), Err(FeedError::InputTooLarge)));
    }

    #[test]
    fn feed_exactly_capacity_succeeds() {
        let mut acc = Accumulator::<8>::new();
        assert!(acc.feed(&[0u8; 8]).is_ok());
    }

    #[test]
    fn feed_fails_when_full_then_succeeds_after_yield_frees_space() {
        // frame1 fills most of the buffer, frame2 fits in the remaining tail,
        // frame3 does not fit until frame1 is consumed and the buffer is compacted.
        let mut acc = Accumulator::<32>::new();

        let frame1 = make_frame(&[0u8; 20]);
        let frame2 = make_frame(&[1u8]);
        let frame3 = make_frame(&[2u8; 10]);

        acc.feed(&frame1).unwrap();
        acc.feed(&frame2).unwrap();

        // frame3 does not fit yet
        assert!(matches!(acc.feed(&frame3), Err(FeedError::InputTooLarge)));

        // Consuming frame1 frees enough space; next feed compacts then succeeds
        assert_eq!(acc.yield_frame().unwrap(), &[0u8; 20]);
        acc.feed(&frame3).unwrap();

        // Remaining frames decode correctly
        assert_eq!(acc.yield_frame().unwrap(), &[1u8]);
        assert_eq!(acc.yield_frame().unwrap(), &[2u8; 10]);
    }

    #[test]
    fn feed_compacts_after_partial_consumption() {
        // After consuming one frame, start_idx > 0. A subsequent feed
        // should compact the buffer so remaining data fits.
        let mut acc = Accumulator::<32>::new();
        acc.feed(&make_frame(&[1])).unwrap();
        acc.feed(&make_frame(&[2])).unwrap();
        acc.yield_frame().unwrap(); // consume first frame, start_idx advances
        acc.feed(&make_frame(&[3])).unwrap(); // triggers compaction
        assert_eq!(acc.yield_frame().unwrap(), &[2]);
        assert_eq!(acc.yield_frame().unwrap(), &[3]);
    }

    // --- reset ---

    #[test]
    fn reset_causes_no_data() {
        let mut acc = Accumulator::<64>::new();
        acc.feed(&[1, 2, 3]).unwrap();
        acc.reset();
        assert!(matches!(acc.yield_frame(), Err(DecodeError::NoData)));
    }

    #[test]
    fn reset_allows_new_data() {
        let mut acc = Accumulator::<64>::new();
        acc.feed(&[0xFF; 32]).unwrap();
        acc.reset();
        let data: &[u8] = &[5, 6, 7];
        acc.feed(&make_frame(data)).unwrap();
        assert_eq!(acc.yield_frame().unwrap(), data);
    }
}
