use crate::{Result, decoders::Bytes};
use bytes::Buf;
use haste_protobuf::{Demo, DemoKind};
use std::{
    collections::HashSet,
    io::{self, Read},
};

const BUF_SIZE: usize = 512;

pub struct DemoMessage {
    pub kind: DemoKind,
    pub tick: i32,
    pub content: Demo,
}

impl core::fmt::Debug for DemoMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DemoMessage")
            .field("kind", &self.kind)
            .field("tick", &self.tick)
            .finish()
    }
}

pub struct DemoReader<R: Read> {
    data: R,
    buffer: [u8; BUF_SIZE],
    initialized_bytes: usize,
    messages_kinds: HashSet<DemoKind>,
}

impl<R: Read> DemoReader<R> {
    pub fn new(data: R, message_kinds: Vec<DemoKind>) -> Self {
        Self {
            data,
            buffer: [0; BUF_SIZE],
            initialized_bytes: 0,
            messages_kinds: HashSet::from_iter(message_kinds.into_iter()),
        }
    }

    fn read_message_header<B: Buf>(mut buffer: B) -> Result<(DemoKind, bool, i32, usize)> {
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
                    io::copy(
                        &mut (&mut self.data).take((size - remaining) as u64),
                        &mut io::sink(),
                    )?;
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

                let content = Demo::decode(&kind.clone().try_into()?, message_data.as_slice())?;
                Some(DemoMessage {
                    kind,
                    tick,
                    content,
                })
            };

            // Copy the end of whatever we did not read to the start of the buffer
            self.initialized_bytes = buffer.remaining();
            self.buffer
                .copy_within((total_bytes - self.initialized_bytes)..total_bytes, 0);

            if let Some(message) = message {
                return Ok(Some(message));
            }
        }
    }
}

impl<R: Read> Iterator for DemoReader<R> {
    type Item = Result<DemoMessage>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_next_message() {
            Ok(Some(result)) => Some(Ok(result)),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        }
    }
}
