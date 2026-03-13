use crate::util::arguments::HashType;
use digest::Digest;
use md5::Md5;
use sha1::Sha1;
use sha2::Sha256;

pub trait Hasher: Send {
    fn hash(&self, chunk: &[u8]) -> [u8; 32];
}

struct Blake3Hasher;
impl Hasher for Blake3Hasher {
    fn hash(&self, chunk: &[u8]) -> [u8; 32] {
        *blake3::hash(chunk).as_bytes()
    }
}

struct Sha1Hasher;
impl Hasher for Sha1Hasher {
    fn hash(&self, chunk: &[u8]) -> [u8; 32] {
        let mut h = Sha1::new();
        h.update(chunk);
        let mut out = [0u8; 32];
        out[..20].copy_from_slice(&h.finalize());
        out
    }
}

struct Sha256Hasher;
impl Hasher for Sha256Hasher {
    fn hash(&self, chunk: &[u8]) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(chunk);
        let mut out = [0u8; 32];
        out.copy_from_slice(&h.finalize());
        out
    }
}

struct Md5Hasher;
impl Hasher for Md5Hasher {
    fn hash(&self, chunk: &[u8]) -> [u8; 32] {
        let mut h = Md5::new();
        h.update(chunk);
        let mut out = [0u8; 32];
        out[..16].copy_from_slice(&h.finalize());
        out
    }
}

pub struct HasherFactory {
    t: HashType,
}

impl HasherFactory {
    pub fn new(hashType: HashType) -> Self {
        Self { t: hashType }
    }

    pub fn createHasher(&self) -> Box<dyn Hasher> {
        match self.t {
            HashType::BLAKE3 => Box::new(Blake3Hasher),
            HashType::SHA1 => Box::new(Sha1Hasher),
            HashType::SHA256 => Box::new(Sha256Hasher),
            HashType::MD5 => Box::new(Md5Hasher),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use hex;

    const ALL_HASH_TYPES: &[HashType] = &[
        HashType::BLAKE3,
        HashType::SHA1,
        HashType::SHA256,
        HashType::MD5,
    ];

    fn createTestData() -> String {
        String::from("Lorem ipsum dolor sit amet consectetur adipiscing elit. Quisque faucibus ex sapien vitae pellentesque sem placerat. In id cursus mi pretium tellus duis convallis. Tempus leo eu aenean sed diam urna tempor. Pulvinar vivamus fringilla lacus nec metus bibendum egestas. Iaculis massa nisl malesuada lacinia integer nunc posuere. Ut hendrerit semper vel class aptent taciti sociosqu. Ad litora torquent per conubia nostra inceptos himenaeos.")
    }

    fn makeExpected(hex: &str, len: usize) -> [u8; 32] {
        let mut out = [0u8; 32];
        out[..len].copy_from_slice(&hex::decode(hex).unwrap());
        out
    }

    // ── Known-vector tests ────────────────────────────────────────

    #[test]
    fn testBlake3Hasher() {
        let data = createTestData();
        let hasher = HasherFactory::new(HashType::BLAKE3).createHasher();
        assert_eq!(
            hasher.hash(data.as_bytes()),
            makeExpected("28e77146a019c1264a6eb63097758a0eeacd2e867abb4e8b4925a4254cb19884", 32)
        );
    }

    #[test]
    fn testSha1Hasher() {
        let data = createTestData();
        let hasher = HasherFactory::new(HashType::SHA1).createHasher();
        assert_eq!(
            hasher.hash(data.as_bytes()),
            makeExpected("b964a452b73632aafaadfd2f219f06344a367ec1", 20)
        );
    }

    #[test]
    fn testSha256Hasher() {
        let data = createTestData();
        let hasher = HasherFactory::new(HashType::SHA256).createHasher();
        assert_eq!(
            hasher.hash(data.as_bytes()),
            makeExpected("ea948568682bc13198dfbd40b8b7b11f04d5b670cf5018f90237696dd6028a59", 32)
        );
    }

    #[test]
    fn testMd5Hasher() {
        let data = createTestData();
        let hasher = HasherFactory::new(HashType::MD5).createHasher();
        assert_eq!(
            hasher.hash(data.as_bytes()),
            makeExpected("d17fa6e4567f9baf13768881a2114bf7", 16)
        );
    }

    // ── Empty-input tests (well-known empty-input vectors) ────────

    #[test]
    fn testEmptyInput() {
        // SHA-1, SHA-256, MD5 vectors for empty input are well-established.
        // BLAKE3 vector verified against the reference implementation.
        let cases: &[(HashType, &str, usize)] = &[
            (HashType::BLAKE3,  "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262", 32),
            (HashType::SHA1,    "da39a3ee5e6b4b0d3255bfef95601890afd80709", 20),
            (HashType::SHA256,  "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", 32),
            (HashType::MD5,     "d41d8cd98f00b204e9800998ecf8427e", 16),
        ];
        for (hashType, expected, len) in cases {
            let hasher = HasherFactory::new(*hashType).createHasher();
            assert_eq!(
                hasher.hash(&[]),
                makeExpected(expected, *len),
                "empty-input vector mismatch for {hashType:?}"
            );
        }
    }

    // ── Zero-padding tests ────────────────────────────────────────

    #[test]
    fn testSha1ZeroPadding() {
        // SHA-1 digest is 20 bytes; bytes 20..32 of the output must be zero.
        let hasher = HasherFactory::new(HashType::SHA1).createHasher();
        let result = hasher.hash(createTestData().as_bytes());
        assert_eq!(&result[20..], &[0u8; 12], "SHA-1 padding bytes must be zero");
    }

    #[test]
    fn testMd5ZeroPadding() {
        // MD5 digest is 16 bytes; bytes 16..32 of the output must be zero.
        let hasher = HasherFactory::new(HashType::MD5).createHasher();
        let result = hasher.hash(createTestData().as_bytes());
        assert_eq!(&result[16..], &[0u8; 16], "MD5 padding bytes must be zero");
    }

    // ── Determinism tests ─────────────────────────────────────────

    #[test]
    fn testDeterminism() {
        // Hashing the same input twice with the same hasher instance must
        // return identical results (guards against accidental shared state).
        let data = createTestData();
        for &hashType in ALL_HASH_TYPES {
            let hasher = HasherFactory::new(hashType).createHasher();
            assert_eq!(
                hasher.hash(data.as_bytes()),
                hasher.hash(data.as_bytes()),
                "non-deterministic output for {hashType:?}"
            );
        }
    }

    // ── Collision sanity tests ────────────────────────────────────

    #[test]
    fn testDistinctInputsProduceDistinctHashes() {
        let a = b"input_a".as_slice();
        let b = b"input_b".as_slice();
        for &hashType in ALL_HASH_TYPES {
            let hasher = HasherFactory::new(hashType).createHasher();
            assert_ne!(
                hasher.hash(a),
                hasher.hash(b),
                "collision on trivially distinct inputs for {hashType:?}"
            );
        }
    }
}
