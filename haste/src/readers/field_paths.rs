use once_cell::sync::Lazy;
use std::ops::Deref;

use crate::{
    decoders::Bits,
    entities::FieldPath,
    huffman_tree::{self, Node, Weighted},
    readers::bits::BitReader,
};

#[derive(Debug)]
struct FieldPathOperation {
    name: &'static str,
    weight: u32,
    operation: fn(&mut BitReader, &mut FieldPath),
}

impl<'a> Clone for FieldPathOperation {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            weight: self.weight.clone(),
            operation: self.operation.clone(),
        }
    }
}

impl<'a> Weighted for FieldPathOperation {
    fn weight(&self) -> u32 {
        if self.weight == 0 {
            1
        } else {
            self.weight
        }
    }
}

pub fn read_field_paths(data: &mut BitReader) -> Vec<FieldPath> {
    let mut node = HUFFMAN_TREE.deref();
    let mut result = Vec::new();
    let mut current = FieldPath {
        done: false,
        last: 0,
        path: [-1, 0, 0, 0, 0, 0, 0],
    };

    loop {
        match &node.content {
            huffman_tree::NodeContent::Leaf(value) => {
                (value.operation)(data, &mut current);
                if current.done {
                    return result;
                } else {
                    result.push(current.clone());
                    node = HUFFMAN_TREE.deref();
                };
            }
            huffman_tree::NodeContent::Parent(left, right) => {
                node = if data.read_boolean() { right } else { left }
            }
        }
    }
}

