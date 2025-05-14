use crate::chunker::chunker::Chunker;
use memmap2::Mmap;
use std::slice::Chunks;

pub struct StaticChunker {
    size: usize,
}

impl StaticChunker {
    pub fn new(chunkSize: usize) -> Self {
        Self { size: chunkSize }
    }
}

impl Chunker for StaticChunker {
    fn chunk<'a>(&self, mmap: &'a Mmap) -> Chunks<'a, u8> {
        mmap.chunks(self.size)
    }
}
