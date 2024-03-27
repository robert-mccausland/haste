use std::collections::HashSet;

use haste_protobuf::{Packet, PacketKind};

use crate::{decoders::Bits, readers::bits::BitReader, Result};

pub struct PacketReader<'a> {
    stream: BitReader<'a>,
    messages_kinds: HashSet<PacketKind>,
}

pub struct PacketMessage {
    pub content: Packet,
    pub kind: PacketKind,
}

impl core::fmt::Debug for PacketMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PacketMessage")
            .field("kind", &self.kind)
            .finish()
    }
}

impl<'a> PacketReader<'a> {
    pub fn new(stream: BitReader<'a>, message_kinds: Vec<PacketKind>) -> Self {
        Self {
            stream,
            messages_kinds: HashSet::from_iter(message_kinds.into_iter()),
        }
    }

    pub fn read_next_message(&mut self) -> Result<Option<PacketMessage>> {
        while self.stream.remaining_bits() >= 8 {
            let kind = self.stream.read_varbit().try_into()?;
            let size = self.stream.read_varint_u32()?.try_into()?;
            if self.messages_kinds.contains(&kind) || true {
                let data = self.stream.read_bytes(size);
                return Ok(Some(PacketMessage {
                    content: Packet::decode(&kind, data.as_slice())?,
                    kind,
                }));
            } else {
                self.stream.advance(size * 8);
            }
        }

        return Ok(None);
    }
}

impl<'a> Iterator for PacketReader<'a> {
    type Item = Result<PacketMessage>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_next_message() {
            Ok(Some(message)) => Some(Ok(message)),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        }
    }
}
