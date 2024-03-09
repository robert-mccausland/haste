use std::collections::HashMap;
use std::rc::Rc;

use crate::{readers::string_table::StringTableReader, Result};
use bytes::Buf;

pub struct StringTable {
    name: String,
    data_size_bits: Option<usize>,
    has_varint_bit_counts: bool,
    flags: u32,
    entries: HashMap<u32, (Rc<String>, Vec<u8>)>,
    entries_by_name: HashMap<Rc<String>, u32>,
}

pub struct StringTableEntry<'a> {
    pub index: u32,
    pub name: &'a str,
    pub data: &'a Vec<u8>,
}

impl StringTable {
    pub(crate) fn new(
        name: String,
        data_size_bits: Option<usize>,
        has_varint_bit_counts: bool,
        flags: u32,
    ) -> Self {
        Self {
            name,
            data_size_bits,
            has_varint_bit_counts,
            flags,
            entries: HashMap::new(),
            entries_by_name: HashMap::new(),
        }
    }

    pub fn get_name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn get_entry(&self, index: u32) -> Option<StringTableEntry> {
        if let Some(entry) = self.entries.get(&index) {
            return Some(StringTableEntry {
                index,
                name: entry.0.as_ref(),
                data: entry.1.as_ref(),
            });
        } else {
            return None;
        }
    }

    pub(crate) fn update_entries<B: Buf>(&mut self, data: B, count: usize) -> Result<()> {
        let reader = StringTableReader::new(
            data,
            count,
            self.data_size_bits,
            self.has_varint_bit_counts,
            self.flags,
        );

        for entry in reader {
            let entry = entry?;

            if let Some(existing_entry) = self.entries.get(&entry.index) {
                if let Some(ref name) = entry.name {
                    if *existing_entry.0 != *name {
                        return Err(format!(
                            "name did not match existing entry, expected {}, got {}",
                            existing_entry.0, name
                        )
                        .into());
                    }
                }
            } else {
                if let Some(name) = entry.name {
                    let name = Rc::new(name);
                    self.entries_by_name.insert(name.clone(), entry.index);
                    self.entries
                        .insert(entry.index, (name, entry.data.unwrap_or_default()));
                } else {
                    return Err(format!("new entries must have their name set").into());
                }
            }
        }

        Ok(())
    }
}
