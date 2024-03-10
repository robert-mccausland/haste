

use crate::{decoders::Bits, readers::bits::BitReader, Result};

pub struct QuantizedFloatDecoder {
    low: f32,
    high: f32,
    decimal_multiplier: f32,
    bit_count: u32,
    flags: u32,
}

impl QuantizedFloatDecoder {
    const ROUND_DOWN: u32 = (1 << 0);
    const ROUND_UP: u32 = (1 << 1);
    const ENCODE_ZERO: u32 = (1 << 2);
    const ENCODE_INTEGERS: u32 = (1 << 3);

    pub fn decode(&self, data: &mut BitReader) -> Result<f32> {
        if self.flags & Self::ROUND_DOWN != 0 && data.read_boolean() {
            return Ok(self.low);
        }

        if self.flags & Self::ROUND_UP != 0 && data.read_boolean() {
            return Ok(self.high);
        }

        if self.flags & Self::ENCODE_ZERO != 0 && data.read_boolean() {
            return Ok(0.0);
        }

        let result = self.low
            + (self.high - self.low)
                * data.read_bits(self.bit_count as usize) as f32
                * self.decimal_multiplier;
        return Ok(result);
    }

    pub fn new(
        bit_count: u32,
        flags: u32,
        low_value: Option<f32>,
        high_value: Option<f32>,
    ) -> Self {
        if bit_count == 0 || bit_count >= 32 {
            return Self {
                low: 0.0,
                high: 0.0,
                decimal_multiplier: 0.0,
                bit_count: 32,
                flags: 0,
            };
        }

        let mut flags = flags;
        let mut bit_count = bit_count;
        let mut low = low_value.unwrap_or(0.0);
        let mut high = high_value.unwrap_or(1.0);

        // Validate flags

        let mut steps = 1 << bit_count;

        let (_range, _offset) = if flags & Self::ROUND_DOWN != 0 {
            let range = high - low;
            let offset = range / steps as f32;
            high -= offset;
            (range, offset)
        } else if flags & Self::ROUND_UP != 0 {
            let range = high - low;
            let offset = range / steps as f32;
            low += offset;
            (range, offset)
        } else {
            (0.0, 0.0)
        };

        if flags & Self::ENCODE_INTEGERS != 0 {
            let delta = (high - low).max(1.0);
            let delta_log2 = (delta as f64).log2().ceil() as u32;
            let range2 = 1 << delta_log2;

            // TODO this code looks pretty shite but cba to fix atm
            let mut bc = bit_count;
            loop {
                if 1 << bc > range2 {
                    break;
                } else {
                    bc += 1;
                }
            }

            if bc < bit_count {
                bit_count = bc;
                steps = 1 << bit_count;
            }
        }

        // Assign multipliers
        let max: u32 = if bit_count == 32 {
            0xFFFFFFFE
        } else {
            (1 << bit_count) - 1
        };
        let actual_range = high - low;

        let mut high_low_multiplier = if actual_range == 0.0 {
            max as f32
        } else {
            max as f32 / actual_range
        };

        if high_low_multiplier * actual_range > max as f32
            || (high_low_multiplier * actual_range) as f64 > max as f64
        {
            for multiplier in [0.9999, 0.99, 0.9, 0.8, 0.7] {
                high_low_multiplier = (max as f32 / actual_range) * multiplier;

                if high_low_multiplier * actual_range > max as f32
                    || (high_low_multiplier * actual_range) as f64 > max as f64
                {
                    continue;
                }

                break;
            }
        }
        let decimal_multiplier = 1.0 / (steps - 1) as f32;

        let quantize = |value: f32| -> f32 {
            let steps = ((value - low) * high_low_multiplier) as u32;
            return low + (high - low) * (steps as f32) * decimal_multiplier;
        };

        // Remove unnecessary flags
        if (flags & Self::ROUND_DOWN) != 0 {
            if quantize(low) == low {
                flags &= !Self::ROUND_DOWN;
            }
        }

        if (flags & Self::ROUND_UP) != 0 {
            if quantize(high) == high {
                flags &= !Self::ROUND_UP;
            }
        }

        if (flags & Self::ENCODE_ZERO) != 0 {
            if quantize(0.0) == 0.0 {
                flags &= !Self::ENCODE_ZERO;
            }
        }

        return QuantizedFloatDecoder {
            bit_count,
            decimal_multiplier,
            flags,
            high,
            low,
        };
    }
}
