use crate::{Result, decoders::Bits, entities::Field, readers::bits::BitReader};

use super::quantized_float::QuantizedFloatDecoder;

pub fn decode_f32(data: &mut BitReader, field: &Field) -> Result<f32> {
    if field.encoder == "coord" {
        return Ok(read_coord(data));
    }

    match field.var_name.as_ref() {
        "m_flSimulationTime" | "m_flAnimTime" => Ok((data.read_varint_u32()? as f32) / 30.0),
        "m_flRuneTime" => Ok(f32::from_bits(data.read_bits(4) as u32)),
        _ => Ok(if field.bit_count <= 0 || field.bit_count >= 32 {
            decode_f32_no_scale(data)
        } else {
            decode_f32_quantized(data, field)?
        }),
    }
}

pub fn decode_vector2(data: &mut BitReader, field: &Field) -> Result<(f32, f32)> {
    Ok((decode_f32(data, field)?, decode_f32(data, field)?))
}

pub fn decode_vector3(data: &mut BitReader, field: &Field) -> Result<(f32, f32, f32)> {
    if field.encoder == "normal" {
        return Ok(read_normal_vector(data));
    }

    Ok((
        decode_f32(data, field)?,
        decode_f32(data, field)?,
        decode_f32(data, field)?,
    ))
}

pub fn decode_vector4(data: &mut BitReader, field: &Field) -> Result<(f32, f32, f32, f32)> {
    Ok((
        decode_f32(data, field)?,
        decode_f32(data, field)?,
        decode_f32(data, field)?,
        decode_f32(data, field)?,
    ))
}

pub fn decode_f32_no_scale(data: &mut BitReader) -> f32 {
    f32::from_bits(data.read_bits(32) as u32)
}

pub fn decode_f32_quantized(data: &mut BitReader, field: &Field) -> Result<f32> {
    let decoder = QuantizedFloatDecoder::new(
        field.bit_count,
        field.encode_flags,
        field.low_value,
        field.high_value,
    );

    return decoder.decode(data);
}

pub fn decode_q_angle(data: &mut BitReader, field: &Field) -> (f32, f32, f32) {
    let bit_count = field.bit_count;
    if field.encoder == "qangle_pitch_yaw" {
        (
            read_angle(data, bit_count),
            read_angle(data, bit_count),
            0.0,
        )
    } else if field.bit_count != 0 {
        (
            read_angle(data, bit_count),
            read_angle(data, bit_count),
            read_angle(data, bit_count),
        )
    } else {
        let has_x = data.read_boolean();
        let has_y = data.read_boolean();
        let has_z = data.read_boolean();
        (
            if has_x { read_coord(data) } else { 0.0 },
            if has_y { read_coord(data) } else { 0.0 },
            if has_z { read_coord(data) } else { 0.0 },
        )
    }
}

pub fn decode_u64(data: &mut BitReader, field: &Field) -> Result<u64> {
    match field.encoder.as_ref() {
        "fixed64" => Ok(u64::from_le_bytes(
            data.read_bytes(8).as_slice().try_into()?,
        )),
        _ => data.read_varint_u64(),
    }
}

fn read_coord(data: &mut BitReader) -> f32 {
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

fn read_angle(data: &mut BitReader, n: u32) -> f32 {
    (data.read_bits(n as usize) as f32 * 360.0) / (1 << n) as f32
}

fn read_normal_vector(data: &mut BitReader) -> (f32, f32, f32) {
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

fn read_normal(data: &mut BitReader) -> f32 {
    let is_negative = data.read_boolean();
    let length = data.read_bits(11);
    let result = length as f32 / ((1 << 11) - 1) as f32;
    return if is_negative { -result } else { result };
}
