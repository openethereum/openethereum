use crate::blockchain::block::Block;
use serde_json::{self, Error};
use std::{collections::BTreeMap, io::Read};

/// Blockchain test deserializer.
#[derive(Debug, PartialEq, Deserialize)]
pub struct BlockEnDeTest(BTreeMap<String, Block>);

impl IntoIterator for BlockEnDeTest {
    type Item = <BTreeMap<String, Block> as IntoIterator>::Item;
    type IntoIter = <BTreeMap<String, Block> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl BlockEnDeTest {
    /// Loads test from json.
    pub fn load<R>(reader: R) -> Result<Self, Error>
    where
        R: Read,
    {
        serde_json::from_reader(reader)
    }
}
