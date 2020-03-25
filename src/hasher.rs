use crate::utils::*;
use crate::*;
use digest::Digest;

#[derive(Clone, Debug)]
pub struct Blake2s;
impl Hasher for Blake2s {
    fn new() -> Self {
        Blake2s
    }

    fn digest(&self, bytes: &[u8]) -> Hash {
        let mut hasher = blake2_rfc::blake2s::Blake2s::new(HASH_LEN);
        hasher.update(bytes);
        let hash = hasher.finalize();
        slice_to_hash(hash.as_bytes())
    }
}

#[derive(Clone, Debug)]
pub struct Blake2b;
impl Hasher for Blake2b {
    fn new() -> Self {
        Blake2b
    }

    fn digest(&self, bytes: &[u8]) -> Hash {
        let mut hasher = blake2_rfc::blake2b::Blake2b::new(HASH_LEN);
        hasher.update(bytes);
        let hash = hasher.finalize();
        slice_to_hash(hash.as_bytes())
    }
}

#[derive(Clone, Debug)]
pub struct Blake3;
impl Hasher for Blake3 {
    fn new() -> Self {
        Blake3
    }

    // Note that currently supports 256-bit or 32-byte only
    fn digest(&self, bytes: &[u8]) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(bytes);
        let hash = hasher.finalize();
        slice_to_hash(hash.as_bytes())
    }
}

#[derive(Clone, Debug)]
pub struct Sha2;
impl Hasher for Sha2 {
    fn new() -> Self {
        Sha2
    }

    // Note that currently supports 256-bit or 32-byte only
    fn digest(&self, bytes: &[u8]) -> Hash {
        let mut hasher = sha2::Sha256::new();
        hasher.input(bytes);
        let hash = hasher.result();
        slice_to_hash(hash.as_slice())
    }
}

#[derive(Clone, Debug)]
pub struct Sha3;
impl Hasher for Sha3 {
    fn new() -> Self {
        Sha3
    }

    // Note that currently supports 256-bit or 32-byte only
    fn digest(&self, bytes: &[u8]) -> Hash {
        let mut hasher = sha3::Sha3_256::new();
        hasher.input(bytes);
        let hash = hasher.result();
        slice_to_hash(hash.as_slice())
    }
}
