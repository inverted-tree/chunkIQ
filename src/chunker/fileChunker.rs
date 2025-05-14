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

#[cfg(test)]
mod tests {
    use super::super::chunker::*;
    use super::*;

    use memmap2::Mmap;
    use std::io::Write;
    use tempfile::tempfile;

    #[test]
    fn testFileChunker() {
        let mut file = tempfile().unwrap();
        let data = b"Lorem ipsum dolor sit amet.";
        file.write_all(data).unwrap();
        let mmap = unsafe { Mmap::map(&file).unwrap() };

        let chunker = FileChunker::new();
        let mut chunks = chunker.chunk(&mmap);

        assert_eq!(chunks.next(), Some(data.as_slice()));
        assert!(chunks.next().is_none());
    }

    #[test]
    fn testFileChunkerFactory() {
        let mut file = tempfile().unwrap();
        let data = b"hello world";
        file.write_all(data).unwrap();
        let mmap = unsafe { Mmap::map(&file).unwrap() };

        let factory = ChunkFactory::new(ChunkerType::FILE, None);
        let chunker = factory.createChunker();
        let mut chunks = chunker.chunk(&mmap);

        assert_eq!(chunks.next(), Some(data.as_slice()));
        assert!(chunks.next().is_none());
    }
}
