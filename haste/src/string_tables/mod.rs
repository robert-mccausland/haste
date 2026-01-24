mod reader;

use std::collections::HashMap;
use std::rc::Rc;

use haste_protobuf::dota::{CsvcMsgCreateStringTable, CsvcMsgUpdateStringTable};

use crate::Result;

use self::reader::StringTableReader;

#[derive(Debug)]
pub struct StringTables {
    string_tables: Vec<StringTable>,
    string_table_names: HashMap<String, usize>,
}

impl StringTables {
    pub fn new() -> Self {
        Self {
            string_tables: Vec::new(),
            string_table_names: HashMap::new(),
        }
    }

    pub fn get_by_name(&self, name: &str) -> Option<&StringTable> {
        let position = self.string_table_names.get(name);
        return position.map(|position| self.string_tables.get(*position).unwrap());
    }

    pub fn get(&self, id: usize) -> Option<&StringTable> {
        self.string_tables.get(id)
    }

    pub fn on_create_string_table(&mut self, message: &CsvcMsgCreateStringTable) -> Result<()> {
        let table_name = message
            .name
            .as_ref()
            .ok_or("string table did not have a name")?;
        if self.string_table_names.contains_key(table_name.as_str()) {
            return Err(format!("string table {:} already exists", table_name.as_str()).into());
        }

        let mut string_table = StringTable::new(
            table_name.to_owned(),
            message
                .user_data_fixed_size()
                .then_some(message.user_data_size_bits().try_into()?),
            message.using_varint_bitcounts(),
            message.flags().try_into()?,
        );

        let data = if message.data_compressed() {
            snap::raw::Decoder::new().decompress_vec(message.string_data())?
        } else {
            message.clone().string_data.unwrap_or_default()
        };

        string_table.update_entries(
            data.as_slice(),
            message.num_entries.unwrap_or_default().try_into()?,
        )?;

        self.string_table_names
            .insert(message.clone().name.unwrap(), self.string_tables.len());

        self.string_tables.push(string_table);

        Ok(())
    }

    pub fn on_update_string_table(&mut self, message: &CsvcMsgUpdateStringTable) -> Result<()> {
        if let Some(string_table) = self
            .string_tables
            .get_mut(TryInto::<usize>::try_into(message.table_id())?)
        {
            string_table.update_entries(
                message.string_data(),
                message.num_changed_entries().try_into()?,
            )?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct StringTable {
    name: String,
    data_size_bits: Option<usize>,
    has_varint_bit_counts: bool,
    flags: u32,
    entries: HashMap<u32, (Rc<str>, Vec<u8>)>,
    entries_by_name: HashMap<Rc<str>, u32>,
}

#[derive(Debug)]
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

    pub fn get_entry(&self, index: u32) -> Option<StringTableEntry<'_>> {
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

    pub fn get_entry_by_name(&self, name: &str) -> Option<StringTableEntry<'_>> {
        if let Some(id) = self.entries_by_name.get(name) {
            return self.get_entry(*id);
        } else {
            return None;
        }
    }

    pub fn get_entries(&self) -> Vec<StringTableEntry<'_>> {
        self.entries
            .iter()
            .map(|(index, value)| StringTableEntry {
                index: *index,
                name: value.0.as_ref(),
                data: value.1.as_ref(),
            })
            .collect()
    }

    pub fn update_entries(&mut self, data: &[u8], count: usize) -> Result<()> {
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
                    let name = Rc::<str>::from(name.as_str());
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
