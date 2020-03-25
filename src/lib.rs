//! # Monotree
//! Rust implementation of an optimized Sparse Merkle Tree.  
//! This is a kind of binary-radix tree based on bitwise branching, _currently_, no nibble of bit (nor a 4-bit neither a byte nibble).  
//! The branching unit is _just a single bit_, for now.  
//!
//! ## Features
//! - Very simple, concise and __easy to read__, but __fast__ and robust.  
//! - __Fully featured__ Sparse Merkle Tree (SMT) as a storage
//! - <ins>This includes: __non-inclusion proof__ , as well as __inclusion proof__, and its verification.</ins>
//! - Again, _NOT verbose_ at all.  
//!
//! This library mostly relies on the _Rust standard library only_ except for `database APIs` and `hashers`.  
//! Currently, `monotree` supports these databases and hash functions following, but is designed to be super easy to customize and add:
//!
//! _Databases include_:
//! - [`HashMap`](https://lib.rs/crates/hashbrown)
//! - [`RocksDB`](https://lib.rs/crates/rocksdb)
//! - [`Sled`](https://lib.rs/crates/sled)
//!
//! _Hashers include_:
//! - [`Blake3`](https://lib.rs/crates/blake3)
//! - [`Blake2s`](https://lib.rs/crates/blake2-rfc) and [`Blake2b`](https://lib.rs/crates/blake2-rfc)
//! - [`SHA-2`](https://lib.rs/crates/sha2)
//! - [`SHA-3 (Keccak)`](https://lib.rs/crates/sha3)
use std::error::Error;
use std::fmt;
use std::ops::Range;

pub const HASH_LEN: usize = 32;
// pub const UNIT_BIT: usize = 4;
// pub const NL: usize = 1 << UNIT_BIT;
pub type BitsLen = u16;
pub type Result<T> = std::result::Result<T, Errors>;
pub type Hash = [u8; HASH_LEN];
pub type Proof = Vec<(bool, Vec<u8>)>;

#[macro_use]
pub mod utils;

#[derive(Debug, Clone, PartialEq)]
pub struct Bits<'a> {
    pub path: &'a [u8],
    pub range: Range<BitsLen>,
}
pub mod bits;

pub type Cell<'a> = Option<Unit<'a>>;

#[derive(Clone, Debug)]
pub enum Node<'a> {
    Soft(Cell<'a>),
    Hard(Cell<'a>, Cell<'a>),
}
pub mod node;

#[derive(Clone, Debug, PartialEq)]
pub struct Unit<'a> {
    pub hash: &'a [u8],
    pub bits: Bits<'a>,
}

pub trait Database {
    fn new(dbpath: &str) -> Self;
    fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()>;
    fn delete(&mut self, key: &[u8]) -> Result<()>;
    fn init_batch(&mut self) -> Result<()>;
    fn finish_batch(&mut self) -> Result<()>;
}
pub mod database;

pub trait Hasher {
    fn new() -> Self;
    fn digest(&self, bytes: &[u8]) -> Hash;
}
pub mod hasher;

pub type DefaultDatabase = database::MemoryDB;
pub type DefaultHasher = hasher::Blake3;

#[derive(Debug)]
pub struct Monotree<D = DefaultDatabase, H = DefaultHasher> {
    db: D,
    pub hasher: H,
}
pub mod tree;

#[derive(Debug)]
pub struct Errors {
    details: String,
}

impl Errors {
    pub fn new(msg: &str) -> Errors {
        Errors {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for Errors {
    fn description(&self) -> &str {
        &self.details
    }
}