static HUFFMAN_TREE: Lazy<Node<FieldPathOperation>> = Lazy::new(|| {
    huffman_tree::build_tree(vec![
        FieldPathOperation {
            name: "PlusOne",
            weight: 36271,
            operation: |_data, field_path| field_path.path[field_path.last as usize] += 1,
        },
        FieldPathOperation {
            name: "PlusTwo",
            weight: 10334,
            operation: |_data, field_path| {
                field_path.path[field_path.last as usize] += 2;
            },
        },
        FieldPathOperation {
            name: "PlusThree",
            weight: 1375,
            operation: |_data, field_path| {
                field_path.path[field_path.last as usize] += 3;
            },
        },
        FieldPathOperation {
            name: "PlusFour",
            weight: 646,
            operation: |_data, field_path| {
                field_path.path[field_path.last as usize] += 4;
            },
        },
        FieldPathOperation {
            name: "PlusN",
            weight: 4128,
            operation: |data, field_path| {
                field_path.path[field_path.last as usize] +=
                    data.read_varbit_field_path().unwrap() + 5;
            },
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaZeroRightZero",
            weight: 35,
            operation: |_data, field_path| {
                field_path.last += 1;
                field_path.path[field_path.last as usize] = 0;
            },
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaZeroRightNonZero",
            weight: 3,
            operation: |data, field_path| {
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_varbit_field_path().unwrap();
            },
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaOneRightZero",
            weight: 521,
            operation: |_dataa, field_path| {
                field_path.path[field_path.last as usize] += 1;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = 0;
            },
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaOneRightNonZero",
            weight: 2942,
            operation: |data, field_path| {
                field_path.path[field_path.last as usize] += 1;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_varbit_field_path().unwrap();
            },
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaNRightZero",
            weight: 560,
            operation: |data, field_path| {
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last as usize] = 0;
            },
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaNRightNonZero",
            weight: 471,
            operation: |data, field_path| {
                field_path.path[field_path.last as usize] +=
                    data.read_varbit_field_path().unwrap() + 2;
                field_path.last += 1;
                field_path.path[field_path.last as usize] =
                    data.read_varbit_field_path().unwrap() + 1;
            },
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaNRightNonZeroPack6Bits",
            weight: 10530,
            operation: |data, field_path| {
                field_path.path[field_path.last as usize] += data.read_bits(3) as i32 + 2;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(3) as i32 + 1;
            },
        },
        FieldPathOperation {
            name: "PushOneLeftDeltaNRightNonZeroPack8Bits",
            weight: 251,
            operation: |data, field_path| {
                field_path.path[field_path.last as usize] += data.read_bits(4) as i32 + 2;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(4) as i32 + 1;
            },
        },
        FieldPathOperation {
            name: "PushTwoLeftDeltaZero",
            weight: 0,
            operation: |data, field_path| {
                field_path.last += 1;
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
            },
        },
        FieldPathOperation {
            name: "PushTwoPack5LeftDeltaZero",
            weight: 0,
            operation: |data, field_path| {
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(5) as i32;
            },
        },
        FieldPathOperation {
            name: "PushThreeLeftDeltaZero",
            weight: 0,
            operation: |data, field_path| {
                field_path.last += 1;
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
            },
        },
        FieldPathOperation {
            name: "PushThreePack5LeftDeltaZero",
            weight: 0,
            operation: |data, field_path| {
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(5) as i32;
            },
        },
        FieldPathOperation {
            name: "PushTwoLeftDeltaOne",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last as usize] += 1;
                field_path.last += 1;
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
            },
        },
        FieldPathOperation {
            name: "PushTwoPack5LeftDeltaOne",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last as usize] += 1;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(5) as i32;
            },
        },
        FieldPathOperation {
            name: "PushThreeLeftDeltaOne",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last as usize] += 1;
                field_path.last += 1;
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
            },
        },
        FieldPathOperation {
            name: "PushThreePack5LeftDeltaOne",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last as usize] += 1;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(5) as i32;
            },
        },
        FieldPathOperation {
            name: "PushTwoLeftDeltaN",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last as usize] += data.read_varbit() as i32 + 2;
                field_path.last += 1;
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
            },
        },
        FieldPathOperation {
            name: "PushTwoPack5LeftDeltaN",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last as usize] += data.read_varbit() as i32 + 2;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(5) as i32;
            },
        },
        FieldPathOperation {
            name: "PushThreeLeftDeltaN",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last as usize] += data.read_varbit() as i32 + 2;
                field_path.last += 1;
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
                field_path.last += 1;
                field_path.path[field_path.last as usize] += data.read_varbit_field_path().unwrap();
            },
        },
        FieldPathOperation {
            name: "PushThreePack5LeftDeltaN",
            weight: 0,
            operation: |data, field_path| {
                field_path.path[field_path.last as usize] += data.read_varbit() as i32 + 2;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(5) as i32;
                field_path.last += 1;
                field_path.path[field_path.last as usize] = data.read_bits(5) as i32;
            },
        },
        FieldPathOperation {
            name: "PushN",
            weight: 0,
            operation: |data, field_path| {
                let n = data.read_varbit() as usize;
                field_path.path[field_path.last as usize] += data.read_varbit() as i32;
                for _ in 0..n {
                    field_path.last += 1;
                    field_path.path[field_path.last as usize] +=
                        data.read_varbit_field_path().unwrap();
                }
            },
        },
        FieldPathOperation {
            name: "PushNAndNonTopological",
            weight: 310,
            operation: |data, field_path| {
                for i in 0..=field_path.last {
                    if data.read_boolean() {
                        field_path.path[i as usize] += data.read_varint_i32().unwrap() as i32 + 1;
                    }
                }
                let count = data.read_varbit() as usize;
                for _ in 0..count {
                    field_path.last += 1;
                    field_path.path[field_path.last as usize] =
                        data.read_varbit_field_path().unwrap();
                }
            },
        },
        FieldPathOperation {
            name: "PopOnePlusOne",
            weight: 2,
            operation: |_data, field_path| {
                field_path.pop(1);
                field_path.path[field_path.last as usize] += 1;
            },
        },
        FieldPathOperation {
            name: "PopOnePlusN",
            weight: 0,
            operation: |data, field_path| {
                field_path.pop(1);
                field_path.path[field_path.last as usize] +=
                    data.read_varbit_field_path().unwrap() + 1;
            },
        },
        FieldPathOperation {
            name: "PopAllButOnePlusOne",
            weight: 1837,
            operation: |_data, field_path| {
                field_path.pop(field_path.last);
                field_path.path[0] += 1;
            },
        },
        FieldPathOperation {
            name: "PopAllButOnePlusN",
            weight: 149,
            operation: |data, field_path| {
                field_path.pop(field_path.last);
                field_path.path[0] += data.read_varbit_field_path().unwrap() + 1;
            },
        },
        FieldPathOperation {
            name: "PopAllButOnePlusNPack3Bits",
            weight: 300,
            operation: |data, field_path| {
                field_path.pop(field_path.last);
                field_path.path[0] += data.read_bits(3) as i32 + 1;
            },
        },
        FieldPathOperation {
            name: "PopAllButOnePlusNPack6Bits",
            weight: 634,
            operation: |data, field_path| {
                field_path.pop(field_path.last);
                field_path.path[0] += data.read_bits(6) as i32 + 1;
            },
        },
        FieldPathOperation {
            name: "PopNPlusOne",
            weight: 0,
            operation: |data, field_path| {
                field_path.pop(data.read_varbit_field_path().unwrap());
                field_path.path[field_path.last as usize] += 1;
            },
        },
        FieldPathOperation {
            name: "PopNPlusN",
            weight: 0,
            operation: |data, field_path| {
                field_path.pop(data.read_varbit_field_path().unwrap());
                field_path.path[field_path.last as usize] += data.read_varint_i32().unwrap() as i32;
            },
        },
        FieldPathOperation {
            name: "PopNAndNonTopographical",
            weight: 1,
            operation: |data, field_path| {
                field_path.pop(data.read_varbit_field_path().unwrap());
                for i in 0..=field_path.last {
                    if data.read_boolean() {
                        field_path.path[i as usize] += data.read_varint_i32().unwrap();
                    }
                }
            },
        },
        FieldPathOperation {
            name: "NonTopoComplex",
            weight: 76,
            operation: |data, field_path| {
                for i in 0..=field_path.last {
                    if data.read_boolean() {
                        field_path.path[i as usize] += data.read_varint_i32().unwrap();
                    }
                }
            },
        },
        FieldPathOperation {
            name: "NonTopoPenultimatePlusOne",
            weight: 271,
            operation: |_data, field_path| {
                field_path.path[field_path.last as usize - 1] += 1;
            },
        },
        FieldPathOperation {
            name: "NonTopoComplexPack4Bits",
            weight: 99,
            operation: |data, field_path| {
                for i in 0..=field_path.last {
                    if data.read_boolean() {
                        field_path.path[i as usize] += data.read_bits(4) as i32 - 7;
                    }
                }
            },
        },
        FieldPathOperation {
            name: "FieldPathEnweightFinish",
            weight: 25474,
            operation: |_data, field_path| {
                field_path.done = true;
            },
        },
    ])
});
