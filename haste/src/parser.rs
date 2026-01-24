use std::io::Read;

use haste_protobuf::{
    Demo, DemoKind, Packet, PacketKind,
    dota::{self, CDemoClassInfo},
};

use crate::{
    Result,
    entities::{Entities, Entity},
    readers::{
        bits::BitReader,
        demos::{DemoMessage, DemoReader},
        packets::{PacketMessage, PacketReader},
    },
    string_tables::StringTables,
};

const SIGNATURE: &[u8] = b"PBDEMS2\0";
const ENTITY_BASELINES_TABLE_NAME: &str = "instancebaseline";

pub trait EventHandler {
    fn on_demo(&mut self, _demo: &Demo, _context: &ParserContext) -> Result<()> {
        Ok(())
    }

    fn on_packet(&mut self, _demo: &Packet, _context: &ParserContext) -> Result<()> {
        Ok(())
    }

    fn on_entity_changed(&mut self, _entity: &Entity, _context: &ParserContext) -> Result<()> {
        Ok(())
    }

    fn should_continue(&mut self) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct ParserContext {
    pub entities: Entities,
    pub string_tables: StringTables,
    pub current_tick: i32,
}

#[derive(Debug)]
pub struct Parser {
    context: ParserContext,
    demos_handled: Vec<DemoKind>,
    packets_handled: Vec<PacketKind>,
}

#[derive(Debug)]
pub struct ParserError {
    pub message: Option<DemoMessage>,
    pub message_index: i32,
    pub inner_error: InnerError,
}

impl From<String> for ParserError {
    fn from(value: String) -> Self {
        Self {
            message: None,
            message_index: -1,
            inner_error: value.into(),
        }
    }
}

#[derive(Debug)]
pub enum InnerError {
    PacketError(PacketError),
    Generic(String),
}

impl From<String> for InnerError {
    fn from(value: String) -> Self {
        InnerError::Generic(value)
    }
}

impl From<PacketError> for InnerError {
    fn from(value: PacketError) -> Self {
        InnerError::PacketError(value)
    }
}

#[derive(Debug)]
pub struct PacketError {
    pub error_message: String,
    pub packet_index: i32,
    pub packet: Option<PacketMessage>,
}

impl From<String> for PacketError {
    fn from(value: String) -> Self {
        PacketError {
            error_message: value,
            packet_index: -1,
            packet: None,
        }
    }
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
    ) -> std::result::Result<(), ParserError> {
        let mut buf = [0; 16];
        let mut message_index = 0i32;

        data.read_exact(&mut buf)
            .map_err(|err| format!("{}", err))?;

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
            let message = message.map_err(|err| ParserError {
                inner_error: format!("{}", err).into(),
                message_index: message_index,
                message: None,
            })?;

            self.context.current_tick = message.tick;
            let result: std::result::Result<(), InnerError> = match &message.content {
                Demo::SignOnPacket(packet) => self
                    .handle_packet(packet, event_handler)
                    .map_err(|err| err.into()),
                Demo::Packet(packet) => self
                    .handle_packet(packet, event_handler)
                    .map_err(|err| err.into()),
                Demo::FullPacket(data) => {
                    if let Some(packet) = &data.packet {
                        self.handle_packet(packet, event_handler)
                            .map_err(|err| err.into())
                    } else {
                        Ok(())
                    }
                }
                Demo::ClassInfo(class_info) => self
                    .handle_class_info(class_info)
                    .map_err(|err| format!("{}", err).into()),
                Demo::SendTables(send_tables) => self
                    .context
                    .entities
                    .on_send_tables(send_tables)
                    .map_err(|err| format!("{}", err).into()),
                ref demo => event_handler
                    .on_demo(&demo, &self.context)
                    .map_err(|err| format!("{}", err).into()),
            };

            if let Err(err) = result {
                return Err(ParserError {
                    message_index: message_index,
                    message: Some(message),
                    inner_error: err,
                });
            }

            message_index += 1;
            if !event_handler.should_continue() {
                return Ok(());
            }
        }

        Ok(())
    }

    fn handle_class_info(&mut self, class_info: &CDemoClassInfo) -> Result<()> {
        self.context.entities.on_class_info(class_info)?;
        if let Some(baseline_table) = self
            .context
            .string_tables
            .get_by_name(ENTITY_BASELINES_TABLE_NAME)
        {
            self.context.entities.update_baselines(baseline_table)?;
        }
        Ok(())
    }

    fn handle_packet<T: EventHandler>(
        &mut self,
        packet: &dota::CDemoPacket,
        event_handler: &mut T,
    ) -> std::result::Result<(), PacketError> {
        let packet_reader = PacketReader::new(
            BitReader::new(packet.data()),
            self.packets_handled.to_owned(),
        );

        // We must sort packets as some packets contain data that is needed to process other packets.
        let mut packets = packet_reader
            .collect::<Result<Vec<_>>>()
            .map_err::<PacketError, _>(|err| format!("{}", err).into())?;
        packets.sort_by_key(|packet| match packet.kind {
            PacketKind::CreateStringTable => -2,
            PacketKind::UpdateStringTable => -1,
            _ => 0,
        });

        let mut packet_index = 0i32;

        for packet in packets {
            let result = match &packet.content {
                Packet::CreateStringTable(message) => {
                    let update_baseline = message.name() == ENTITY_BASELINES_TABLE_NAME;
                    if let Err(err) = self.context.string_tables.on_create_string_table(message) {
                        Err(err)
                    } else if update_baseline {
                        self.context.entities.update_baselines(
                            self.context
                                .string_tables
                                .get_by_name(ENTITY_BASELINES_TABLE_NAME)
                                .unwrap(),
                        )
                    } else {
                        Ok(())
                    }
                }
                Packet::UpdateStringTable(message) => self.update_string_table(message),
                Packet::ServerInfo(message) => {
                    let max_classes = message.max_classes().try_into();
                    match max_classes {
                        Ok(max_classes) => {
                            self.context.entities.update_max_classes(max_classes);
                            Ok(())
                        }
                        Err(err) => Err(err.into()),
                    }
                }
                Packet::PacketEntities(entities) => {
                    self.on_packet_entities(entities, event_handler)
                }
                packet => event_handler.on_packet(&packet, &self.context),
            };

            if let Err(err) = result {
                return Err(PacketError {
                    error_message: format!("{}", err),
                    packet_index,
                    packet: Some(packet),
                });
            }

            packet_index += 1;
        }

        Ok(())
    }

    fn update_string_table(&mut self, message: &dota::CsvcMsgUpdateStringTable) -> Result<()> {
        let table_id = message.table_id().try_into()?;
        self.context.string_tables.on_update_string_table(message)?;
        let table = self.context.string_tables.get(table_id).unwrap();
        if table.get_name() == ENTITY_BASELINES_TABLE_NAME {
            self.context.entities.update_baselines(table)?
        }

        Ok(())
    }

    fn on_packet_entities<T: EventHandler>(
        &mut self,
        entities: &dota::CsvcMsgPacketEntities,
        event_handler: &mut T,
    ) -> Result<()> {
        let updated_entities = self.context.entities.on_packet_entities(entities)?;
        for entity_id in updated_entities {
            if let Some(entity) = self.context.entities.get(entity_id.try_into()?) {
                event_handler.on_entity_changed(entity, &self.context)?;
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
