use std::{cell::RefCell, mem::MaybeUninit, ops::Deref, rc::Rc};

use crate::{
    decoders::Bits,
    entities::{Class, Entity, FieldPath, FieldState, Serializer},
    huffman_tree,
    readers::bits::BitReader,
    Result,
};

use fxhash::FxHashMap;

use super::{
    field_decoders::{decode_boolean, decode_field, decode_field_by_type, decode_unsigned},
    field_paths::FIELD_PATH_TREE,
    FieldModel, FieldValue,
};

pub struct EntityReader<'a> {
    current_index: i32,
    class_id_size: usize,
    classes: &'a FxHashMap<u32, Rc<RefCell<Class>>>,
    entities: &'a mut FxHashMap<u32, Entity>,
    paths_cache: [MaybeUninit<FieldPath>; 2048],
    data: BitReader<'a>,
}

impl<'a> EntityReader<'a> {
    pub fn new(
        max_classes: u32,
        classes: &'a FxHashMap<u32, Rc<RefCell<Class>>>,
        entities: &'a mut FxHashMap<u32, Entity>,
        data: &'a [u8],
    ) -> Self {
        let class_id_size = (max_classes as f64).log2().ceil() as usize;
        Self {
            current_index: -1,
            class_id_size,
            classes,
            entities,
            paths_cache: unsafe { MaybeUninit::uninit().assume_init() },
            data: BitReader::new(data),
        }
    }

    pub fn read_next_entity(&mut self) -> Result<i32> {
        self.current_index += TryInto::<i32>::try_into(self.data.read_varbit())? + 1;
        let command = self.data.read_bits(2);

        let is_deactivate = command & 1 != 0;
        let is_create = command & 2 != 0;

        if !is_deactivate {
            if is_create {
                let class_id = self.data.read_bits(self.class_id_size).try_into()?;
                let serial = self.data.read_bits(17).try_into()?;

                // Some extra data that we do not currently use
                let _extra = self.data.read_varint_u32();
                let class = self
                    .classes
                    .get(&class_id)
                    .ok_or_else(|| format!("class {} not found", &class_id))?;

                let mut entity = Entity {
                    serial,
                    class: class.clone(),
                    active: true,
                    fields: Vec::new(),
                };

                let baseline = &class.borrow().baseline;
                let mut baseline = BitReader::new(baseline);
                Self::read_fields(
                    &mut baseline,
                    class.borrow().serializer.as_ref(),
                    &mut entity.fields,
                    &mut self.paths_cache,
                )?;
                Self::read_fields(
                    &mut self.data,
                    class.borrow().serializer.as_ref(),
                    &mut entity.fields,
                    &mut self.paths_cache,
                )?;
                self.entities.insert(self.current_index.try_into()?, entity);
            } else {
                let index = self.current_index.try_into()?;
                let mut entity = self.entities.remove(&index).ok_or_else(|| {
                    format!("unable to find entity with index: {}", &self.current_index)
                })?;

                Self::read_fields(
                    &mut self.data,
                    entity.class.borrow().serializer.as_ref(),
                    &mut entity.fields,
                    &mut self.paths_cache,
                )?;

                self.entities.insert(index, entity);
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

        Ok(self.current_index)
    }

    fn read_fields<'b>(
        data: &'b mut BitReader,
        serializer: &Serializer,
        entity_fields: &mut Vec<FieldState>,
        paths_cache: &mut [MaybeUninit<FieldPath>; 2048],
    ) -> Result<()> {
        let count = read_field_paths(data, paths_cache);

        for index in 0..count {
            let field_path = unsafe { paths_cache[index].assume_init_ref() };
            let value = Self::parse_field(data, serializer, field_path, 0)?;
            let mut fields = &mut *entity_fields;
            for (index, position) in field_path.path.iter().enumerate() {
                while fields.get(*position as usize).is_none() {
                    fields.push(FieldState {
                        children: None,
                        value: None,
                    })
                }
                let state = &mut fields[*position as usize];

                if index == field_path.last {
                    state.value = Some(value);
                    break;
                }

                if state.children.is_none() {
                    state.children = Some(Vec::new());
                }

                fields = state.children.as_mut().unwrap();
            }
            unsafe { paths_cache[index].assume_init_drop() }
        }

        Ok(())
    }

    fn parse_field(
        data: &mut BitReader,
        serializer: &Serializer,
        field_path: &FieldPath,
        position: usize,
    ) -> Result<FieldValue> {
        let index: usize = (*field_path
            .path
            .get(position)
            .ok_or("invalid field path position")?)
        .try_into()?;

        let field = serializer.fields.get(index).unwrap();
        match field.model {
            FieldModel::FixedTable => {
                if field_path.last == position as usize {
                    return Ok(decode_boolean(data));
                } else {
                    return Self::parse_field(
                        data,
                        field.serializer.as_ref().unwrap(),
                        field_path,
                        position + 1,
                    );
                }
            }
            FieldModel::VariableTable => {
                if field_path.last >= position as usize + 2 {
                    return Self::parse_field(
                        data,
                        field.serializer.as_ref().unwrap(),
                        field_path,
                        position + 2,
                    );
                } else {
                    return decode_unsigned(data);
                }
            }
            FieldModel::FixedArray => decode_field(data, field.as_ref()),
            FieldModel::VariableArray => {
                if field_path.last == position as usize + 1 {
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
                    return decode_unsigned(data);
                }
            }
            FieldModel::Simple => decode_field(data, field.as_ref()),
        }
    }
}

fn read_field_paths(data: &mut BitReader, output: &mut [MaybeUninit<FieldPath>]) -> usize {
    let mut node = FIELD_PATH_TREE.deref();
    let mut index = 0;
    let mut current = FieldPath {
        last: 0,
        path: [-1, 0, 0, 0, 0, 0, 0],
    };

    loop {
        match &node.content {
            huffman_tree::NodeContent::Leaf(value) => {
                (value.operation)(data, &mut current);
                if value.is_final {
                    return index;
                } else {
                    output[index] = MaybeUninit::new(current.clone());
                    index += 1;
                    node = FIELD_PATH_TREE.deref();
                };
            }
            huffman_tree::NodeContent::Parent(left, right) => {
                node = if data.read_boolean() { right } else { left }
            }
        }
    }
}
