use std::io::Read;

use haste_protobuf::{dota, Demo, DemoKind, Packet, PacketKind};

use crate::{
    entities::{Entities, Entity},
    readers::{bits::BitReader, demos::DemoReader, packets::PacketReader},
    string_tables::StringTables,
    Result,
};

const SIGNATURE: &[u8] = b"PBDEMS2\0";
const ENTITY_BASELINES_TABLE_NAME: &str = "instancebaseline";

pub trait EventHandler {
    fn on_demo(&mut self, _demo: Demo, _context: &ParserContext) -> Result<()> {
        Ok(())
    }

    fn on_packet(&mut self, _demo: Packet, _context: &ParserContext) -> Result<()> {
        Ok(())
    }

    fn on_entity_changed(&mut self, _entity: &Entity, _context: &ParserContext) -> Result<()> {
        Ok(())
    }

    fn should_continue(&mut self) -> Result<bool> {
        Ok(true)
    }
}

pub struct ParserContext {
    pub entities: Entities,
    pub string_tables: StringTables,
    pub current_tick: i32,
}

pub struct Parser {
    context: ParserContext,
    demos_handled: Vec<DemoKind>,
    packets_handled: Vec<PacketKind>,
}

impl Parser {
    pub fn new(
        demos_handled: &[DemoKind],
        packets_handled: &[PacketKind],
        parse_entities: bool,
    ) -> Self {
        Self {
            context: ParserContext {
                entities: Entities::new(),
                string_tables: StringTables::new(),
                current_tick: 0,
            },
            demos_handled: Vec::from_iter(
                demos_handled
                    .to_owned()
                    .into_iter()
                    .chain(get_internal_demos(parse_entities).to_owned()),
            ),
            packets_handled: Vec::from_iter(
                packets_handled
                    .to_owned()
                    .into_iter()
                    .chain(get_internal_packets(parse_entities).to_owned()),
            ),
        }
    }

    pub fn parse<R: Read, T: EventHandler>(
        &mut self,
        data: &mut R,
        event_handler: &mut T,
    ) -> Result<()> {
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
            let message = message?;
            self.context.current_tick = message.tick;
            match message.content {
                Demo::SignOnPacket(packet) => self.handle_packet(packet, event_handler)?,
                Demo::Packet(packet) => self.handle_packet(packet, event_handler)?,
                Demo::FullPacket(data) => {
                    if let Some(packet) = data.packet {
                        self.handle_packet(packet, event_handler)?
                    }
                }
                Demo::ClassInfo(class_info) => {
                    self.context.entities.on_class_info(class_info)?;
                    if let Some(baseline_table) = self
                        .context
                        .string_tables
                        .get_by_name(ENTITY_BASELINES_TABLE_NAME)
                    {
                        self.context.entities.update_baselines(baseline_table)?;
                    }
                }
                Demo::SendTables(send_tables) => {
                    self.context.entities.on_send_tables(send_tables)?
                }
                demo => event_handler.on_demo(demo, &self.context)?,
            }

            if !event_handler.should_continue()? {
                return Ok(());
            }
        }

        Ok(())
    }

    fn handle_packet<T: EventHandler>(
        &mut self,
        packet: dota::CDemoPacket,
        event_handler: &mut T,
    ) -> Result<()> {
        let packet_reader = PacketReader::new(
            BitReader::new(packet.data()),
            self.packets_handled.to_owned(),
        );

        // We must sort packets as some packets contain data that is needed to process other packets.
        let mut packets = packet_reader.collect::<Result<Vec<_>>>()?;
        packets.sort_by_key(|packet| match packet.kind {
            PacketKind::CreateStringTable => -2,
            PacketKind::UpdateStringTable => -1,
            _ => 0,
        });

        for packet in packets {
            match packet.content {
                Packet::CreateStringTable(message) => {
                    let update_baseline = message.name() == ENTITY_BASELINES_TABLE_NAME;
                    self.context.string_tables.on_create_string_table(message)?;
                    if update_baseline {
                        self.context.entities.update_baselines(
                            self.context
                                .string_tables
                                .get_by_name(ENTITY_BASELINES_TABLE_NAME)
                                .unwrap(),
                        )?;
                    }
                }
                Packet::UpdateStringTable(message) => {
                    let table_id = message.table_id().try_into()?;
                    self.context.string_tables.on_update_string_table(message)?;
                    let table = self.context.string_tables.get(table_id).unwrap();
                    if table.get_name() == ENTITY_BASELINES_TABLE_NAME {
                        self.context.entities.update_baselines(table)?;
                    }
                }
                Packet::ServerInfo(message) => self
                    .context
                    .entities
                    .update_max_classes(message.max_classes().try_into()?),
                Packet::PacketEntities(message) => {
                    let updated_entities = self.context.entities.on_packet_entities(message)?;
                    for entity_id in updated_entities {
                        if let Some(entity) = self.context.entities.get(entity_id.try_into()?) {
                            event_handler.on_entity_changed(entity, &self.context)?;
                        }
                    }
                }
                packet => event_handler.on_packet(packet, &self.context)?,
            }
        }

        Ok(())
    }
}

fn get_internal_demos(parse_entities: bool) -> &'static [DemoKind] {
    if parse_entities {
        return &[
            DemoKind::Packet,
            DemoKind::SignOnPacket,
            DemoKind::FullPacket,
            DemoKind::SendTables,
            DemoKind::ClassInfo,
        ];
    } else {
        return &[
            DemoKind::Packet,
            DemoKind::SignOnPacket,
            DemoKind::FullPacket,
        ];
    }
}

fn get_internal_packets(parse_entities: bool) -> &'static [PacketKind] {
    if parse_entities {
        return &[
            PacketKind::CreateStringTable,
            PacketKind::UpdateStringTable,
            PacketKind::PacketEntities,
            PacketKind::ServerInfo,
        ];
    } else {
        return &[PacketKind::CreateStringTable, PacketKind::UpdateStringTable];
    }
}
