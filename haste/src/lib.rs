pub mod entities;
pub mod parser;
pub mod string_tables;
pub type Result<T> = core::result::Result<T, Box<dyn std::error::Error + 'static>>;

mod decoders;
mod huffman_tree;
mod readers;
mod utils;
