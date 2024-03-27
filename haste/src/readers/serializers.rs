use std::{collections::HashMap, rc::Rc};

use haste_protobuf::dota::CsvcMsgFlattenedSerializer;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    entities::{Field, FieldType, Serializer},
    utils::transmute_i32_to_u32,
    Result,
};

pub struct SerializerReader {
    data: CsvcMsgFlattenedSerializer,
    field_cache: HashMap<i32, Rc<Field>>,
    current_index: usize,
}

impl SerializerReader {
    pub fn new(data: CsvcMsgFlattenedSerializer) -> Self {
        Self {
            data,
            field_cache: HashMap::new(),
            current_index: 0,
        }
    }

    pub fn read_next_serializer(&mut self) -> Result<Option<Serializer>> {
        // Cache fields because we will get a lot of duplicate fields
        if self.current_index == self.data.serializers.len() {
            return Ok(None);
        }

        let s = self.data.serializers.get(self.current_index).unwrap();
        let mut fields = Vec::with_capacity(s.fields_index.len());
        for field_index in &s.fields_index {
            if !self.field_cache.contains_key(&field_index) {
                self.field_cache
                    .insert(*field_index, Rc::from(self.create_field(*field_index)?));
            }
            fields.push(self.field_cache[&field_index].clone());
        }

        let serializer = Serializer {
            name: self.get_symbol_name(s.serializer_name_sym.ok_or("no serializer name")?)?,
            version: s.serializer_version.ok_or("no serializer version")?,
            fields,
        };

        self.current_index += 1;
        Ok(Some(serializer))
    }

    fn get_symbol_name(&self, index: i32) -> Result<String> {
        let index: usize = index.try_into()?;
        Ok(self
            .data
            .symbols
            .get(index)
            .ok_or_else(|| format!("could not find symbol '{:}'", index))?
            .clone())
    }

    fn create_field(&self, field_index: i32) -> Result<Field> {
        let f = self
            .data
            .fields
            .get(TryInto::<usize>::try_into(field_index)?)
            .ok_or_else(|| format!("could not find field: '{:}'", field_index))?;

        let field_type = parse_field_type(
            self.get_symbol_name(f.var_type_sym.ok_or("no variable type")?)?
                .as_str(),
        )?;

        // TODO - might need to default "(root)" to an empty string here???
        let send_node = f
            .var_name_sym
            .map_or(Ok("".to_owned()), |index| self.get_symbol_name(index))?;

        Ok(Field {
            var_name: self.get_symbol_name(f.var_name_sym.ok_or("no variable Name")?)?,
            send_node: send_node,
            encoder: f
                .var_encoder_sym
                .map_or(Ok("".to_owned()), |index| self.get_symbol_name(index))?,
            encode_flags: transmute_i32_to_u32(f.encode_flags.unwrap_or_default()),
            bit_count: f.bit_count.unwrap_or_default().try_into()?,
            low_value: f.low_value,
            high_value: f.high_value,
            field_type: field_type,
            serializer_name: f
                .field_serializer_name_sym
                .map_or(Ok("".to_owned()), |index| self.get_symbol_name(index))?,
            serializer_version: f.field_serializer_version.unwrap_or_default(),
        })
    }
}
static ITEM_COUNTS: Lazy<HashMap<&str, u32>> =
    Lazy::new(|| HashMap::from([("MAX_ITEM_STOCKS", 8), ("MAX_ABILITY_DRAFT_ABILITIES", 48)]));

static FIELD_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([^<\[*]+)(<\s(.*)\s>)?(\*)?(\[(.*)\])?").unwrap());

fn parse_field_type(name: &str) -> Result<FieldType> {
    let captures = FIELD_REGEX
        .captures(name)
        .ok_or("field type did not match expected regex")?;

    return Ok(FieldType {
        base_type: captures.get(1).unwrap().as_str().to_owned(),
        generic_type: if captures.get(3).is_none() {
            None
        } else {
            Some(Box::new(parse_field_type(
                captures.get(3).unwrap().as_str(),
            )?))
        },
        pointer: captures.get(4).is_some(),
        count: if captures.get(6).is_none() {
            0
        } else if let Some(count) = ITEM_COUNTS.get(captures.get(6).unwrap().as_str()) {
            *count
        } else if let Ok(count) = captures.get(6).unwrap().as_str().parse::<u32>() {
            count
        } else {
            1024
        },
    });
}
