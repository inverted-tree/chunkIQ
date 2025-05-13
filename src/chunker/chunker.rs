// use std::path::PathBuf;
//
// use crate::chunker::chunk::Chunk;
use clap::ValueEnum;

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum ChunkerType {
    SC2K,
    SC4K,
    SC8K,
    SC16K,
    SC32K,
    SC64K,
    CDC2K,
    CDC4K,
    CDC8K,
    CDC16K,
    CDC32K,
    CDC64K,
}

impl ChunkerType {
    pub fn getSize(&self) -> usize {
        match self {
            ChunkerType::SC2K => 1 << 11,
            ChunkerType::SC4K => 1 << 12,
            ChunkerType::SC8K => 1 << 13,
            ChunkerType::SC16K => 1 << 14,
            ChunkerType::SC32K => 1 << 15,
            ChunkerType::SC64K => 1 << 16,
            ChunkerType::CDC2K => 1 << 11,
            ChunkerType::CDC4K => 1 << 12,
            ChunkerType::CDC8K => 1 << 13,
            ChunkerType::CDC16K => 1 << 14,
            ChunkerType::CDC32K => 1 << 15,
            ChunkerType::CDC64K => 1 << 16,
        }
    }
}

// pub trait Chunker {
//     fn accept<H>(buf: Box<PathBuf>, h: H)
//     where
//         H: Fn(Chunk);
//     fn chunk();
//     fn close();
// }
