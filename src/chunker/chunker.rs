use crate::chunker::dynamicChunker::DynamicChunker;
use crate::chunker::fileChunker::FileChunker;
use crate::chunker::staticChunker::StaticChunker;
use std::slice::Chunks;

use clap::ValueEnum;
use memmap2::Mmap;

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum ChunkerType {
    FILE,
    SC1K,
    SC2K,
    SC4K,
    SC8K,
    SC16K,
    SC32K,
    SC64K,
    CDC1K,
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
            ChunkerType::FILE => 0,
            ChunkerType::SC1K => 1 << 10,
            ChunkerType::SC2K => 1 << 11,
            ChunkerType::SC4K => 1 << 12,
            ChunkerType::SC8K => 1 << 13,
            ChunkerType::SC16K => 1 << 14,
            ChunkerType::SC32K => 1 << 15,
            ChunkerType::SC64K => 1 << 16,
            ChunkerType::CDC1K => 1 << 10,
            ChunkerType::CDC2K => 1 << 11,
            ChunkerType::CDC4K => 1 << 12,
            ChunkerType::CDC8K => 1 << 13,
            ChunkerType::CDC16K => 1 << 14,
            ChunkerType::CDC32K => 1 << 15,
            ChunkerType::CDC64K => 1 << 16,
        }
    }
}

pub trait Chunker {
    fn chunk<'a>(&self, mmap: &'a Mmap) -> Chunks<'a, u8>;
}

pub struct ChunkFactory {
    t: ChunkerType,
    s: Option<Box<str>>,
}

impl ChunkFactory {
    pub fn new(chunkerType: ChunkerType, salt: Option<Box<str>>) -> Self {
        Self {
            t: chunkerType,
            s: salt,
        }
    }

    pub fn createChunker(&self) -> Box<dyn Chunker> {
        match self.t {
            ChunkerType::FILE => Box::new(FileChunker::new()),
            ChunkerType::SC1K
            | ChunkerType::SC2K
            | ChunkerType::SC4K
            | ChunkerType::SC8K
            | ChunkerType::SC16K
            | ChunkerType::SC32K
            | ChunkerType::SC64K => Box::new(StaticChunker::new(ChunkerType::getSize(&self.t))),
            ChunkerType::CDC1K
            | ChunkerType::CDC2K
            | ChunkerType::CDC4K
            | ChunkerType::CDC8K
            | ChunkerType::CDC16K
            | ChunkerType::CDC32K
            | ChunkerType::CDC64K => Box::new(DynamicChunker::new(ChunkerType::getSize(&self.t))),
        }
    }
}
