use crate::chunker::chunker::Chunker;
use fastcdc::FastCDC;

pub struct RabinChunker {
    min_size: usize,
    avg_size: usize,
    max_size: usize,
}

impl RabinChunker {
    pub fn new(avg_size: usize) -> Self {
        let min_size = avg_size / 4;
        let max_size = avg_size * 4;
        Self {
            min_size,
            avg_size,
            max_size,
        }
    }
}

impl Chunker for RabinChunker {
    fn chunk<'a>(&self, data: &'a [u8]) -> Box<dyn Iterator<Item = &'a [u8]> + 'a> {
        let chunker = FastCDC::new(data, self.min_size, self.avg_size, self.max_size);
        Box::new(chunker.map(|chunk| &data[chunk.offset..chunk.offset + chunk.length]))
    }
}

#[cfg(test)]
mod test {
    use crate::chunker::chunker::{Chunker, ChunkerType};

    use super::super::chunker::ChunkFactory;
    use super::*;
    use memmap2::Mmap;
    use std::io::Write;
    use tempfile::tempfile;

    fn generateTestData() -> (String, Mmap) {
        let mut file = tempfile().unwrap();
        let data = String::from("Lorem ipsum dolor sit amet consectetur adipiscing elit. Quisque faucibus ex sapien vitae pellentesque sem placerat. In id cursus mi pretium tellus duis convallis. Tempus leo eu aenean sed diam urna tempor. Pulvinar vivamus fringilla lacus nec metus bibendum egestas. Iaculis massa nisl malesuada lacinia integer nunc posuere. Ut hendrerit semper vel class aptent taciti sociosqu. Ad litora torquent per conubia nostra inceptos himenaeos.");
        file.write_all(data.as_bytes()).unwrap();
        let mmap = unsafe { Mmap::map(&file).unwrap() };

        (data, mmap)
    }

    #[test]
    fn testRabinChunker() {
        let (data, mmap) = generateTestData();

        let chunker = RabinChunker::new(ChunkerType::CDC1K.getSize());
        let chunks: Vec<&[u8]> = chunker.chunk(&mmap).collect();

        assert!(!chunks.is_empty(), "No chunks were produced");
        let totalLength: usize = chunks.iter().map(|chunk| chunk.len()).sum();
        assert_eq!(
            totalLength,
            data.len(),
            "Chunked data doesn't match input length"
        );
    }

    #[test]
    fn testRabinChunkerFactory() {
        let (data, mmap) = generateTestData();

        let factory = ChunkFactory::new(ChunkerType::CDC1K);
        let chunker = factory.createChunker();

        let chunks: Vec<&[u8]> = chunker.chunk(&mmap).collect();

        assert!(!chunks.is_empty(), "No chunks were produced");
        let totalLength: usize = chunks.iter().map(|chunk| chunk.len()).sum();
        assert_eq!(
            totalLength,
            data.len(),
            "Chunked data doesn't match input length"
        );
    }
}
