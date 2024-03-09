pub mod parser;
pub mod protos;
pub mod string_tables;
pub type Result<T> = core::result::Result<T, Box<dyn std::error::Error + 'static>>;

mod decoders;
mod readers;
