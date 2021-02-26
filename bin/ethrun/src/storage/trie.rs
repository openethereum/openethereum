// Copyright 2021 The OpenEthereum Authors.
// Licensed under the Apache License, Version 2.0.

use super::nibble::{self, NibbleSlice};
use elastic_array::ElasticArray32;
use ethereum_types::{H256, U256};
use tiny_keccak::keccak256;

/// This type represents the fundamental storage data structure in Ethereum.
///
/// Instances of this type are used to store Account State, World State,
/// Transactions State and Receipts State
pub struct MerklePatriciaTree<'a> {
    root: MarklePatriciaTreeNode<'a>,
}

impl<'a> MerklePatriciaTree<'a> {
    /// Creates a new instance of the tree with no data in them.
    /// the root hash of such a tree is Keccak of an empty string.
    pub fn new() -> Self {
        MerklePatriciaTree {
            root: MarklePatriciaTreeNode::Empty,
        }
    }

    /// Gets the 256-bit Keccak hash of the entire tree.
    pub fn hash(&self) -> H256 {
        match self.root {
            MarklePatriciaTreeNode::Leaf { hash, .. } => hash,
            MarklePatriciaTreeNode::Branch { hash, .. } => hash,
            MarklePatriciaTreeNode::Extension { hash, .. } => hash,
            MarklePatriciaTreeNode::Empty => H256::from_slice(&keccak256(&[])),
        }
    }

    /// Inserts or updates a storage value located under "key".
    pub fn upsert<K: AsRef<[u8]>, V: Into<U256>>(&self, key: K, value: V) -> Option<U256> {
        let key = todo!();
    }

    pub fn delete<K: AsRef<[u8]>>(&self, key: K) -> Option<U256> {
        todo!();
    }

    pub fn get<K: AsRef<[u8]>>(&self, key: K) -> Option<U256> {
        let hashed_key = MerklePatriciaTree::secure_key(key);
        let path = NibbleSlice::new(&hashed_key);

        let localroot = &self.root;
        match localroot {
            MarklePatriciaTreeNode::Empty => None,
            _ => None
        }
    }

    fn secure_key<K: AsRef<[u8]>>(key: K) -> H256 {
        H256::from_slice(&keccak256(key.as_ref()))
    }
}

type ValueType = ElasticArray32<u8>;

enum MarklePatriciaTreeNode<'a> {
    Empty,
    Leaf {
        hash: H256,
        key_end: NibbleSlice<'a>,
        value: ValueType,
    },
    Branch {
        hash: H256,
        branches: [Option<u8>; 16],
        value: Option<ValueType>,
    },
    Extension {
        hash: H256,
        shared: NibbleSlice<'a>,
        value: ValueType,
    },
}

impl<'a> MarklePatriciaTreeNode<'a> {
    pub fn get() { todo!(); }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_nibble_iterator() {
        let data = [1, 2, 3, 4, 5];
        let nibbles = NibbleSlice::new(&data);

        assert_eq!(nibbles.len(), 10);

        let it = nibbles.iter();
        let expanded: Vec<_> = it.collect();
        println!("nibbles: {:?}", &expanded);
        assert_eq!(expanded[0], 0);
        assert_eq!(expanded[1], 1);
        assert_eq!(expanded[2], 0);
        assert_eq!(expanded[3], 2);
        assert_eq!(expanded[4], 0);
        assert_eq!(expanded[5], 3);
        assert_eq!(expanded[6], 0);
        assert_eq!(expanded[7], 4);
        assert_eq!(expanded[8], 0);
        assert_eq!(expanded[9], 5);
    }
}
