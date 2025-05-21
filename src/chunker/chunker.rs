use crate::chunker::fileChunker::FileChunker;
use crate::chunker::rabinChunker::RabinChunker;
use crate::chunker::staticChunker::StaticChunker;

use clap::ValueEnum;
use memmap2::Mmap;

#[derive(Debug, Clone, Copy)]
pub enum ChunkingScheme {
    FILE,
    STATIC,
    CONTENT,
}

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

    pub fn getScheme(&self) -> ChunkingScheme {
        match self {
            ChunkerType::FILE => ChunkingScheme::FILE,
            ChunkerType::SC1K
            | ChunkerType::SC2K
            | ChunkerType::SC4K
            | ChunkerType::SC8K
            | ChunkerType::SC16K
            | ChunkerType::SC32K
            | ChunkerType::SC64K => ChunkingScheme::STATIC,
            ChunkerType::CDC1K
            | ChunkerType::CDC2K
            | ChunkerType::CDC4K
            | ChunkerType::CDC8K
            | ChunkerType::CDC16K
            | ChunkerType::CDC32K
            | ChunkerType::CDC64K => ChunkingScheme::CONTENT,
        }
    }
}

pub trait Chunker {
    fn chunk<'a>(&self, mmap: &'a Mmap) -> Box<dyn Iterator<Item = &'a [u8]> + 'a>;
}

pub struct ChunkFactory {
    t: ChunkerType,
}

impl ChunkFactory {
    pub fn new(chunkerType: ChunkerType) -> Self {
        Self { t: chunkerType }
    }

    pub fn createChunker(&self) -> Box<dyn Chunker> {
        match self.t.getScheme() {
            ChunkingScheme::FILE => Box::new(FileChunker::new()),
            ChunkingScheme::STATIC => Box::new(StaticChunker::new(self.t.getSize())),
            ChunkingScheme::CONTENT => Box::new(RabinChunker::<64>::new(self.t.getSize())),
        }
    }
}
