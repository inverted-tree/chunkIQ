use crate::chunker::chunker::Chunker;
use memmap2::Mmap;
use std::slice::Chunks;

pub struct DynamicChunker {
    size: usize,
}

impl DynamicChunker {
    pub fn new(chunkSize: usize) -> Self {
        Self { size: chunkSize }
    }
}

impl Chunker for DynamicChunker {
    // TODO: Implement a rabin chunker for content-defined chunking of the input files.
    fn chunk<'a>(&self, _mmap: &'a Mmap) -> Chunks<'a, u8> {
        todo!("Implement the rabin chunker.")
    }
}
