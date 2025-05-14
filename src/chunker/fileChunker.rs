use crate::chunker::chunker::Chunker;
use memmap2::Mmap;
use std::slice::Chunks;

pub struct FileChunker {}

impl FileChunker {
    pub fn new() -> Self {
        Self {}
    }
}

impl Chunker for FileChunker {
    fn chunk<'a>(&self, mmap: &'a Mmap) -> Chunks<'a, u8> {
        (&mmap[..]).chunks(mmap.len())
    }
}
