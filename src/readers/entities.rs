use std::collections::HashMap;

use crate::{
    decoders::Bits,
    entities::{Class, Entity, FieldPath, Serializer},
    utils, Result,
};
use bytes::Buf;

use super::{bits::BitReader, field_paths::FieldPathReader};

pub struct EntityReader<'a> {
    current_index: i32,
    class_id_size: usize,
    classes: &'a HashMap<u32, Class>,
}

impl<'a> EntityReader<'a> {
    pub fn new(max_classes: u32, classes: &'a HashMap<u32, Class>) -> Self {
        let class_id_size = (max_classes as f64).log2().ceil() as usize;
        Self {
            current_index: -1,
            class_id_size,
            classes,
        }
    }

    pub fn read_entities<B: Buf>(&mut self, data: B, count: usize) -> Result<()> {
        let mut reader = BitReader::new(data);
        for _ in 0..count {
            self.read_next_entity(&mut reader)?;
        }

        Ok(())
    }

    fn read_next_entity<B: Buf>(&mut self, data: &mut BitReader<B>) -> Result<()> {
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
                    .ok_or(format!("class {} not found", &class_id))?;

                let mut entity = Entity {
                    serial,
                    class_id,
                    active: true,
                };

                let serializer = utils::expect_one(class.serializers.iter())?.1;
                let mut baseline = BitReader::new(class.baseline.as_slice());

                let field_paths = FieldPathReader::new()
                    .read(&mut baseline)
                    .collect::<Vec<_>>();
                read_fields(&mut baseline, serializer, field_paths.as_slice())?;

                let field_paths = FieldPathReader::new().read(data).collect::<Vec<_>>();
                read_fields(data, serializer, field_paths.as_slice())?;
            }
        }

        todo!();
    }
}

fn read_fields<B: Buf>(
    data: &mut BitReader<B>,
    serializer: &Serializer,
    field_paths: &[FieldPath],
) -> Result<()> {
    todo!();
}
