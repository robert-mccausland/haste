use std::{collections::HashMap, rc::Rc};

use fxhash::FxHashMap;
use haste_protobuf::dota::{self, CsvcMsgFlattenedSerializer};
use prost::Message;

use crate::{
    decoders::Bits,
    readers::{bits::BitReader, entities::EntityReader, serializers::read_serializers},
    string_tables::StringTable,
    utils, Result,
};

pub mod field_decoders;
mod quantized_float_decoder;

#[derive(Debug, Clone)]
pub struct Class {
    pub id: u32,
    pub name: String,
    pub serializer: Rc<Serializer>,
    pub baseline: Vec<u8>,
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
    pub serializer: Option<Rc<Serializer>>,
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
    max_classes: u32,
    full_entity_packet_count: u32,
    classes: FxHashMap<u32, Class>,
    entities: FxHashMap<u32, Entity>,
    serializers: HashMap<String, HashMap<i32, Rc<Serializer>>>,
}

impl Entities {
    pub fn new() -> Self {
        Self {
            max_classes: 0,
            full_entity_packet_count: 0,
            classes: FxHashMap::default(),
            entities: FxHashMap::default(),
            serializers: HashMap::new(),
        }
    }

    pub fn update_baselines(&mut self, baselines: &StringTable) -> Result<()> {
        // Skip this if we have not had any class data yet
        if self.classes.len() == 0 {
            return Ok(());
        }

        for baseline in baselines.get_entries() {
            let class = self
                .classes
                .get_mut(&u32::from_str_radix(baseline.name, 10)?)
                .ok_or("could not find class")?;

            class.baseline = baseline.data.clone();
        }

        Ok(())
    }

    pub fn update_max_classes(&mut self, max_classes: u32) {
        self.max_classes = max_classes
    }

    pub fn on_class_info(&mut self, class_info: dota::CDemoClassInfo) -> Result<()> {
        for class in class_info.classes {
            let id = class.class_id.ok_or("class did not have id")?.try_into()?;
            let name = class.network_name.ok_or("class did not have name")?;
            let class = Class {
                id,
                serializer: utils::expect_one(
                    self.serializers
                        .get(&name)
                        .ok_or("could not find class serializer")?
                        .values(),
                )?
                .clone(),
                baseline: vec![],
                name,
            };
            self.classes.insert(id, class);
        }

        Ok(())
    }

    pub fn on_send_tables(&mut self, send_tables: dota::CDemoSendTables) -> Result<()> {
        let mut data = BitReader::new(send_tables.data());
        let size = data.read_varint_u32()?;
        self.serializers = read_serializers(&CsvcMsgFlattenedSerializer::decode(
            data.read_bytes(size.try_into()?).as_slice(),
        )?)?;

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

        let mut entity_reader =
            EntityReader::new(self.max_classes, &self.classes, &mut self.entities);

        entity_reader.read_entities(
            packet_entities.entity_data(),
            packet_entities.updated_entries().try_into()?,
        )?;

        Ok(())
    }
}
