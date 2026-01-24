use std::{cell::RefCell, collections::HashMap, fmt::Debug, rc::Rc};

use ahash::AHashMap;
use haste_protobuf::dota::{self, CsvcMsgFlattenedSerializer};
use prost::Message;

use crate::{Result, decoders::Bits, readers::bits::BitReader, string_tables::StringTable, utils};

use self::{entity_reader::EntityReader, field_paths::FieldPath, serializers::read_serializers};

mod entity_reader;
mod field_decoders;
mod field_paths;
mod serializers;

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

#[derive(Debug)]
pub struct Field {
    pub var_name: String,
    pub encoder: String,
    pub encode_flags: u32,
    pub bit_count: u32,
    pub low_value: Option<f32>,
    pub high_value: Option<f32>,
    pub field_type: FieldType,
    pub serializer: Option<Rc<Serializer>>,
    pub model: FieldModel,
}

#[derive(Debug)]
pub enum FieldModel {
    FixedTable,
    VariableTable,
    FixedArray,
    VariableArray,
    Simple,
}

#[derive(Debug)]
pub struct FieldType {
    pub base_type: String,
    pub generic_type: Option<Box<FieldType>>,
    pub pointer: bool,
    pub count: u32,
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    Empty,
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

#[derive(Debug, Clone)]
pub struct Entity {
    pub serial: u32,
    pub active: bool,
    pub class: Rc<RefCell<Class>>,
    pub fields: Vec<FieldState>,
}

impl Entity {
    pub fn get_field(&self, mut path: Vec<&str>) -> Option<&FieldValue> {
        path.reverse();
        self.get_field_recursive(path, &self.fields, &self.class.borrow().serializer)
    }

    fn get_field_recursive<'a>(
        &self,
        mut path: Vec<&str>,
        fields: &'a Vec<FieldState>,
        serializer: &Serializer,
    ) -> Option<&'a FieldValue> {
        let field_name = path.pop()?;
        let field = serializer
            .fields
            .iter()
            .enumerate()
            .find(|(_index, field)| field.var_name == field_name);

        if let Some((index, field)) = field {
            let field_value = &fields[index];
            if path.len() == 0 {
                field_value.value.as_ref()
            } else {
                let serializer = field.serializer.as_ref()?;
                let children = field_value.children.as_ref()?;
                if field.field_type.count != 0 {
                    let index = path.pop().unwrap().parse::<usize>().ok()?;
                    let state = children.get(index)?;
                    if path.len() == 0 {
                        state.value.as_ref()
                    } else {
                        self.get_field_recursive(path, state.children.as_ref()?, serializer)
                    }
                } else {
                    self.get_field_recursive(path, children, serializer)
                }
            }
        } else {
            None
        }
    }

    pub fn dump_fields(&self) {
        self.dump_fields_recursive(&self.fields, &self.class.borrow().serializer, Vec::new());
    }

    fn dump_fields_recursive(
        &self,
        fields: &Vec<FieldState>,
        serializer: &Serializer,
        mut prefixes: Vec<String>,
    ) -> Vec<String> {
        for index in 0..serializer.fields.len() {
            let field = &serializer.fields[index];
            if fields.len() <= index {
                continue;
            }

            prefixes.push(field.var_name.to_owned());
            println!(
                "{:}: {:?}",
                prefixes.join("."),
                fields[index].value.as_ref()
            );

            if let Some(child_fields) = fields[index].children.as_ref() {
                if let Some(serializer) = field.serializer.as_ref() {
                    if field.field_type.count > 0 {
                        for index in 0..field.field_type.count {
                            prefixes.push(index.to_string());
                            if let Some(element) = child_fields.get(index as usize) {
                                if let Some(child_fields) = &element.children {
                                    prefixes = self.dump_fields_recursive(
                                        child_fields,
                                        serializer,
                                        prefixes,
                                    );
                                }
                            }
                            prefixes.pop();
                        }
                    } else {
                        prefixes = self.dump_fields_recursive(child_fields, serializer, prefixes);
                    }
                } else {
                    for (index, field) in child_fields.iter().enumerate() {
                        println!(
                            "{:}: {:?}",
                            prefixes.join(".") + &format!(".{}", index),
                            &field.value
                        );
                    }
                }
            }
            prefixes.pop();
        }

        prefixes
    }
}

