use crate::chunker::chunker::Chunker;
use memmap2::Mmap;

pub struct StaticChunker {
    size: usize,
}

impl StaticChunker {
    pub fn new(chunkSize: usize) -> Self {
        Self { size: chunkSize }
    }
}

impl Chunker for StaticChunker {
    fn chunk<'a>(&self, mmap: &'a Mmap) -> Box<dyn Iterator<Item = &'a [u8]> + 'a> {
        let chunkSize = self.size;
        Box::new(mmap.chunks(chunkSize))
    }
}

#[cfg(test)]
mod tests {
    use super::super::chunker::*;
    use super::*;

    use memmap2::Mmap;
    use std::io::Write;
    use tempfile::tempfile;

    fn createTestTypes() -> Vec<ChunkerType> {
        vec![
            ChunkerType::SC1K,
            ChunkerType::SC2K,
            ChunkerType::SC4K,
            ChunkerType::SC8K,
            ChunkerType::SC16K,
            ChunkerType::SC32K,
            ChunkerType::SC64K,
        ]
    }

    fn createTestData(N: usize) -> Mmap {
        let mut file = tempfile().unwrap();
        file.write_all(&vec![0u8; N * 1024]).unwrap();
        let mmap = unsafe { Mmap::map(&file).unwrap() };

        mmap
    }

    #[test]
    fn testStaticChunker() {
        let types = createTestTypes();
        for c in types {
            let N = 128;
            let mmap = createTestData(N);

            let chunker = StaticChunker::new(ChunkerType::getSize(&c));
            let mut chunks = chunker.chunk(&mmap);

            // assert_eq!(chunks.len(), N * 1024 / ChunkerType::getSize(&c));
            let expected = vec![0u8; ChunkerType::getSize(&c)];
            for chunk in chunks.by_ref() {
                assert_eq!(chunk, expected.as_slice());
            }
            assert!(chunks.next().is_none());
        }
    }

    #[test]
    fn testStaticChunkerFactory() {
        let types = createTestTypes();

        for c in types {
            let N = 128;
            let mmap = createTestData(N);

            let factory = ChunkFactory::new(c);
            let chunker = factory.createChunker();
            let mut chunks = chunker.chunk(&mmap);

            // assert_eq!(chunks.len(), N * 1024 / ChunkerType::getSize(&c));
            let expected = vec![0u8; ChunkerType::getSize(&c)];
            for chunk in chunks.by_ref() {
                assert_eq!(chunk, expected.as_slice());
            }
            assert!(chunks.next().is_none());
        }
    }
}
