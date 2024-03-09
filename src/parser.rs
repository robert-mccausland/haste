use std::{
    collections::{HashMap, HashSet},
    io::{Read, Seek},
};

use crate::{
    protos::{dota, Demo, DemoKind, Packet, PacketKind},
    readers::{bits::BitReader, demos::DemoReader, packets::PacketReader},
    string_tables::StringTable,
    Result,
};

const SIGNATURE: &[u8] = b"PBDEMS2\0";

const DEMOS_INTERNALLY_HANDLED: &[DemoKind] = &[
    DemoKind::Packet,
    DemoKind::SignOnPacket,
    DemoKind::FullPacket,
];

const PACKETS_INTERNALLY_HANDLED: &[PacketKind] =
    &[PacketKind::CreateStringTable, PacketKind::UpdateStringTable];

pub trait EventHandler {
    fn on_demo(&mut self, _demo: Demo, _context: &ParserContext) -> Result<()> {
        Ok(())
    }

    fn on_packet(&mut self, _demo: Packet, _context: &ParserContext) -> Result<()> {
        Ok(())
    }
}

pub struct ParserContext {
    string_tables: Vec<StringTable>,
    string_table_names: HashMap<String, usize>,
}

pub struct Parser<T: EventHandler> {
    context: ParserContext,
    event_handler: T,
    demos_handled: Vec<DemoKind>,
    packets_handled: Vec<PacketKind>,
}

impl ParserContext {
    pub fn get_string_table_by_name(&self, name: &str) -> Option<&StringTable> {
        self.string_table_names
            .get(name)
            .and_then(|id| self.get_string_table_by_id(*id))
    }

    pub fn get_string_table_by_id(&self, id: usize) -> Option<&StringTable> {
        self.string_tables.get(id)
    }
}

impl<T: EventHandler> Parser<T> {
    pub fn new(
        event_handler: T,
        demos_handled: &[DemoKind],
        packets_handled: &[PacketKind],
    ) -> Self {
        Self {
            context: ParserContext {
                string_tables: vec![],
                string_table_names: HashMap::new(),
            },
            event_handler,
            demos_handled: Vec::from_iter(
                demos_handled
                    .to_owned()
                    .into_iter()
                    .chain(DEMOS_INTERNALLY_HANDLED.to_owned()),
            ),
            packets_handled: Vec::from_iter(
                packets_handled
                    .to_owned()
                    .into_iter()
                    .chain(PACKETS_INTERNALLY_HANDLED.to_owned()),
            ),
        }
    }

    pub fn parse<R: Read + Seek>(&mut self, data: &mut R) -> Result<()> {
        let mut buf = [0; 16];
        data.read_exact(&mut buf)?;

        // Check signature of file
        let signature = &buf[0..8];
        if signature != SIGNATURE {
            return Err(format!(
                "expected signature to equal {:?} but got {:?}",
                SIGNATURE, signature
            )
            .into());
        }

        // Read demo messages
        let reader = DemoReader::new(data, self.demos_handled.to_owned());
        for message in reader {
            match message?.content {
                Demo::SignOnPacket(packet) => self.handle_packet(packet)?,
                Demo::Packet(packet) => self.handle_packet(packet)?,
                Demo::FullPacket(data) => {
                    if let Some(packet) = data.packet {
                        self.handle_packet(packet)?
                    }
                }
                demo => self.event_handler.on_demo(demo, &self.context)?,
            }
        }

        Ok(())
    }

    fn handle_packet(&mut self, packet: dota::CDemoPacket) -> Result<()> {
        let packet_reader = PacketReader::new(
            BitReader::new(packet.data()),
            self.packets_handled.to_owned(),
        );

        // We must sort packets as some packets contain data that is needed to process other packets.
        let mut packets = packet_reader.collect::<Result<Vec<_>>>()?;
        packets.sort_by_key(|packet| match packet {
            Packet::CreateStringTable(_) => -2,
            Packet::UpdateStringTable(_) => -1,
            _ => 0,
        });

        for packet in packets {
            match packet {
                Packet::CreateStringTable(message) => self.on_create_string_table(message)?,
                Packet::UpdateStringTable(message) => self.on_update_string_table(message)?,
                packet => self.event_handler.on_packet(packet, &self.context)?,
            }
        }

        Ok(())
    }

    fn on_create_string_table(&mut self, message: dota::CsvcMsgCreateStringTable) -> Result<()> {
        let mut string_table = StringTable::new(
            message.name().to_owned(),
            message
                .user_data_fixed_size()
                .then_some(message.user_data_size_bits().try_into()?),
            message.using_varint_bitcounts(),
            message.flags().try_into()?,
        );

        let data = if message.data_compressed() {
            snap::raw::Decoder::new().decompress_vec(message.string_data())?
        } else {
            message.string_data.unwrap_or_default()
        };

        string_table.update_entries(
            data.as_slice(),
            message.num_entries.unwrap_or_default().try_into()?,
        )?;

        self.context.string_table_names.insert(
            message.name.unwrap_or_default(),
            self.context.string_tables.len(),
        );
        self.context.string_tables.push(string_table);

        Ok(())
    }

    fn on_update_string_table(&mut self, message: dota::CsvcMsgUpdateStringTable) -> Result<()> {
        if let Some(string_table) = self
            .context
            .string_tables
            .get_mut(TryInto::<usize>::try_into(message.table_id())?)
        {
            string_table.update_entries(
                message.string_data(),
                message.num_changed_entries().try_into()?,
            )?;
        }

        Ok(())
    }
}
