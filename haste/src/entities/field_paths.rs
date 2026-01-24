use once_cell::sync::Lazy;

use crate::{decoders::Bits, readers::bits::BitReader};

use crate::huffman_tree::{self, Node, Weighted};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FieldPath {
    pub path: [i32; 7],
    pub last: usize,
}

impl FieldPath {
    pub fn pop(&mut self, n: usize) {
        for _ in 0..n {
            self.path[self.last] = 0;
            self.last -= 1;
        }
    }
}

#[derive(Debug)]
pub struct FieldPathOperation {
    pub name: &'static str,
    pub weight: u32,
    pub operation: fn(&mut BitReader, &mut FieldPath),
    pub is_final: bool,
}

impl<'a> Clone for FieldPathOperation {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            weight: self.weight.clone(),
            operation: self.operation.clone(),
            is_final: self.is_final.clone(),
        }
    }
}

impl<'a> Weighted for FieldPathOperation {
    fn weight(&self) -> u32 {
        if self.weight == 0 { 1 } else { self.weight }
    }
}

pub static FIELD_PATH_TREE: Lazy<Node<FieldPathOperation>> = Lazy::new(|| {
    huffman_tree::build_tree(vec![
        FieldPathOperation {
            name: "PlusOne",
            weight: 36271,
            operation: |_data, field_path| field_path.path[field_path.last] += 1,
            is_final: false,
        },
        FieldPathOperation {
            name: "PlusTwo",
            weight: 10334,
            operation: |_data, field_path| {
                field_path.path[field_path.last] += 2;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PlusThree",
            weight: 1375,
            operation: |_data, field_path| {
                field_path.path[field_path.last] += 3;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PlusFour",
            weight: 646,
            operation: |_data, field_path| {
                field_path.path[field_path.last] += 4;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PlusN",
            weight: 4128,
            operation: |data, field_path| {
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap() + 5;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaZeroRightZero",
            weight: 35,
            operation: |_data, field_path| {
                field_path.last += 1;
                field_path.path[field_path.last] = 0;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaZeroRightNonZero",
            weight: 3,
            operation: |data, field_path| {
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_varbit_field_path().unwrap();
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaOneRightZero",
            weight: 521,
            operation: |_dataa, field_path| {
                field_path.path[field_path.last] += 1;
                field_path.last += 1;
                field_path.path[field_path.last] = 0;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaOneRightNonZero",
            weight: 2942,
            operation: |data, field_path| {
                field_path.path[field_path.last] += 1;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_varbit_field_path().unwrap();
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaNRightZero",
            weight: 560,
            operation: |data, field_path| {
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last] = 0;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaNRightNonZero",
            weight: 471,
            operation: |data, field_path| {
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap() + 2;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_varbit_field_path().unwrap() + 1;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaNRightNonZeroPack6Bits",
            weight: 10530,
            operation: |data, field_path| {
                field_path.path[field_path.last] += data.read_bits(3) as i32 + 2;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(3) as i32 + 1;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaNRightNonZeroPack8Bits",
            weight: 251,
            operation: |data, field_path| {
                field_path.path[field_path.last] += data.read_bits(4) as i32 + 2;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(4) as i32 + 1;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushTwoLeftDeltaZero",
            weight: 0,
            operation: |data, field_path| {
                field_path.last += 1;
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushTwoPack5LeftDeltaZero",
            weight: 0,
            operation: |data, field_path| {
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(5) as i32;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushThreeLeftDeltaZero",
            weight: 0,
            operation: |data, field_path| {
                field_path.last += 1;
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushThreePack5LeftDeltaZero",
            weight: 0,
            operation: |data, field_path| {
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(5) as i32;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushTwoLeftDeltaOne",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last] += 1;
                field_path.last += 1;
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushTwoPack5LeftDeltaOne",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last] += 1;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(5) as i32;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushThreeLeftDeltaOne",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last] += 1;
                field_path.last += 1;
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushThreePack5LeftDeltaOne",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last] += 1;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(5) as i32;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushTwoLeftDeltaN",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last] += data.read_varbit() as i32 + 2;
                field_path.last += 1;
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushTwoPack5LeftDeltaN",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last] += data.read_varbit() as i32 + 2;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(5) as i32;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushThreeLeftDeltaN",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last] += data.read_varbit() as i32 + 2;
                field_path.last += 1;
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushThreePack5LeftDeltaN",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last] += data.read_varbit() as i32 + 2;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last] = data.read_bits(5) as i32;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushN",
            weight: 0,
            operation: |data, field_path| {
                let n = data.read_varbit();
                field_path.path[field_path.last] += data.read_varbit() as i32;
                for _ in 0..n {
                    field_path.last += 1;
                    field_path.path[field_path.last] += data.read_varbit_field_path().unwrap();
                }
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PushNAndNonTopological",
            weight: 310,
            operation: |data, field_path| {
                for i in 0..=field_path.last {
                    if data.read_boolean() {
                        field_path.path[i] += data.read_varint_i32().unwrap() as i32 + 1;
                    }
                }
                let count = data.read_varbit();
                for _ in 0..count {
                    field_path.last += 1;
                    field_path.path[field_path.last] = data.read_varbit_field_path().unwrap();
                }
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PopOnePlusOne",
            weight: 2,
            operation: |_data, field_path| {
                field_path.pop(1);
                field_path.path[field_path.last] += 1;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PopOnePlusN",
            weight: 0,
            operation: |data, field_path| {
                field_path.pop(1);
                field_path.path[field_path.last] += data.read_varbit_field_path().unwrap() + 1;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PopAllButOnePlusOne",
            weight: 1837,
            operation: |_data, field_path| {
                field_path.pop(field_path.last);
                field_path.path[0] += 1;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PopAllButOnePlusN",
            weight: 149,
            operation: |data, field_path| {
                field_path.pop(field_path.last);
                field_path.path[0] += data.read_varbit_field_path().unwrap() + 1;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PopAllButOnePlusNPack3Bits",
            weight: 300,
            operation: |data, field_path| {
                field_path.pop(field_path.last);
                field_path.path[0] += data.read_bits(3) as i32 + 1;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PopAllButOnePlusNPack6Bits",
            weight: 634,
            operation: |data, field_path| {
                field_path.pop(field_path.last);
                field_path.path[0] += data.read_bits(6) as i32 + 1;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PopNPlusOne",
            weight: 0,
            operation: |data, field_path| {
                field_path.pop(data.read_varbit_field_path().unwrap() as usize);
                field_path.path[field_path.last] += 1;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PopNPlusN",
            weight: 0,
            operation: |data, field_path| {
                field_path.pop(data.read_varbit_field_path().unwrap() as usize);
                field_path.path[field_path.last] += data.read_varint_i32().unwrap();
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "PopNAndNonTopographical",
            weight: 1,
            operation: |data, field_path| {
                field_path.pop(data.read_varbit_field_path().unwrap() as usize);
                for i in 0..=field_path.last {
                    if data.read_boolean() {
                        field_path.path[i] += data.read_varint_i32().unwrap();
                    }
                }
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "NonTopoComplex",
            weight: 76,
            operation: |data, field_path| {
                for i in 0..=field_path.last {
                    if data.read_boolean() {
                        field_path.path[i] += data.read_varint_i32().unwrap();
                    }
                }
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "NonTopoPenultimatePlusOne",
            weight: 271,
            operation: |_data, field_path| {
                field_path.path[field_path.last - 1] += 1;
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "NonTopoComplexPack4Bits",
            weight: 99,
            operation: |data, field_path| {
                for i in 0..=field_path.last {
                    if data.read_boolean() {
                        field_path.path[i] += data.read_bits(4) as i32 - 7;
                    }
                }
            },
            is_final: false,
        },
        FieldPathOperation {
            name: "FieldPathEnweightFinish",
            weight: 25474,
            operation: |_data, _field_path| {},
            is_final: true,
        },
    ])
});
