use crate::Result;
use crate::{decoders::Bits, readers::bits::BitReader};

use self::fields::{
    decode_f32, decode_f32_no_scale, decode_f32_quantized, decode_q_angle, decode_u64,
    decode_vector2, decode_vector3, decode_vector4,
};

use super::{Field, FieldValue};

mod fields;
mod quantized_float;

pub fn decode_field(data: &mut BitReader, field: &Field) -> Result<FieldValue> {
    let field_type = field.field_type.base_type.as_str();
    match field.field_type.base_type.as_str() {
        "float32" => return Ok(FieldValue::Float(decode_f32(data, field)?)),
        "CNetworkedQuantizedFloat" => {
            return Ok(FieldValue::Float(decode_f32_quantized(data, field)?))
        }
        "Vector" => return Ok(FieldValue::Vector3(decode_vector3(data, field)?)),
        "Vector2D" => return Ok(FieldValue::Vector2(decode_vector2(data, field)?)),
        "Vector4D" => return Ok(FieldValue::Vector4(decode_vector4(data, field)?)),
        "uint64" | "CStrongHandle" => return Ok(FieldValue::Unsigned64(decode_u64(data, field)?)),
        "QAngle" => return Ok(FieldValue::Vector3(decode_q_angle(data, field))),
        "CEntityHandle" | "CHandle" => return decode_unsigned(data),
        _ => {}
    }

    return decode_field_by_type(data, field_type);
}

pub fn decode_field_by_type(data: &mut BitReader, field_type: &str) -> Result<FieldValue> {
    match field_type {
        "bool" | "CBodyComponent" | "CPhysicsComponent" | "CRenderComponent" => {
            Ok(FieldValue::Boolean(data.read_boolean()))
        }
        "char" | "CUtlString" | "CUtlSymbolLarge" => Ok(FieldValue::String(data.read_string()?)),
        "int16" | "int32" | "int64" | "AbilityID_t" | "PlayerID_t" => {
            Ok(FieldValue::Signed32(data.read_varint_i32()?))
        }
        "GameTime_t" => Ok(FieldValue::Float(decode_f32_no_scale(data))),
        "MatchID_t" | "itemid_t" => Ok(FieldValue::Unsigned64(data.read_varint_u64()?)),
        _ => decode_unsigned(data),
    }
}

pub fn decode_boolean(data: &mut BitReader) -> FieldValue {
    FieldValue::Boolean(data.read_boolean())
}

pub fn decode_unsigned(data: &mut BitReader) -> Result<FieldValue> {
    let value = data.read_varint_u32()?;
    return Ok(FieldValue::Unsigned32(value));
}
