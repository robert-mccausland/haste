use std::{collections::HashMap, rc::Rc};

use haste_protobuf::dota::{self, CsvcMsgFlattenedSerializer};
use prost::Message;

use crate::{
    decoders::Bits,
    readers::{bits::BitReader, entities::EntityReader, serializers::SerializerReader},
    string_tables::StringTable,
    Result,
};

pub mod field_decoders;
mod quantized_float_decoder;

#[derive(Debug, Clone)]
pub struct Class {
    pub id: u32,
    pub name: String,
}

impl Class {
    fn new(id: u32, name: String) -> Self {
        Self { id, name }
    }
}

#[derive(Debug, Clone)]
pub struct Serializer {
    pub name: String,
    pub version: i32,
    pub fields: Vec<Rc<Field>>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub var_name: String,
    pub send_node: String,
    pub encoder: String,
    pub encode_flags: u32,
    pub bit_count: u32,
    pub low_value: Option<f32>,
    pub high_value: Option<f32>,
    pub field_type: FieldType,
    pub serializer_name: String,
    pub serializer_version: i32,
}

#[derive(Debug, Clone)]
pub struct FieldType {
    pub base_type: String,
    pub generic_type: Option<Box<FieldType>>,
    pub pointer: bool,
    pub count: u32,
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    Boolean(bool),
    String(String),
    Unsigned32(u32),
    Signed32(i32),
    Unsigned64(u64),
    Float(f32),
    Vector2((f32, f32)),
    Vector3((f32, f32, f32)),
    Vector4((f32, f32, f32, f32)),
}

pub struct Entity {
    pub serial: u32,
    pub active: bool,
    pub class_id: u32,
}

#[derive(Debug, Clone)]
pub struct FieldPath {
    pub path: [i32; 7],
    pub last: i32,
    pub done: bool,
}

impl FieldPath {
    pub fn pop(&mut self, n: i32) {
        for _ in 0..n {
            self.path[self.last as usize] = 0;
            self.last -= 1;
        }
    }
}

pub struct Entities {
    baselines: HashMap<u32, Rc<[u8]>>,
    max_classes: u32,
    classes: HashMap<u32, Class>,
    class_names: HashMap<String, u32>,
    serializers: HashMap<String, HashMap<i32, Serializer>>,
    entities: HashMap<u32, Entity>,
    full_entity_packet_count: u32,
}

impl Entities {
    pub fn new() -> Self {
        Self {
            baselines: HashMap::new(),
            max_classes: 0,
            classes: HashMap::new(),
            serializers: HashMap::new(),
            entities: HashMap::new(),
            class_names: HashMap::new(),
            full_entity_packet_count: 0,
        }
    }

    pub fn update_baselines(&mut self, baselines: &StringTable) -> Result<()> {
        for baseline in baselines.get_entries() {
            let data = Rc::from(baseline.data.clone());
            self.baselines
                .insert(u32::from_str_radix(baseline.name, 10)?, data);
        }

        Ok(())
    }

    pub fn update_max_classes(&mut self, max_classes: u32) {
        self.max_classes = max_classes
    }

    pub fn on_class_info(&mut self, class_info: dota::CDemoClassInfo) -> Result<()> {
        for class in class_info.classes {
            let id = class.class_id.ok_or("class did not have id")?.try_into()?;
            let class = Class::new(id, class.network_name.ok_or("class did not have name")?);
            self.class_names.insert(class.name.to_owned(), id);
            self.classes.insert(id, class);
        }

        Ok(())
    }

    pub fn on_send_tables(&mut self, send_tables: dota::CDemoSendTables) -> Result<()> {
        let mut data = BitReader::new(send_tables.data());
        let size = data.read_varint_u32()?;
        let mut reader = SerializerReader::new(CsvcMsgFlattenedSerializer::decode(
            data.read_bytes(size.try_into()?).as_slice(),
        )?);

        while let Some(serializer) = reader.read_next_serializer()? {
            self.serializers
                .entry(serializer.name.to_owned())
                .or_insert_with(|| HashMap::new())
                .insert(serializer.version, serializer);
        }

        Ok(())
    }

    pub fn on_server_info(&mut self, server_info: dota::CsvcMsgServerInfo) -> Result<()> {
        self.max_classes = server_info.max_classes().try_into()?;
        Ok(())
    }

    pub fn on_packet_entities(
        &mut self,
        packet_entities: dota::CsvcMsgPacketEntities,
    ) -> Result<()> {
        if !packet_entities.legacy_is_delta() {
            if self.full_entity_packet_count > 0 {
                return Ok(());
            }
            self.full_entity_packet_count += 1;
        }

        let mut entity_reader = EntityReader::new(
            self.max_classes,
            &self.classes,
            &self.serializers,
            &self.baselines,
            &mut self.entities,
        );

        entity_reader.read_entities(
            packet_entities.entity_data(),
            packet_entities.updated_entries().try_into()?,
        )?;

        Ok(())
    }
}
