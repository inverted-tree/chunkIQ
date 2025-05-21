use crate::chunker::chunker::Chunker;
use memmap2::Mmap;

#[derive(Debug, Clone, Copy)]
struct Rabin<const N: usize> {
    polynomial: u64,
    appendTable: [u64; 256],
}

impl<const N: usize> Rabin<N> {
    pub fn new(polynomial: u64) -> Self {
        let table = Self::createByteLookupTable(polynomial);

        Self {
            polynomial: polynomial,
            appendTable: table,
        }
    }

    pub fn append(&self, fingerprint: u64, byte: u8) -> u64 {
        let shifted = (fingerprint >> 55) as usize;
        ((fingerprint << 8) | byte as u64) ^ self.appendTable[shifted]
    }

    fn createByteLookupTable(polynomial: u64) -> [u64; 256] {
        let mut table = [0u64; 256];
        let t1 = polyMod(0, 1 << 63, polynomial);

        for i in 0..256 {
            let v = polyModMult(i as u64, t1, polynomial);
            let w = (i as u64) << 63;
            table[i] = v | w;
        }

        table
    }
}

#[derive(Debug, Clone, Copy)]
struct RabinWindow<const N: usize> {
    rabin: Rabin<N>,
    removeTable: [u64; 256],
    buffer: [u8; N],
    pos: usize,
    fingerprint: u64,
}

impl<const N: usize> RabinWindow<N> {
    pub fn new(rabin: Rabin<N>) -> Self {
        let table = Self::createByteLookupTable(&rabin);

        Self {
            rabin: rabin,
            removeTable: table,
            buffer: [0; N],
            pos: 0,
            fingerprint: 0,
        }
    }

    pub fn reset(&mut self) {
        self.buffer = [0; N];
        self.pos = 0;
        self.fingerprint = 0;
    }

    pub fn append(&mut self, byte: u8) {
        let outByte = self.buffer[self.pos];
        self.buffer[self.pos] = byte;
        self.pos = (self.pos + 1) % N;

        self.fingerprint = self
            .rabin
            .append(self.fingerprint ^ self.removeTable[outByte as usize], byte);
    }

    pub fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    fn createByteLookupTable(rabin: &Rabin<N>) -> [u64; 256] {
        let mut shift = 1u64;
        for _ in 1..=N {
            shift = rabin.append(shift, 0);
        }

        let mut table = [0u64; 256];
        for i in 0..256 {
            table[i] = polyModMult(i as u64, shift, rabin.polynomial);
        }

        table
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RabinChunker<const N: usize> {
    minSize: usize,
    maxSize: usize,
    breakmark: u64,
    window: RabinWindow<N>,
}

impl<const N: usize> RabinChunker<N> {
    pub fn new(chunkSize: usize) -> Self {
        let polynomial = 13827942727904890243u64;
        let rabin = Rabin::new(polynomial);
        let window = RabinWindow::new(rabin);
        let breakmark = (1u64 << (64 - chunkSize.leading_zeros())) - 1;
        // let bits = (chunkSize.next_power_of_two()).trailing_zeros() - 1;
        // let breakmark = (1u64 << bits) - 1;

        Self {
            minSize: chunkSize / 4,
            maxSize: chunkSize * 4,
            breakmark: breakmark,
            window: window,
        }
    }
}

struct RabinChunkIter<'a, const N: usize> {
    minSize: usize,
    maxSize: usize,
    breakmark: u64,
    window: RabinWindow<N>,
    mmap: &'a Mmap,
    pos: usize,
    start: usize,
}

impl<'a, const N: usize> RabinChunkIter<'a, N> {
    fn new(chunker: RabinChunker<N>, mmap: &'a Mmap) -> Self {
        Self {
            minSize: chunker.minSize,
            maxSize: chunker.maxSize,
            breakmark: chunker.breakmark,
            window: chunker.window,
            mmap: mmap,
            pos: 0,
            start: 0,
        }
    }
}

impl<'a, const N: usize> Iterator for RabinChunkIter<'a, N> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.mmap.len() {
            let byte = self.mmap[self.pos];
            self.window.append(byte);
            self.pos += 1;

            let chunk_len = self.pos - self.start;
            if chunk_len >= self.minSize {
                let fp = self.window.fingerprint();
                if (fp & self.breakmark) == self.breakmark || chunk_len >= self.maxSize {
                    let chunk = &self.mmap[self.start..self.pos];
                    self.start = self.pos;
                    self.window.reset();
                    return Some(chunk);
                }
            }
        }

        if self.start < self.pos {
            let chunk = &self.mmap[self.start..self.pos];
            self.start = self.pos;
            self.window.reset();
            return Some(chunk);
        }

        None
    }
}

impl<const N: usize> Chunker for RabinChunker<N> {
    fn chunk<'a>(&self, mmap: &'a Mmap) -> Box<dyn Iterator<Item = &'a [u8]> + 'a> {
        Box::new(RabinChunkIter::new(self.clone(), mmap))
    }
}

fn polyMult(x: u64, y: u64) -> (u64, u64) {
    let mut hi: u64 = 0;
    let mut lo: u64 = if polyBitIsSet(x, 0) { y } else { 0 };

    for i in 1..64 {
        if polyBitIsSet(x, i) {
            lo ^= y << i;
            hi ^= y << (64 - i);
        }
    }

    (hi, lo)
}

fn polyMod(mut hi: u64, mut lo: u64, p: u64) -> u64 {
    if polyBitIsSet(hi, 63) {
        hi ^= p;
    }

    for i in (0..63).rev() {
        hi ^= p >> (63 - i);
        lo ^= p << (i + 1);
    }

    if polyBitIsSet(lo, 63) {
        lo ^= p;
    }

    lo
}

fn polyModMult(x: u64, y: u64, p: u64) -> u64 {
    let (hi, lo) = polyMult(x, y);
    polyMod(hi, lo, p)
}

fn polyBitIsSet(x: u64, b: u32) -> bool {
    (x & (1u64 << b)) != 0
}

#[cfg(test)]
mod test {
    use crate::chunker::chunker::ChunkerType;

    use super::super::chunker::ChunkFactory;
    use super::*;
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

        let chunker = RabinChunker::<64>::new(ChunkerType::CDC1K.getSize());
        let chunks: Vec<&[u8]> = chunker.chunk(&mmap).collect();

        assert!(!chunks.is_empty(), "No chunks were produced");
        let totalLength: usize = chunks.iter().map(|chunk| chunk.len()).sum();
        assert_eq!(
            totalLength,
            data.len(),
            "Chunked data doesn't match input length"
        );

        for chunk in &chunks {
            assert!(
                chunk.len() >= chunker.minSize || chunk.len() == data.len(),
                "Chunk is smaller than minSize"
            );
            assert!(
                chunk.len() <= chunker.maxSize,
                "Chunk is larger than maxSize"
            );
        }
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
