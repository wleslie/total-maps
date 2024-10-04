use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
};

use serde::{Deserialize, Serialize};

use crate::{Commonality, TotalBTreeMap, TotalHashMap};

impl<K: Serialize, V: Serialize, C> Serialize for TotalHashMap<K, V, C> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_hash_map().serialize(serializer)
    }
}
impl<'de, K: Deserialize<'de> + Eq + Hash, V: Deserialize<'de>, C: Commonality<V>> Deserialize<'de>
    for TotalHashMap<K, V, C>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut this = Self::default();
        *this.as_hash_map_mut() = HashMap::deserialize(deserializer)?;
        Ok(this)
    }
}

impl<K: Serialize, V: Serialize, C> Serialize for TotalBTreeMap<K, V, C> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_btree_map().serialize(serializer)
    }
}
impl<'de, K: Deserialize<'de> + Ord, V: Deserialize<'de>, C: Commonality<V>> Deserialize<'de>
    for TotalBTreeMap<K, V, C>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut this = Self::default();
        *this.as_btree_map_mut() = BTreeMap::deserialize(deserializer)?;
        Ok(this)
    }
}
