use crate::chunker::chunker::Chunker;
use memmap2::Mmap;

pub struct FileChunker {}

impl FileChunker {
    pub fn new() -> Self {
        Self {}
    }
}

impl Chunker for FileChunker {
    fn chunk<'a>(&self, mmap: &'a Mmap) -> Box<dyn Iterator<Item = &'a [u8]> + 'a> {
        let fileSize = mmap.len();
        Box::new(mmap.chunks(fileSize))
    }
}

#[cfg(test)]
mod tests {
    use super::super::chunker::*;
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
    fn testFileChunker() {
        let (data, mmap) = generateTestData();

        let chunker = FileChunker::new();
        let mut chunks = chunker.chunk(&mmap);

        assert_eq!(chunks.next(), Some(data.as_bytes()));
        assert!(chunks.next().is_none());
    }

    #[test]
    fn testFileChunkerFactory() {
        let (data, mmap) = generateTestData();

        let factory = ChunkFactory::new(ChunkerType::FILE);
        let chunker = factory.createChunker();

        let mut chunks = chunker.chunk(&mmap);

        assert_eq!(chunks.next(), Some(data.as_bytes()));
        assert!(chunks.next().is_none());
    }
}
