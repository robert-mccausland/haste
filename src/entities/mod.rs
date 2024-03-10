use std::collections::HashMap;

use crate::{
    protos::dota,
    readers::{bits::BitReader, entities::EntityReader},
    string_tables::StringTable,
    Result,
};

pub mod field_decoders;

#[derive(Debug, Clone)]
pub struct Class {
    pub id: u32,
    pub name: String,
    pub baseline: Vec<u8>,
    pub serializers: HashMap<i32, Serializer>,
}

impl Class {
    fn new(id: u32, name: String, baseline: Vec<u8>) -> Self {
        Self {
            id,
            name,
            baseline,
            serializers: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Serializer {
    pub name: String,
    pub version: i32,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub var_name: String,
    pub var_type: String,
    pub send_node: String,
    pub encoder: String,
    pub encode_flags: u32,
    pub bit_count: u32,
    pub low_value: Option<f32>,
    pub high_value: Option<f32>,
    pub field_type: FieldType,
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

pub struct Entities<'a> {
    classes: HashMap<u32, Class>,
    baselines: &'a StringTable,
    max_classes: u32,
    full_entity_packet_count: u32,
}

impl<'a> Entities<'a> {
    pub fn on_class_info(&mut self, class_info: dota::CDemoClassInfo) -> Result<()> {
        for class in class_info.classes {
            let id = class.class_id.ok_or("class did not have id")?.try_into()?;
            let class = Class::new(
                id,
                class.network_name.ok_or("class did not have name")?,
                self.baselines
                    .get_entry_by_name(&id.to_string())
                    .map(|entry| entry.data.clone())
                    .unwrap_or(vec![]),
            );

            self.classes.insert(id, class);
        }

        Ok(())
    }

    pub fn on_send_tables(&mut self, send_tables: dota::CDemoSendTables) {
        // TODO write serializer reader
    }

    pub fn on_server_info(&mut self, server_info: dota::CsvcMsgServerInfo) -> Result<()> {
        self.max_classes = server_info.max_classes().try_into()?;
        Ok(())
    }

    pub fn on_packet_entities(
        &mut self,
        packet_entities: dota::CsvcMsgPacketEntities,
    ) -> Result<()> {
        if packet_entities.legacy_is_delta() {
            if self.full_entity_packet_count > 0 {
                return Ok(());
            }
            self.full_entity_packet_count += 1;
        }

        let mut entity_reader = EntityReader::new(self.max_classes, &self.classes);
        entity_reader.read_entities(
            packet_entities.entity_data(),
            packet_entities.updated_entries().try_into()?,
        )?;

        Ok(())
    }

    pub fn on_baseline_changes(&mut self) -> Result<()> {
        for entry in self.baselines.get_entries() {
            let class_id = u32::from_str_radix(entry.name, 10)?;
            if let Some(class) = self.classes.get_mut(&class_id) {
                class.baseline = entry.data.clone();
            }
        }

        Ok(())
    }
}
