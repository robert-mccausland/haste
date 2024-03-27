use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{
    decoders::Bits,
    entities::{
        field_decoders::{decode_field, decode_field_by_type, read_boolean, read_unsigned},
        Class, Entity, FieldPath, FieldValue, Serializer,
    },
    utils, Result,
};

use once_cell::sync::Lazy;

use super::{bits::BitReader, field_paths::read_field_paths};

static POINTER_TYPES: Lazy<HashSet<&str>> = Lazy::new(|| {
    HashSet::from([
        "PhysicsRagdollPose_t",
        "CBodyComponent",
        "CEntityIdentity",
        "CPhysicsComponent",
        "CRenderComponent",
        "CDOTAGamerules",
        "CDOTAGameManager",
        "CDOTASpectatorGraphManager",
        "CPlayerLocalData",
        "CPlayer_CameraServices",
        "CDOTAGameRules",
    ])
});

static VARIABLE_ARRAY_TYPES: Lazy<HashSet<&str>> =
    Lazy::new(|| HashSet::from(["CUtlVector", "CNetworkUtlVectorBase"]));

pub struct EntityReader<'a> {
    current_index: i32,
    class_id_size: usize,
    classes: &'a HashMap<u32, Class>,
    serializers: &'a HashMap<String, HashMap<i32, Serializer>>,
    baselines: &'a HashMap<u32, Rc<[u8]>>,
    entities: &'a mut HashMap<u32, Entity>,
}

impl<'a> EntityReader<'a> {
    pub fn new(
        max_classes: u32,
        classes: &'a HashMap<u32, Class>,
        serializers: &'a HashMap<String, HashMap<i32, Serializer>>,
        baselines: &'a HashMap<u32, Rc<[u8]>>,
        entities: &'a mut HashMap<u32, Entity>,
    ) -> Self {
        let class_id_size = (max_classes as f64).log2().ceil() as usize;
        Self {
            current_index: -1,
            class_id_size,
            classes,
            serializers,
            baselines,
            entities,
        }
    }

    pub fn read_entities(&mut self, data: &[u8], count: usize) -> Result<()> {
        let mut reader = BitReader::new(data);
        for _ in 0..count {
            self.read_next_entity(&mut reader)?;
        }

        Ok(())
    }

    fn read_next_entity<'b>(&mut self, data: &'b mut BitReader) -> Result<()> {
        self.current_index += TryInto::<i32>::try_into(data.read_varbit())? + 1;
        let command = data.read_bits(2);

        let is_deactivate = command & 1 != 0;
        let is_create = command & 2 != 0;

        if !is_deactivate {
            if is_create {
                let class_id = data.read_bits(self.class_id_size).try_into()?;
                let serial = data.read_bits(17).try_into()?;

                // Some extra data that we do not currently use
                let _extra = data.read_varint_u32();
                let class = self
                    .classes
                    .get(&class_id)
                    .ok_or_else(|| format!("class {} not found", &class_id))?;

                let entity = Entity {
                    serial,
                    class_id,
                    active: true,
                };

                let serializer = utils::expect_one(
                    self.serializers
                        .get(&class.name)
                        .ok_or_else(|| {
                            format!("unable to find serializer for class: '{:}'", &class.name)
                        })?
                        .iter(),
                )?
                .1;

                let baseline_data = self
                    .baselines
                    .get(&class.id)
                    .ok_or_else(|| format!("unable to find baseline for class: '{}'", &class.name))?
                    .as_ref();
                let mut baseline = BitReader::new(baseline_data);
                self.read_fields(&mut baseline, serializer)?;
                self.read_fields(data, serializer)?;
                self.entities.insert(self.current_index.try_into()?, entity);
            } else {
                let entity = self
                    .entities
                    .get(&self.current_index.try_into()?)
                    .ok_or_else(|| {
                        format!("unable to find entity with index: {}", &self.current_index)
                    })?;

                let class = self
                    .classes
                    .get(&entity.class_id)
                    .ok_or_else(|| format!("class {} not found", &entity.class_id))?;

                let serializer = utils::expect_one(
                    self.serializers
                        .get(&class.name)
                        .ok_or_else(|| {
                            format!("unable to find serializer for class: '{}'", &class.name)
                        })?
                        .iter(),
                )?
                .1;

                self.read_fields(data, serializer)?;
            }
        } else {
            let entity = self
                .entities
                .get(&self.current_index.try_into()?)
                .ok_or_else(|| {
                    format!("unable to find entity with index: {}", &self.current_index)
                })?;
            if !entity.active {
                return Err("Entity already inactive".into());
            }

            if is_create {
                self.entities.remove(&self.current_index.try_into()?);
            }
        }

        Ok(())
    }

    fn read_fields<'b>(&mut self, data: &'b mut BitReader, serializer: &Serializer) -> Result<()> {
        for field_path in read_field_paths(data) {
            let _value = self.parse_field(data, serializer, &field_path, 0)?;

            //println!("{:?}", value);

            // TODO actually use this value...
            // There is probably some improvements to what this value is, currently it may not
            // really represent what the field actually is, until we decode deeper into the field
        }

        Ok(())
    }

    // TODO this method needs cleaning up:
    // - Remove casts and unwraps that can cause panics if we get invalid data
    // - determine better way of passing the field serializers to this method, other than
    // referencing them by name
    // - Maybe we even need a better way of deciding how to decode the different types of values
    fn parse_field(
        &mut self,
        data: &mut BitReader,
        serializer: &Serializer,
        field_path: &FieldPath,
        position: usize,
    ) -> Result<FieldValue> {
        // TODO stop this from potentially panicking...
        let index: usize = field_path.path[position] as usize;
        let field = serializer.fields.get(index).unwrap();

        // Conditional logic based on the field (this is kind ugly...)
        if field.serializer_name != "" {
            if field.field_type.pointer
                || HashSet::contains(&POINTER_TYPES, field.field_type.base_type.as_str())
            {
                if field_path.last == position as i32 {
                    return Ok(read_boolean(data));
                } else {
                    let serializer = self
                        .serializers
                        .get(&field.serializer_name)
                        .and_then(|s| s.get(&field.serializer_version))
                        .ok_or_else(|| {
                            format!(
                                "unable to find serializer: '{:}@{:}'",
                                &field.serializer_name, &field.serializer_version
                            )
                        })?;
                    return self.parse_field(data, serializer, field_path, position + 1);
                }
            } else {
                if field_path.last >= position as i32 + 2 {
                    let serializer = self
                        .serializers
                        .get(&field.serializer_name)
                        .and_then(|s| s.get(&field.serializer_version))
                        .ok_or_else(|| {
                            format!(
                                "unable to find serializer: '{:}@{:}'",
                                &field.serializer_name, &field.serializer_version
                            )
                        })?;
                    return self.parse_field(data, serializer, field_path, position + 2);
                } else {
                    return read_unsigned(data);
                }
            }
        } else if field.field_type.count > 0 && field.field_type.base_type != "char" {
            return decode_field(data, field.as_ref());
        } else if HashSet::contains(&VARIABLE_ARRAY_TYPES, field.field_type.base_type.as_str()) {
            if field_path.last == position as i32 + 1 {
                return decode_field_by_type(
                    data,
                    field
                        .field_type
                        .generic_type
                        .as_ref()
                        .unwrap()
                        .base_type
                        .as_ref(),
                );
            } else {
                return read_unsigned(data);
            }
        } else {
            return decode_field(data, field.as_ref());
        }
    }
}
