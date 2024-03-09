use bytes::Buf;

use crate::decoders::Bits;
use crate::Result;

use super::bits::BitReader;

const KEY_HISTORY_BITS: usize = 5;
const KEY_HISTORY_SIZE: usize = 1 << KEY_HISTORY_BITS;
const KEY_HISTORY_MASK: usize = KEY_HISTORY_SIZE - 1;

pub struct StringTableReader<B: Buf> {
    index: usize,
    entry_index: i32,
    entry_count: usize,
    key_history: Vec<String>,
    data: BitReader<B>,
    data_size_bits: Option<usize>,
    has_varint_bit_counts: bool,
    flags: u32,
}

pub struct StringTableReaderEntry {
    pub index: u32,
    pub name: Option<String>,
    pub data: Option<Vec<u8>>,
}

impl<B: Buf> StringTableReader<B> {
    pub fn new(
        data: B,
        entry_count: usize,
        data_size_bits: Option<usize>,
        has_varint_bit_counts: bool,
        flags: u32,
    ) -> Self {
        Self {
            data: BitReader::new(data),
            entry_count,
            data_size_bits,
            has_varint_bit_counts,
            flags,
            index: 0,
            entry_index: -1,
            key_history: vec![String::new(); KEY_HISTORY_SIZE],
        }
    }

    fn read_next_entry(&mut self) -> Result<StringTableReaderEntry> {
        // Read index
        if self.data.read_boolean() {
            self.entry_index += 1;
        } else {
            self.entry_index += TryInto::<i32>::try_into(self.data.read_varint_u32()?)? + 2;
        }

        let name = if self.data.read_boolean() {
            Some(self.read_name()?)
        } else {
            None
        };

        let data = if self.data.read_boolean() {
            Some(self.read_data()?)
        } else {
            None
        };

        self.index += 1;

        return Ok(StringTableReaderEntry {
            index: self.entry_index.try_into()?,
            data,
            name,
        });
    }

    fn read_name(&mut self) -> Result<String> {
        let name = if self.data.read_boolean() {
            let base = if self.index > KEY_HISTORY_SIZE {
                self.index
            } else {
                0
            };

            let offset = self.data.read_bits(5) as usize;
            let length = self.data.read_bits(5) as usize;
            let prefix = &self.key_history[(base + offset) & KEY_HISTORY_MASK];
            prefix[..length.min(prefix.len())].to_owned() + &self.data.read_string()?
        } else {
            self.data.read_string()?
        };

        self.key_history[self.index & KEY_HISTORY_MASK] = name.to_owned();

        return Ok(name);
    }

    fn read_data(&mut self) -> Result<Vec<u8>> {
        let mut is_compressed = false;
        let bit_length = self.data_size_bits.unwrap_or_else(|| {
            if self.flags & 1 != 0 {
                is_compressed = self.data.read_boolean()
            }

            return if self.has_varint_bit_counts {
                self.data.read_varbit() as usize * 8
            } else {
                self.data.read_bits(17) as usize * 8
            };
        });

        let full_byte_length = (bit_length as usize) / 8;
        let mut buffer = self.data.read_bytes(full_byte_length);
        if bit_length % 8 != 0 {
            buffer.push(self.data.read_bits(bit_length % 8) as u8);
        }

        return Ok(if is_compressed {
            snap::raw::Decoder::new().decompress_vec(&buffer)?
        } else {
            buffer
        });
    }
}

impl<B: Buf> Iterator for StringTableReader<B> {
    type Item = Result<StringTableReaderEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.entry_count {
            Some(self.read_next_entry())
        } else {
            None
        }
    }
}
