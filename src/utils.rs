use crate::Result;

pub fn expect_one<I: Iterator>(mut iter: I) -> Result<I::Item> {
    let result = iter.next().ok_or("iterator had no elements")?;
    if iter.next().is_none() {
        Err("iterator had multiple elements".into())
    } else {
        Ok(result)
    }
}
