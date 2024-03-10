use bytes::Buf;

use crate::{decoders::Bits, entities::FieldValue, readers::bits::BitReader, Result};

use super::Field;

pub fn decode_field<B: Buf>(data: &mut BitReader<B>, field: &Field) -> Result<FieldValue> {
    let field_type = field.field_type.base_type.as_str();
    match field.field_type.base_type.as_str() {
        "float32" => return Ok(FieldValue::Float(decode_f32(data, field)?)),
        "CNetworkedQuantizedFloat" => {
            return Ok(FieldValue::Float(decode_f32_quantized(data, field)))
        }
        "Vector" => return Ok(FieldValue::Vector3(decode_vector3(data, field)?)),
        "Vector2D" => return Ok(FieldValue::Vector2(decode_vector2(data, field)?)),
        "Vector4D" => return Ok(FieldValue::Vector4(decode_vector4(data, field)?)),
        "uint64" | "CStrongHandle" => return Ok(FieldValue::Unsigned64(decode_u64(data, field)?)),
        "QAngle" => return Ok(FieldValue::Vector3(decode_q_angle(data, field))),
        "CEntityHandle" | "CHandle" => return Ok(FieldValue::Unsigned32(data.read_varint_u32()?)),
        _ => {}
    }

    return decode_field_by_type(data, field_type);
}

pub fn decode_field_by_type<B: Buf>(
    data: &mut BitReader<B>,
    field_type: &str,
) -> Result<FieldValue> {
    match field_type {
        "bool" | "CBodyComponent" | "CPhysicsComponent" | "CRenderComponent" => {
            Ok(FieldValue::Boolean(data.read_boolean()))
        }
        "char" | "CUtlString" | "CUtlSymbolLarge" => Ok(FieldValue::String(data.read_string()?)),
        "int16" | "int32" | "int64" => Ok(FieldValue::Signed32(data.read_varint_i32()?)),
        "GameTime_t" => Ok(FieldValue::Float(decode_f32_no_scale(data))),
        "MatchID_t" | "itemid_t" => Ok(FieldValue::Unsigned64(data.read_varint_u64()?)),
        _ => Ok(FieldValue::Unsigned32(data.read_varint_u32()?)),
    }
}

fn decode_f32<B: Buf>(data: &mut BitReader<B>, field: &Field) -> Result<f32> {
    if field.encoder == "coord" {
        return Ok(read_coord(data));
    }

    match field.var_name.as_ref() {
        "m_flSimulationTime" | "m_flAnimTime" => Ok((data.read_varint_u32()? as f32) / 30.0),
        "m_flRuneTime" => Ok(f32::from_bits(data.read_bits(4) as u32)),
        _ => {
            if (field.bit_count <= 0 || field.bit_count >= 32) {
                return Ok(decode_f32_no_scale(data));
            } else {
                return Ok(decode_f32_quantized(data, field));
            }
        }
    }
}

fn decode_vector2<B: Buf>(data: &mut BitReader<B>, field: &Field) -> Result<(f32, f32)> {
    Ok((decode_f32(data, field)?, decode_f32(data, field)?))
}

fn decode_vector3<B: Buf>(data: &mut BitReader<B>, field: &Field) -> Result<(f32, f32, f32)> {
    if field.encoder == "normal" {
        return Ok(read_normal_vector(data));
    }

    Ok((
        decode_f32(data, field)?,
        decode_f32(data, field)?,
        decode_f32(data, field)?,
    ))
}

fn decode_vector4<B: Buf>(data: &mut BitReader<B>, field: &Field) -> Result<(f32, f32, f32, f32)> {
    Ok((
        decode_f32(data, field)?,
        decode_f32(data, field)?,
        decode_f32(data, field)?,
        decode_f32(data, field)?,
    ))
}

fn decode_f32_no_scale<B: Buf>(data: &mut BitReader<B>) -> f32 {
    f32::from_bits(data.read_bits(32) as u32)
}

fn decode_f32_quantized<B: Buf>(data: &mut BitReader<B>, field: &Field) -> f32 {
    todo!();
}

fn decode_q_angle<B: Buf>(data: &mut BitReader<B>, field: &Field) -> (f32, f32, f32) {
    todo!();
}

fn decode_u64<B: Buf>(data: &mut BitReader<B>, field: &Field) -> Result<u64> {
    match field.encoder.as_ref() {
        "fixed64" => Ok(u64::from_le_bytes(
            data.read_bytes(8).as_slice().try_into()?,
        )),
        _ => data.read_varint_u64(),
    }
}

fn read_coord<B: Buf>(data: &mut BitReader<B>) -> f32 {
    let mut value = 0.0;

    let has_integer_part = data.read_boolean();
    let has_fractional_part = data.read_boolean();

    if has_integer_part || has_fractional_part {
        let is_negative = data.read_boolean();

        let integer_part = if has_integer_part {
            data.read_bits(14) + 1
        } else {
            0
        };

        let fractional_part = if has_fractional_part {
            data.read_bits(5)
        } else {
            0
        };

        value = integer_part as f32 + (fractional_part as f32 / (1 << 5) as f32);

        if is_negative {
            value = -value
        }
    }

    return value;
}

fn read_normal_vector<B: Buf>(data: &mut BitReader<B>) -> (f32, f32, f32) {
    let mut result = (0.0, 0.0, 0.0);

    let has_x = data.read_boolean();
    let has_y = data.read_boolean();

    if has_x {
        result.0 = read_normal(data);
    }
    if has_y {
        result.1 = read_normal(data);
    }

    let len_squared = result.0 * result.0 + result.1 * result.1;
    if len_squared < 1.0 {
        result.2 = (1.0 - len_squared as f64).sqrt() as f32
    } else {
        result.2 = 0.0;
    }

    let negative_z = data.read_boolean();
    if negative_z {
        result.2 = -result.2;
    }

    return result;
}

fn read_normal<B: Buf>(data: &mut BitReader<B>) -> f32 {
    let is_negative = data.read_boolean();
    let length = data.read_bits(11);
    let result = length as f32 / ((1 << 11) - 1) as f32;
    return if is_negative { -result } else { result };
}