#[derive(Debug, Clone)]
pub struct FieldState {
    pub value: Option<FieldValue>,
    pub children: Option<Vec<FieldState>>,
}

#[derive(Debug)]
pub struct Entities {
    max_classes: u32,
    full_entity_packet_count: u32,
    classes: AHashMap<u32, Rc<RefCell<Class>>>,
    entities: AHashMap<u32, Entity>,
    serializers: HashMap<String, HashMap<i32, Rc<Serializer>>>,
}

impl Entities {
    pub fn new() -> Self {
        Self {
            max_classes: 0,
            full_entity_packet_count: 0,
            classes: AHashMap::default(),
            entities: AHashMap::default(),
            serializers: HashMap::new(),
        }
    }

    pub fn get(&self, entity_id: u32) -> Option<&Entity> {
        self.entities.get(&entity_id)
    }

    pub fn update_baselines(&mut self, baselines: &StringTable) -> Result<()> {
        // Skip this if we have not had any class data yet
        if self.classes.len() == 0 {
            return Ok(());
        }

        for baseline in baselines.get_entries() {
            let class = self
                .classes
                .get_mut(&u32::from_str_radix(baseline.name, 10).map_err(|_err| {
                    format!("invalid class id in baseline table: {:}", baseline.name)
                })?)
                .ok_or("could not find class")?;

            class.borrow_mut().baseline = baseline.data.clone();
        }

        Ok(())
    }

    pub fn update_max_classes(&mut self, max_classes: u32) {
        self.max_classes = max_classes
    }

    pub fn on_class_info(&mut self, class_info: &dota::CDemoClassInfo) -> Result<()> {
        for class in &class_info.classes {
            let id = class.class_id.ok_or("class did not have id")?.try_into()?;
            let name = class
                .network_name
                .as_ref()
                .ok_or("class did not have name")?;
            let class = Rc::from(RefCell::from(Class {
                id,
                serializer: utils::expect_one(
                    self.serializers
                        .get(name)
                        .ok_or("could not find class serializer")?
                        .values(),
                )?
                .clone(),
                baseline: vec![],
                name: name.to_string(),
            }));
            self.classes.insert(id, class);
        }

        Ok(())
    }

    pub fn on_send_tables(&mut self, send_tables: &dota::CDemoSendTables) -> Result<()> {
        let mut data = BitReader::new(send_tables.data());
        let size = data.read_varint_u32()?;
        self.serializers = read_serializers(&CsvcMsgFlattenedSerializer::decode(
            data.read_bytes(size.try_into()?).as_slice(),
        )?)?;

        Ok(())
    }

    pub fn on_server_info(&mut self, server_info: &dota::CsvcMsgServerInfo) -> Result<()> {
        self.max_classes = server_info.max_classes().try_into()?;
        Ok(())
    }

    pub fn on_packet_entities(
        &mut self,
        packet_entities: &dota::CsvcMsgPacketEntities,
    ) -> Result<Vec<i32>> {
        if !packet_entities.legacy_is_delta() {
            if self.full_entity_packet_count > 0 {
                return Ok(Vec::new());
            }
            self.full_entity_packet_count += 1;
        }

        let mut entity_reader = EntityReader::new(
            self.max_classes,
            &self.classes,
            &mut self.entities,
            packet_entities.entity_data(),
        );

        let mut result = Vec::new();
        for _ in 0..packet_entities.updated_entries() {
            result.push(entity_reader.read_next_entity()?);
        }

        Ok(result)
    }
}
