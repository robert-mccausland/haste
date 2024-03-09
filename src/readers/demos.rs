use crate::{
    decoders::Bytes,
    protos::{Demo, DemoKind},
    Result,
};

use bytes::Buf;
use std::{
    collections::HashSet,
    io::{Read, Seek},
};

const BUF_SIZE: usize = 512;

pub struct DemoMessage {
    pub kind: DemoKind,
    pub tick: i32,
    pub content: Demo,
}

pub struct DemoReader<R: Read + Seek> {
    data: R,
    buffer: [u8; BUF_SIZE],
    initialized_bytes: usize,
    messages_kinds: HashSet<DemoKind>,
}

impl<R: Read + Seek> DemoReader<R> {
    pub fn new(data: R, message_kinds: Vec<DemoKind>) -> Self {
        Self {
            data,
            buffer: [0; BUF_SIZE],
            initialized_bytes: 0,
            messages_kinds: HashSet::from_iter(message_kinds.into_iter()),
        }
    }

    fn read_message_header<B: Buf>(buffer: &mut B) -> Result<(DemoKind, bool, i32, usize)> {
        let kind: u32 = buffer.decode_varint()?.try_into()?;
        let tick = buffer.decode_varint_signed()?;
        let size: usize = buffer.decode_varint()?.try_into()?;

        let is_compressed = kind & 0x40 > 0;
        let kind = kind & 0x3F;

        return Ok((kind.try_into()?, is_compressed, tick, size));
    }

    pub fn read_next_message(&mut self) -> Result<Option<DemoMessage>> {
        // Populate the buffer with data so we can read the header
        loop {
            let read_bytes = self.data.read(&mut self.buffer[self.initialized_bytes..])?;
            let total_bytes = read_bytes + self.initialized_bytes;
            if total_bytes == 0 {
                return Ok(None);
            }

            let mut buffer = &self.buffer[..total_bytes];
            let (kind, is_compressed, tick, size) = Self::read_message_header(&mut buffer)?;
            let remaining = buffer.remaining();

            let message = if !self.messages_kinds.contains(&kind) {
                if size > remaining {
                    self.data
                        .seek(std::io::SeekFrom::Current((size - remaining).try_into()?))?;
                    buffer.advance(remaining);
                } else {
                    buffer.advance(size);
                }
                None
            } else {
                let mut message_data = vec![0; size];
                if size < remaining {
                    buffer.copy_to_slice(&mut message_data[0..size]);
                } else {
                    buffer.copy_to_slice(&mut message_data[0..remaining]);
                    self.data.read_exact(&mut message_data[remaining..])?;
                }
                if is_compressed {
                    message_data = snap::raw::Decoder::new().decompress_vec(&message_data)?;
                }

                let content = Demo::decode(kind.clone().try_into()?, message_data.as_slice())?;
                Some(DemoMessage {
                    kind,
                    tick,
                    content,
                })
            };

            // Copy the end of whatever we did not read to the start of the buffer
            self.initialized_bytes = buffer.remaining();
            self.buffer
                .copy_within((BUF_SIZE - self.initialized_bytes).., 0);

            if let Some(message) = message {
                return Ok(Some(message));
            }
        }
    }
}

impl<R: Read + Seek> Iterator for DemoReader<R> {
    type Item = Result<DemoMessage>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_next_message() {
            Ok(Some(result)) => Some(Ok(result)),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        }
    }
}
