use bytes::Buf;

pub struct BitReader<'a> {
    buffer: &'a [u8],
    last_byte_offset: usize,
    last_byte: u8,
}

impl<'a> BitReader<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer,
            last_byte_offset: 8,
            last_byte: 0,
        }
    }
}

impl<'a> BitReader<'a> {
    pub fn read_bits(&mut self, count: usize) -> u64 {
        // Check for a 0 count because the code assumes count > 0
        if count == 0 {
            return 0;
        }

        // Read whatever left over from the previous operation first
        let mut result = (self.last_byte as u64) >> self.last_byte_offset;
        let mut bits_read = 8 - self.last_byte_offset;

        // Read any additional bytes that we need to get the total count
        while bits_read < count {
            /*&& self.buffer.remaining() != 0 { */
            self.last_byte = self.buffer.get_u8();
            result += (self.last_byte as u64) << bits_read;
            bits_read += 8;
        }

        // Update the offset with what is currently left over
        self.last_byte_offset = ((self.last_byte_offset + count - 1) & 7) + 1;

        // Strip the extra bits from the end of our result and return
        return result & (u64::MAX >> 64 - count);
    }

    pub fn read_bytes(&mut self, count: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(count);
        let mut current = 0;

        // Read in chunks of 8 bytes for efficiency
        while current + 8 < count {
            let bytes = self.read_bits(64).to_le_bytes();
            result.extend_from_slice(&bytes);
            current += 8;
        }

        // Append the remaining bits
        let rest = count - current;
        result.extend_from_slice(&self.read_bits(rest * 8).to_le_bytes()[0..rest]);
        return result;
    }

    pub fn remaining_bits(&self) -> usize {
        (self.buffer.remaining() * 8) + 8 - self.last_byte_offset
    }

    pub fn advance(&mut self, count: usize) {
        let bits_in_last_byte = 8 - self.last_byte_offset;
        if count > bits_in_last_byte {
            self.buffer.advance((count - bits_in_last_byte - 1) >> 3);
            self.last_byte = self.buffer.get_u8();
        }
        self.last_byte_offset = ((self.last_byte_offset + count - 1) & 7) + 1;
    }
}
#[allow(unused)]
mod tests {
    use super::*;

    #[test]
    fn read_alined_bits() {
        let data = [0x55; 8];
        let mut bit_stream = BitReader::new(data.as_slice());
        let result = bit_stream.read_bits(64);
        assert_eq!(result, 0x5555555555555555)
    }

    #[test]
    fn read_bits_individually() {
        let data = [0x55; 1];
        let mut bit_stream = BitReader::new(data.as_slice());
        for n in 0..8 {
            let result = bit_stream.read_bits(1);
            assert_eq!(result, ((n + 1) % 2) as u64)
        }
    }

    #[test]
    fn read_offset_ints() {
        let data = [0x55; 10];
        let mut bit_stream = BitReader::new(data.as_slice());
        assert_eq!(bit_stream.read_bits(4), 0x5);
        assert_eq!(bit_stream.read_bits(32), 0x55555555);
        assert_eq!(bit_stream.read_bits(1), 0x01);
        assert_eq!(bit_stream.read_bits(32), 0xAAAAAAAA);
        assert_eq!(bit_stream.read_bits(3), 0x2);
    }

    #[test]
    fn read_aligned_bytes() {
        let data = [0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19];
        let read_bytes = BitReader::new(data.as_slice()).read_bytes(8);
        assert_eq!(data.len(), read_bytes.len());
        for (left, right) in data.iter().zip(read_bytes.iter()) {
            assert_eq!(left, right);
        }
    }

    #[test]
    fn advance_bits_small_increments() {
        let data = [0xAA, 0xAA];
        let mut bit_stream = BitReader::new(data.as_slice());
        bit_stream.advance(3);
        assert_eq!(bit_stream.read_bits(1), 0x01);
        bit_stream.advance(4);
        assert_eq!(bit_stream.read_bits(8), 0xAA);
    }

    #[test]
    fn advance_bits_large_increments() {
        let data = [0xAA; 8];
        let mut bit_stream = BitReader::new(data.as_slice());
        assert_eq!(bit_stream.remaining_bits(), 64);
        bit_stream.advance(20);
        assert_eq!(bit_stream.remaining_bits(), 44);
        bit_stream.advance(24);
        assert_eq!(bit_stream.remaining_bits(), 20);
        bit_stream.advance(12);
        assert_eq!(bit_stream.remaining_bits(), 8);
        assert_eq!(bit_stream.read_bits(8), 0xAA);
    }
}
