use crate::utils::*;
use crate::*;

impl Default for Monotree<DefaultDatabase, DefaultHasher> {
    fn default() -> Self {
        Self::new("monotree")
    }
}

/// Example: How to use monotree
/// ```
/// use monotree::{Monotree, Result};
/// use monotree::utils::random_hash;
///
/// fn example() -> Result<()> {
///     // Init a monotree instance
///     // by default, with 'HashMap' and 'Blake3' hash function
///     let mut tree = Monotree::default();
///
///     // It is natural the tree root initially has 'None'
///     let root = None;
///
///     // Prepare a random pair of key and leaf.
///     // random_hashes() gives a fixed length of random array,
///     // where Hash -> [u8; HASH_LEN], HASH_LEN = 32
///     let key = random_hash();
///     let leaf = random_hash();
///
///     // Insert the entry (key, leaf) into tree, yielding a new root of tree
///     let root = tree.insert(root.as_ref(), &key, &leaf)?;
///     assert_ne!(root, None);
///
///     // Get the leaf inserted just before. Note that the last root was used.
///     let found = tree.get(root.as_ref(), &key)?;
///     assert_eq!(found, Some(leaf));
///
///     // Remove the entry
///     let root = tree.remove(root.as_ref(), &key)?;
///
///     // surely, the tree has nothing and the root back to 'None'
///     assert_eq!(tree.get(root.as_ref(), &key)?, None);
///     assert_eq!(root, None);
///     Ok(())
/// }
/// ```
impl<D, H> Monotree<D, H>
where
    D: Database,
    H: Hasher,
{
    pub fn new(dbpath: &str) -> Self {
        let db = Database::new(dbpath);
        let hasher = Hasher::new();
        Monotree { db, hasher }
    }

    pub fn insert(&mut self, root: Option<&Hash>, key: &Hash, leaf: &Hash) -> Result<Option<Hash>> {
        match root {
            None => {
                let (hash, bits) = (leaf, Bits::new(key));
                self.put_node(Node::new(Some(Unit { hash, bits }), None))
            }
            Some(root) => self.put(root, Bits::new(key), leaf),
        }
    }

    fn put_node(&mut self, node: Node) -> Result<Option<Hash>> {
        let bytes = node.to_bytes()?;
        let hash = self.hasher.digest(&bytes);
        self.db.put(&hash, bytes)?;
        Ok(Some(hash))
    }

    /// Recursively insert a bytes (in forms of Bits) and a leaf into the tree
    /// Whenever invoked a `put()` call, at least, more than one `put_node()` called,
    /// which triggers a single hash digest + a single DB write.
    /// Optimizations in monotree is to compress the path as much as possible
    /// while reducing the number of db accesses using the most intutive model
    ///
    /// There are four modes when putting the entries: One of them is processed each (recursive) call
    /// - (1) set-aside: putting the leaf to the next node in the curruent depth
    /// - (1) replacement: replacement the existing node on path with the leaf
    /// - (2+) consume & pass-over: consuming the path on the way, then pass the rest of work to child node
    /// - (2) split-node: immideately split node into two with the logest common prefix, then wind recursive stack.
    /// the number in parenthesis refers to the minimum of DB access and hash fn call required.
    fn put(&mut self, root: &[u8], bits: Bits, leaf: &[u8]) -> Result<Option<Hash>> {
        let bytes = self.db.get(root)?.expect("bytes");
        let (lc, rc) = Node::cells_from_bytes(&bytes, bits.first())?;
        let unit = lc.as_ref().expect("put(): left-unit");
        let n = Bits::len_common_bits(&unit.bits, &bits);
        match n {
            n if n == 0 => self.put_node(Node::new(lc, Some(Unit { hash: leaf, bits }))),
            n if n == bits.len() => self.put_node(Node::new(Some(Unit { hash: leaf, bits }), rc)),
            n if n == unit.bits.len() => {
                let hash = &self
                    .put(unit.hash, bits.shift(n, false), leaf)?
                    .expect("put(): hash");
                let unit = unit.to_owned();
                self.put_node(Node::new(Some(Unit { hash, ..unit }), rc))
            }
            _ => {
                let bits = bits.shift(n, false);
                let ru = Unit { hash: leaf, bits };

                let (cloned, unit) = (unit.bits.clone(), unit.to_owned());
                let (hash, bits) = (unit.hash, unit.bits.shift(n, false));
                let lu = Unit { hash, bits };

                let hash = &self
                    .put_node(Node::new(Some(lu), Some(ru)))?
                    .expect("put(): hash");
                let bits = cloned.shift(n, true);
                self.put_node(Node::new(Some(Unit { hash, bits }), rc))
            }
        }
    }

    pub fn get(&mut self, root: Option<&Hash>, key: &Hash) -> Result<Option<Hash>> {
        match root {
            None => Ok(None),
            Some(root) => self.find_key(root, Bits::new(key)),
        }
    }

    fn find_key(&mut self, root: &[u8], bits: Bits) -> Result<Option<Hash>> {
        let bytes = self.db.get(root)?.expect("bytes");
        let (cell, _) = Node::cells_from_bytes(&bytes, bits.first())?;
        let unit = cell.as_ref().expect("find_key(): left-unit");
        let n = Bits::len_common_bits(&unit.bits, &bits);
        match n {
            n if n == bits.len() => Ok(Some(slice_to_hash(unit.hash))),
            n if n == unit.bits.len() => self.find_key(&unit.hash, bits.shift(n, false)),
            _ => Ok(None),
        }
    }

    pub fn remove(&mut self, root: Option<&Hash>, key: &[u8]) -> Result<Option<Hash>> {
        match root {
            None => Ok(None),
            Some(root) => self.delete_key(root, Bits::new(key)),
        }
    }

    fn delete_key(&mut self, root: &[u8], bits: Bits) -> Result<Option<Hash>> {
        let bytes = self.db.get(root)?.expect("bytes");
        let (lc, rc) = Node::cells_from_bytes(&bytes, bits.first())?;
        let unit = lc.as_ref().expect("delete_key(): left-unit");
        let n = Bits::len_common_bits(&unit.bits, &bits);
        match n {
            n if n == bits.len() => match rc {
                Some(_) => self.put_node(Node::new(None, rc)),
                None => Ok(None),
            },
            n if n == unit.bits.len() => {
                let hash = self.delete_key(&unit.hash, bits.shift(n, false))?;
                match (hash, &rc) {
                    (None, None) => Ok(None),
                    (None, Some(_)) => self.put_node(Node::new(None, rc)),
                    (Some(ref hash), _) => {
                        let unit = unit.to_owned();
                        let lc = Some(Unit { hash, ..unit });
                        self.put_node(Node::new(lc, rc))
                    }
                }
            }
            _ => Ok(None),
        }
    }

    /// This method is for batch use of `insert()` method
    /// input: slice of each keys and leaves.
    pub fn inserts(
        &mut self,
        root: Option<&Hash>,
        keys: &[Hash],
        leaves: &[Hash],
    ) -> Result<Option<Hash>> {
        let indices = get_sorted_indices(keys, false);
        self.db.init_batch()?;
        let mut root = root.cloned();
        for i in indices.iter() {
            root = self.insert(root.as_ref(), &keys[*i], &leaves[*i])?;
        }
        self.db.finish_batch()?;
        Ok(root)
    }

    /// This method is for batch use of `get()` method
    /// output: vector of leaves retrieved
    pub fn gets(&mut self, root: Option<&Hash>, keys: &[Hash]) -> Result<Vec<Option<Hash>>> {
        let mut leaves: Vec<Option<Hash>> = Vec::new();
        for key in keys.iter() {
            leaves.push(self.get(root, key)?);
        }
        Ok(leaves)
    }

    /// This method is for batch use of `remove()` method
    /// input: slice of each keys and leaves.
    pub fn removes(&mut self, root: Option<&Hash>, keys: &[Hash]) -> Result<Option<Hash>> {
        let indices = get_sorted_indices(keys, false);
        let mut root = root.cloned();
        self.db.init_batch()?;
        for i in indices.iter() {
            root = self.remove(root.as_ref(), &keys[*i])?;
        }
        self.db.finish_batch()?;
        Ok(root)
    }

    /// `Merkle proof` secion: verifying inclusion of data (inclusion proof)
    /// --------------------------------------------------------------------
    /// `Monotree` has compressed representation, but it fully retains
    /// the properties of the Sparse Merkle Tree (SMT).
    /// Thus, `non-inclusion proof` is quite straightforward. Just go walk down
    /// the tree with a key (or a path) given. If we cannot successfully get a leaf,
    /// we can assure that the leaf is not a part of the tree.
    /// The process of inclusion proof is below:
    ///
    /// ```
    /// use monotree::tree::verify_proof;
    /// use monotree::utils::random_hashes;
    /// use monotree::hasher::Blake3;
    /// use monotree::{Hasher, Monotree, Result};
    ///
    /// fn example() -> Result<()> {
    ///     // random pre-insertion for Merkle proof test
    ///     let mut tree = Monotree::default();
    ///     let root = None;
    ///     let keys = random_hashes(500);
    ///     let leaves = random_hashes(500);
    ///     let root = tree.inserts(root.as_ref(), &keys, &leaves)?;
    ///
    ///     // pick a random key from keys among inserted just before
    ///     let key = keys[99];
    ///
    ///     // generate the Merkle proof for the root and the key
    ///     let proof = tree.get_merkle_proof(root.as_ref(), &key)?;
    ///
    ///     // To verify the proof correctly, you need to provide a hasher matched
    ///     // the default tree was initialized with `Blake3`
    ///     let hasher = Blake3::new();
    ///
    ///     // get a leaf matched with the key: where the Merkle proof starts off
    ///     let leaf = leaves[99];
    ///
    ///     // verify the Merkle proof using all those above
    ///     let verified = verify_proof(&hasher, root.as_ref(), &leaf, proof.as_ref());
    ///     assert_eq!(verified, true);
    ///     Ok(())
    /// }
    /// ```
    pub fn get_merkle_proof(&mut self, root: Option<&Hash>, key: &[u8]) -> Result<Option<Proof>> {
        let mut proof: Proof = Vec::new();
        match root {
            None => Ok(None),
            Some(root) => self.gen_proof(root, Bits::new(key), &mut proof),
        }
    }

    fn gen_proof(&mut self, root: &[u8], bits: Bits, proof: &mut Proof) -> Result<Option<Proof>> {
        let bytes = self.db.get(root)?.expect("bytes");
        let (cell, _) = Node::cells_from_bytes(&bytes, bits.first())?;
        let unit = cell.as_ref().expect("gen_proof(): left-unit");
        let n = Bits::len_common_bits(&unit.bits, &bits);
        match n {
            n if n == bits.len() => {
                proof.push(self.encode_proof(&bytes, bits.first())?);
                Ok(Some(proof.to_owned()))
            }
            n if n == unit.bits.len() => {
                proof.push(self.encode_proof(&bytes, bits.first())?);
                self.gen_proof(unit.hash, bits.shift(n, false), proof)
            }
            _ => Ok(None),
        }
    }

    fn encode_proof(&self, bytes: &[u8], right: bool) -> Result<(bool, Vec<u8>)> {
        match Node::from_bytes(bytes)? {
            Node::Soft(_) => Ok((false, bytes[HASH_LEN..].to_vec())),
            Node::Hard(_, _) => {
                if right {
                    Ok((
                        true,
                        [&bytes[..bytes.len() - HASH_LEN - 1], &[0x01]].concat(),
                    ))
                } else {
                    Ok((false, bytes[HASH_LEN..].to_vec()))
                }
            }
        }
    }
}

/// Verify a Merkle proof with the given root, leaf and hasher
/// Be aware of that it fails if not provided a suitable hasher used in the tree
/// This generic fn must be independantly called upon request, not a member of Monotree.
pub fn verify_proof<H: Hasher>(
    hasher: &H,
    root: Option<&Hash>,
    leaf: &Hash,
    proof: Option<&Proof>,
) -> bool {
    match proof {
        None => false,
        Some(proof) => {
            let mut hash = leaf.to_owned();
            proof.iter().rev().for_each(|(right, cut)| {
                if *right {
                    let l = cut.len();
                    let o = [&cut[..l - 1], &hash[..], &cut[l - 1..]].concat();
                    hash = hasher.digest(&o);
                } else {
                    let o = [&hash[..], &cut[..]].concat();
                    hash = hasher.digest(&o);
                }
            });
            root.expect("verify_proof(): root") == &hash
        }
    }
}
