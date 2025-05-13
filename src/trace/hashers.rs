use crate::util::arguments::HashType;
use digest::Digest;
use md5::Md5;
use sha1::Sha1;
use sha2::Sha256;

pub trait Hasher {
    // TODO: Benchmark performance loss by using heap memory
    fn hash(&self, chunk: &[u8]) -> Vec<u8>;
}

struct Sha1Hasher;
impl Hasher for Sha1Hasher {
    fn hash(&self, chunk: &[u8]) -> Vec<u8> {
        let mut h = Sha1::new();
        h.update(chunk);

        h.finalize().to_vec()
    }
}

struct Sha256Hasher;
impl Hasher for Sha256Hasher {
    fn hash(&self, chunk: &[u8]) -> Vec<u8> {
        let mut h = Sha256::new();
        h.update(chunk);

        h.finalize().to_vec()
    }
}

struct Md5Hasher;
impl Hasher for Md5Hasher {
    fn hash(&self, chunk: &[u8]) -> Vec<u8> {
        let mut h = Md5::new();
        h.update(chunk);

        h.finalize().to_vec()
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
