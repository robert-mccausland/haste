use std::collections::HashSet;

use bytes::Buf;

use crate::{
    decoders::Bits,
    protos::{Packet, PacketKind},
    readers::bits::BitReader,
    Result,
};

pub struct PacketReader<B: Buf> {
    stream: BitReader<B>,
    messages_kinds: HashSet<PacketKind>,
}

impl<B: Buf> PacketReader<B> {
    pub fn new(stream: BitReader<B>, message_kinds: Vec<PacketKind>) -> Self {
        Self {
            stream,
            messages_kinds: HashSet::from_iter(message_kinds.into_iter()),
        }
    }

    pub fn read_next_message(&mut self) -> Result<Option<Packet>> {
        while self.stream.remaining_bits() >= 8 {
            let kind = self.stream.read_varbit().try_into()?;
            let size = self.stream.read_varint_u32()?.try_into()?;
            if self.messages_kinds.contains(&kind) {
                let data = self.stream.read_bytes(size);
                return Ok(Some(Packet::decode(kind, data.as_slice())?));
            } else {
                //self.stream.read_bytes(size);
                self.stream.advance(size * 8);
            }
        }

        return Ok(None);
    }
}

impl<B: Buf> Iterator for PacketReader<B> {
    type Item = Result<Packet>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_next_message() {
            Ok(Some(message)) => Some(Ok(message)),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        }
    }
}
