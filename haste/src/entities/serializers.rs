use std::{collections::HashMap, rc::Rc};

use haste_protobuf::dota::CsvcMsgFlattenedSerializer;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    entities::{Field, FieldType, Serializer},
    utils::transmute_i32_to_u32,
    Result,
};

use super::FieldModel;

static POINTER_TYPES: [&str; 11] = [
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
];

pub fn read_serializers(
    data: &CsvcMsgFlattenedSerializer,
) -> Result<HashMap<String, HashMap<i32, Rc<Serializer>>>> {
    let mut field_cache = HashMap::new();
    let mut serializers = HashMap::new();

    for s in &data.serializers {
        let mut fields = Vec::with_capacity(s.fields_index.len());
        for field_index in &s.fields_index {
            if !field_cache.contains_key(&field_index) {
                field_cache.insert(
                    field_index,
                    Rc::from(create_field(data, *field_index, &mut serializers)?),
                );
            }
            fields.push(field_cache[&field_index].clone());
        }

        let serializer = Serializer {
            name: get_symbol_name(data, s.serializer_name_sym.ok_or("no serializer name")?)?,
            version: s.serializer_version.ok_or("no serializer version")?,
            fields,
        };

        serializers
            .entry(serializer.name.clone())
            .or_insert_with(|| HashMap::new())
            .insert(serializer.version, Rc::new(serializer));
    }

    Ok(serializers)
}

fn create_field(
    data: &CsvcMsgFlattenedSerializer,
    field_index: i32,
    serializers: &HashMap<String, HashMap<i32, Rc<Serializer>>>,
) -> Result<Field> {
    let f = data
        .fields
        .get(TryInto::<usize>::try_into(field_index)?)
        .ok_or_else(|| format!("could not find field: '{:}'", field_index))?;

    let field_type = parse_field_type(
        get_symbol_name(data, f.var_type_sym.ok_or("no variable type")?)?.as_str(),
    )?;

    let serializer = match f.field_serializer_name_sym {
        Some(symbol) => Some(
            serializers
                .get(&get_symbol_name(data, symbol)?)
                .ok_or("could not find serializer")?
                .get(&f.field_serializer_version())
                .ok_or("could not find serializer version")?
                .clone(),
        ),
        None => None,
    };
    Ok(Field {
        var_name: get_symbol_name(data, f.var_name_sym.ok_or("no variable name")?)?,
        encoder: f
            .var_encoder_sym
            .map_or(Ok("".to_owned()), |index| get_symbol_name(data, index))?,
        encode_flags: transmute_i32_to_u32(f.encode_flags.unwrap_or_default()),
        bit_count: f.bit_count.unwrap_or_default().try_into()?,
        low_value: f.low_value,
        high_value: f.high_value,
        model: get_field_model(serializer.is_some(), &field_type),
        field_type: field_type,
        serializer,
    })
}

fn get_symbol_name(data: &CsvcMsgFlattenedSerializer, index: i32) -> Result<String> {
    let index: usize = index.try_into()?;
    Ok(data
        .symbols
        .get(index)
        .ok_or_else(|| format!("could not find symbol '{:}'", index))?
        .clone())
}

static ITEM_COUNTS: Lazy<HashMap<&str, u32>> =
    Lazy::new(|| HashMap::from([("MAX_ITEM_STOCKS", 8), ("MAX_ABILITY_DRAFT_ABILITIES", 48)]));

static FIELD_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([^<\[*]+)(<\s(.*)\s>)?(\*)?(\[(.*)\])?").unwrap());

fn parse_field_type(name: &str) -> Result<FieldType> {
    let captures = FIELD_REGEX
        .captures(name)
        .ok_or("field type did not match expected regex")?;

    let base_type = captures.get(1).unwrap().as_str();
    return Ok(FieldType {
        generic_type: if captures.get(3).is_none() {
            None
        } else {
            Some(Box::new(parse_field_type(
                captures.get(3).unwrap().as_str(),
            )?))
        },
        pointer: captures.get(4).is_some() || POINTER_TYPES.contains(&base_type),
        count: if captures.get(6).is_none() {
            0
        } else if let Some(count) = ITEM_COUNTS.get(captures.get(6).unwrap().as_str()) {
            *count
        } else if let Ok(count) = captures.get(6).unwrap().as_str().parse::<u32>() {
            count
        } else {
            1024
        },
        base_type: base_type.to_owned(),
    });
}

fn get_field_model(has_serializer: bool, field_type: &FieldType) -> FieldModel {
    if has_serializer {
        if field_type.pointer {
            FieldModel::FixedTable
        } else {
            FieldModel::VariableTable
        }
    } else if field_type.count > 0 && field_type.base_type != "char" {
        FieldModel::FixedArray
    } else if field_type.base_type == "CUtilVector"
        || field_type.base_type == "CNetworkUtlVectorBase"
    {
        FieldModel::VariableArray
    } else {
        FieldModel::Simple
    }
}
