use bytes::Buf;

use crate::{readers::bits::BitReader, Result};

pub trait Bytes {
    fn decode_varint(&mut self) -> Result<u64>;
    fn decode_varint_signed(&mut self) -> Result<i32>;
}

pub trait Bits {
    fn read_varint_u64(&mut self) -> Result<u64>;
    fn read_varint_u32(&mut self) -> Result<u32>;
    fn read_varint_i32(&mut self) -> Result<i32>;
    fn read_varbit(&mut self) -> u32;
    fn read_varbit_field_path(&mut self) -> Result<i32>;
    fn read_boolean(&mut self) -> bool;
    fn read_string(&mut self) -> Result<String>;
}

impl<B: Buf> Bytes for B {
    fn decode_varint(&mut self) -> Result<u64> {
        Ok(prost::encoding::decode_varint(self)?)
    }

    fn decode_varint_signed(&mut self) -> Result<i32> {
        // Safety: its probably fine idk
        unsafe {
            Ok(core::mem::transmute::<u32, i32>(
                self.decode_varint()?.try_into()?,
            ))
        }
    }
}

impl<'a> Bits for BitReader<'a> {
    fn read_varint_u64(&mut self) -> Result<u64> {
        let mut result = 0;
        let mut offset = 0;
        loop {
            let byte = self.read_bits(8);

            // TODO check int is not too big!

            result |= (byte & 0x7F) << offset;

            if byte < 0x80 {
                return Ok(result);
            }

            offset += 7;
        }
    }

    fn read_varint_u32(&mut self) -> Result<u32> {
        Ok(self.read_varint_u64()?.try_into()?)
    }

    fn read_varint_i32(&mut self) -> Result<i32> {
        let unsigned = self.read_varint_u32()?;
        let signed = (unsigned >> 1) as i32;
        Ok(if unsigned & 1 != 0 { !signed } else { signed })
    }

    fn read_varbit(&mut self) -> u32 {
        let start = self.read_bits(6) as u8;
        let indicator = start >> 4;
        let length = match indicator {
            0 => return start as u32,
            1 => 4,
            2 => 8,
            3 => 28,
            _ => unreachable!(),
        };

        return (start as u32 & 15) | ((self.read_bits(length) as u32) << 4);
    }

    fn read_varbit_field_path(&mut self) -> Result<i32> {
        let mut i = 0;
        while i < 4 {
            if self.read_boolean() {
                break;
            }

            i += 1;
        }

        let bit_count = match i {
            0 => 2,
            1 => 4,
            2 => 10,
            3 => 17,
            4 => 31,
            _ => Err(format!("Invalid bit_count"))?,
        };

        return Ok(self.read_bits(bit_count) as i32);
    }

    fn read_boolean(&mut self) -> bool {
        self.read_bits(1) != 0
    }

    fn read_string(&mut self) -> Result<String> {
        let mut result = Vec::new();
        loop {
            let char = self.read_bits(8) as u8;
            if char == 0 {
                return Ok(String::from_utf8(result)?);
            }
            result.push(char)
        }
    }
}
#[allow(unused)]
mod tests {
    use super::*;

    #[test]
    fn read_var_bit_4_length() {
        let data = [0x0A];
        let mut bit_stream = BitReader::new(data.as_slice());
        assert_eq!(bit_stream.read_varbit(), 0x0A);
    }

    #[test]
    fn read_var_bit_8_length() {
        let data = [0b1001_0111, 0b11];
        let mut bit_stream = BitReader::new(data.as_slice());
        assert_eq!(bit_stream.read_varbit(), 0b11100111);
    }

    #[test]
    fn read_var_bit_12_length() {
        let data = [0b1011_0111, 0b11_0011];
        let mut bit_stream = BitReader::new(data.as_slice());
        assert_eq!(bit_stream.read_varbit(), 0b1100_1110_0111);
    }

    #[test]
    fn read_var_bit_32_length() {
        let data = [0b0011_1010, 0b0011_1100, 0b1010_1000, 0b0101_0110, 0b01];
        let mut bit_stream = BitReader::new(data.as_slice());
        assert_eq!(bit_stream.read_varbit(), 0x55AA0F0A);
    }
}
