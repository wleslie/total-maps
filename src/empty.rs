//! Provides [EmptyCommonality], a [commonality](Commonality) for collection types where the common
//! value is an empty collection.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use crate::{Commonality, TotalBTreeMap, TotalHashMap};

/// A [commonality](Commonality) for collection types, where the common value is an empty
/// collection.
///
/// In most cases, this commonality behaves the same as
/// [DefaultCommonality][super::DefaultCommonality] because typical collection types'
/// implementations of [Default::default] yield empty collections. However, this commonality avoids
/// the need for [PartialEq] bounds on the collection type.
pub struct EmptyCommonality(());

macro_rules! impl_empty {
    ({$($generics:tt)*}, $Coll:path $(,)?) => {
        impl<$($generics)*> Commonality<$Coll> for EmptyCommonality {
            fn common() -> $Coll {
                Default::default()
            }
            fn is_common(value: &$Coll) -> bool {
                value.is_empty()
            }
        }
    };
}

impl_empty!({ T }, Vec<T>);
impl_empty!({ T }, VecDeque<T>);
impl_empty!({ T }, HashSet<T>);
impl_empty!({ K, V }, HashMap<K, V>);
impl_empty!({ T }, BTreeSet<T>);
impl_empty!({ K, V }, BTreeMap<K, V>);
impl_empty!({ K, V, C: Commonality<V> }, TotalHashMap<K, V, C>);
impl_empty!({ K, V, C: Commonality<V> }, TotalBTreeMap<K, V, C>);
