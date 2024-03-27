use crate::Result;

pub fn expect_one<I: Iterator>(mut iter: I) -> Result<I::Item> {
    let result = iter.next().ok_or("iterator had no elements")?;
    if iter.next().is_none() {
        Ok(result)
    } else {
        Err("iterator had multiple elements".into())
    }
}

pub fn transmute_i32_to_u32(value: i32) -> u32 {
    // Safety: its probably fine idk
    unsafe { core::mem::transmute::<i32, u32>(value) }
}
