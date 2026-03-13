use crate::util::arguments::HashType;
use digest::Digest;
use md5::Md5;
use sha1::Sha1;
use sha2::Sha256;

pub trait Hasher: Send {
    fn hash(&self, chunk: &[u8]) -> [u8; 32];
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

    fn createTestData() -> String {
        String::from("Lorem ipsum dolor sit amet consectetur adipiscing elit. Quisque faucibus ex sapien vitae pellentesque sem placerat. In id cursus mi pretium tellus duis convallis. Tempus leo eu aenean sed diam urna tempor. Pulvinar vivamus fringilla lacus nec metus bibendum egestas. Iaculis massa nisl malesuada lacinia integer nunc posuere. Ut hendrerit semper vel class aptent taciti sociosqu. Ad litora torquent per conubia nostra inceptos himenaeos.")
    }

    fn makeExpected(hex: &str, len: usize) -> [u8; 32] {
        let mut out = [0u8; 32];
        out[..len].copy_from_slice(&hex::decode(hex).unwrap());
        out
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
}
