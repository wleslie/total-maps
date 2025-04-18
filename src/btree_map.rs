//! Provides [TotalBTreeMap], an ordered map in which every possible key has an associated value.
//! Only entries with *uncommon* values are actually stored in the map; all other keys are presumed
//! to be associated with a *common* value.

use std::{
    borrow::Borrow,
    cmp::Ordering,
    collections::{btree_map, BTreeMap},
    fmt::{self, Debug, Formatter},
    hash::{Hash, Hasher},
    iter::FusedIterator,
    mem,
    ops::{Deref, DerefMut, Index},
};

use crate::{Commonality, DefaultCommonality, PhantomPtr};

// --------------------------------------------------------------------------

/// An ordered map in which every possible key has an associated value. Only entries with *uncommon*
/// values are actually stored in the map; all other keys are presumed to be associated with a
/// *common* value.
///
/// See the [crate documentation](crate) for more information.
///
/// The API is more-or-less a subset of that of [BTreeMap]. However, methods that treat this type
/// like a collection (for example, [`len()`](Self::len) and [`iter()`](Self::iter)) operate only on
/// the *uncommon* entries.
pub struct TotalBTreeMap<K, V, C = DefaultCommonality> {
    inner: BTreeMap<K, V>,
    common: V, // need to store this value so we can return references to it, e.g., in Self::get
    _commonality: PhantomPtr<C>,
}

impl<K: Clone, V: Clone, C> Clone for TotalBTreeMap<K, V, C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            common: self.common.clone(),
            _commonality: PhantomPtr::default(),
        }
    }
}

impl<K, V, C: Commonality<V>> Default for TotalBTreeMap<K, V, C> {
    fn default() -> Self {
        Self {
            inner: BTreeMap::default(),
            common: C::common(),
            _commonality: PhantomPtr::default(),
        }
    }
}
impl<K, V, C: Commonality<V>> TotalBTreeMap<K, V, C> {
    /// Constructs a `TotalBTreeMap` in which all keys are associated with the *common* value.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<K, V, C> TotalBTreeMap<K, V, C> {
    /// Returns the number of *uncommon* entries in the map.
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    /// Returns true if the map contains no *uncommon* entries.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    /// Resets all entries in the map to the *common* value.
    pub fn clear(&mut self) {
        self.inner.clear()
    }
}

// --------------------------------------------------------------------------
// Element access

impl<K: Ord, V, C> TotalBTreeMap<K, V, C> {
    /// Returns a reference to the value associated with the given key.
    pub fn get<Q>(&self, key: &Q) -> &V
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner.get(key).unwrap_or(&self.common)
    }
    /// Returns true if the map contains an *uncommon* entry with the given key.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner.contains_key(key)
    }
}

impl<K: Borrow<Q> + Ord, Q: Ord + ?Sized, V, C> Index<&Q> for TotalBTreeMap<K, V, C> {
    type Output = V;
    fn index(&self, index: &Q) -> &Self::Output {
        self.get(index)
    }
}

impl<K: Ord, V, C: Commonality<V>> TotalBTreeMap<K, V, C> {
    /// Associates a key with a value in the map, and returns the value previously associated with
    /// that key.
    pub fn insert(&mut self, key: K, value: V) -> V {
        if C::is_common(&value) { self.inner.remove(&key) } else { self.inner.insert(key, value) }
            .unwrap_or_else(C::common)
    }

    /// Associates a key with the *common* value in the map, and returns the value previously
    /// associated with that key.
    pub fn remove<Q>(&mut self, key: &Q) -> V
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner.remove(key).unwrap_or_else(C::common)
    }

    /// Gets the given key's associated entry in the map for in-place manipulation.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, K, V, C> {
        Entry {
            inner: match self.inner.entry(key) {
                btree_map::Entry::Occupied(inner) => EntryInner::Occupied { inner },
                btree_map::Entry::Vacant(inner) => EntryInner::Vacant { inner, value: C::common() },
            },
        }
    }

    /// Gets the given key's associated entry in the map if it has an *uncommon* value; otherwise
    /// returns [None].
    ///
    /// In contrast with [Self::entry], this method accepts the key in borrowed form.
    pub fn uncommon_entry<'a, Q>(&'a mut self, key: &'a Q) -> Option<Entry<'a, Q, K, V, C>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let map = self as *mut _;
        let value = self.inner.get_mut(key)?;
        Some(Entry { inner: EntryInner::ByRef { map, key, value } })
    }
}

/// A view into a single entry in a [TotalBTreeMap].
///
/// This view is constructed from [TotalBTreeMap::entry].
pub struct Entry<'a, Q, K, V, C = DefaultCommonality>
where
    Q: Ord + ?Sized,
    K: Ord + Borrow<Q>,
    C: Commonality<V>,
{
    inner: EntryInner<'a, Q, K, V, C>,
}

impl<Q, K, V, C> Deref for Entry<'_, Q, K, V, C>
where
    Q: Ord + ?Sized,
    K: Ord + Borrow<Q>,
    C: Commonality<V>,
{
    type Target = V;
    fn deref(&self) -> &Self::Target {
        match &self.inner {
            EntryInner::Occupied { inner } => inner.get(),
            EntryInner::Vacant { value, .. } => value,
            EntryInner::ByRef { value, .. } => value,
            EntryInner::Dropping => unreachable!(),
        }
    }
}
impl<Q, K, V, C> DerefMut for Entry<'_, Q, K, V, C>
where
    Q: Ord + ?Sized,
    K: Ord + Borrow<Q>,
    C: Commonality<V>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.inner {
            EntryInner::Occupied { inner } => inner.get_mut(),
            EntryInner::Vacant { value, .. } => value,
            EntryInner::ByRef { value, .. } => value,
            EntryInner::Dropping => unreachable!(),
        }
    }
}

impl<Q, K, V, C> Drop for Entry<'_, Q, K, V, C>
where
    Q: Ord + ?Sized,
    K: Ord + Borrow<Q>,
    C: Commonality<V>,
{
    fn drop(&mut self) {
        match mem::replace(&mut self.inner, EntryInner::Dropping) {
            EntryInner::Occupied { inner } => {
                if C::is_common(inner.get()) {
                    inner.remove();
                }
            }
            EntryInner::Vacant { inner, value } => {
                if !C::is_common(&value) {
                    inner.insert(value);
                }
            }
            EntryInner::ByRef { map, key, value } => {
                if C::is_common(value) {
                    unsafe { &mut *map }.remove(key);
                }
            }
            EntryInner::Dropping => unreachable!(),
        }
    }
}

impl<'a, Q, K, V, C> Debug for Entry<'a, Q, K, V, C>
where
    Q: Debug + Ord + ?Sized,
    K: Debug + Ord + Borrow<Q>,
    V: Debug,
    C: Commonality<V>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_tuple("Entry");
        match &self.inner {
            EntryInner::Occupied { inner } => f.field(inner.key()).field(inner.get()),
            EntryInner::Vacant { inner, value } => f.field(inner.key()).field(value),
            EntryInner::ByRef { key, value, .. } => f.field(key).field(value),
            EntryInner::Dropping => &mut f,
        };
        f.finish()
    }
}

enum EntryInner<'a, Q: ?Sized, K, V, C> {
    Occupied { inner: btree_map::OccupiedEntry<'a, K, V> },
    Vacant { inner: btree_map::VacantEntry<'a, K, V>, value: V },
    ByRef { map: *mut TotalBTreeMap<K, V, C>, key: &'a Q, value: &'a mut V },
    Dropping,
}

// --------------------------------------------------------------------------
// Iteration

impl<K, V, C> TotalBTreeMap<K, V, C> {
    /// An iterator over all keys associated with *uncommon* values in the map, in sorted order.
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys(self.inner.keys())
    }
    /// Creates a consuming iterator over all keys associated with *uncommon* values in the map, in
    /// sorted order.
    pub fn into_keys(self) -> IntoKeys<K, V> {
        IntoKeys(self.inner.into_keys())
    }
    /// An iterator over all *uncommon* values in the map, in sorted order.
    pub fn values(&self) -> Values<'_, K, V> {
        Values(self.inner.values())
    }
    /// Creates a consuming iterator over all *uncommon* values in the map, in sorted order.
    pub fn into_values(self) -> IntoValues<K, V> {
        IntoValues(self.inner.into_values())
    }
    /// An iterator over all *uncommon* entries in the map, in sorted order.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter(self.inner.iter())
    }
}

impl<K, V, C> IntoIterator for TotalBTreeMap<K, V, C> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.inner.into_iter())
    }
}
impl<'a, K, V, C> IntoIterator for &'a TotalBTreeMap<K, V, C> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over the keys associated with *uncommon* values in a [TotalBTreeMap].
///
/// This iterator is created by [TotalBTreeMap::keys].
pub struct Keys<'a, K, V>(btree_map::Keys<'a, K, V>);
impl<K, V> Clone for Keys<'_, K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<'a, K, V> Default for Keys<'a, K, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<K, V> DoubleEndedIterator for Keys<'_, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}
impl<K, V> ExactSizeIterator for Keys<'_, K, V> {
    fn len(&self) -> usize {
        self.0.len()
    }
}
impl<K, V> FusedIterator for Keys<'_, K, V> {}
impl<K: Debug, V: Debug> Debug for Keys<'_, K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// An owning iterator over the keys associated with *uncommon* values in a [TotalBTreeMap].
///
/// This iterator is created by [TotalBTreeMap::into_keys].
pub struct IntoKeys<K, V>(btree_map::IntoKeys<K, V>);
impl<K, V> Default for IntoKeys<K, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<K, V> Iterator for IntoKeys<K, V> {
    type Item = K;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<K, V> DoubleEndedIterator for IntoKeys<K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}
impl<K, V> ExactSizeIterator for IntoKeys<K, V> {
    fn len(&self) -> usize {
        self.0.len()
    }
}
impl<K, V> FusedIterator for IntoKeys<K, V> {}

/// An iterator over the *uncommon* values in a [TotalBTreeMap].
///
/// This iterator is created by [TotalBTreeMap::values].
pub struct Values<'a, K, V>(btree_map::Values<'a, K, V>);
impl<K, V> Clone for Values<'_, K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<'a, K, V> Default for Values<'a, K, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<'a, K, V> DoubleEndedIterator for Values<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}
impl<K, V> ExactSizeIterator for Values<'_, K, V> {
    fn len(&self) -> usize {
        self.0.len()
    }
}
impl<K, V> FusedIterator for Values<'_, K, V> {}
impl<K: Debug, V: Debug> Debug for Values<'_, K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// An owning iterator over the *uncommon* values in a [TotalBTreeMap].
///
/// This iterator is created by [TotalBTreeMap::into_values].
pub struct IntoValues<K, V>(btree_map::IntoValues<K, V>);
impl<K, V> Default for IntoValues<K, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<K, V> Iterator for IntoValues<K, V> {
    type Item = V;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<K, V> DoubleEndedIterator for IntoValues<K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}
impl<K, V> ExactSizeIterator for IntoValues<K, V> {
    fn len(&self) -> usize {
        self.0.len()
    }
}
impl<K, V> FusedIterator for IntoValues<K, V> {}

/// An iterator over the *uncommon* entries in a [TotalBTreeMap].
///
/// This iterator is created by [TotalBTreeMap::iter].
pub struct Iter<'a, K, V>(btree_map::Iter<'a, K, V>);
impl<K, V> Clone for Iter<'_, K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<'a, K, V> Default for Iter<'a, K, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<'a, K, V> DoubleEndedIterator for Iter<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}
impl<K, V> ExactSizeIterator for Iter<'_, K, V> {
    fn len(&self) -> usize {
        self.0.len()
    }
}
impl<K, V> FusedIterator for Iter<'_, K, V> {}
impl<K: Debug, V: Debug> Debug for Iter<'_, K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// An owning iterator over the *uncommon* entries in a [TotalBTreeMap].
///
/// This iterator is created by [TotalBTreeMap]'s implementation of [IntoIterator].
pub struct IntoIter<K, V>(btree_map::IntoIter<K, V>);
impl<K, V> Default for IntoIter<K, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<K, V> DoubleEndedIterator for IntoIter<K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}
impl<K, V> ExactSizeIterator for IntoIter<K, V> {
    fn len(&self) -> usize {
        self.0.len()
    }
}
impl<K, V> FusedIterator for IntoIter<K, V> {}

// --------------------------------------------------------------------------
// Population from iterators

impl<K: Ord, V, C: Commonality<V>> Extend<(K, V)> for TotalBTreeMap<K, V, C> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}
impl<K: Ord, V, C: Commonality<V>> FromIterator<(K, V)> for TotalBTreeMap<K, V, C> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut this = Self::default();
        this.extend(iter);
        this
    }
}

// --------------------------------------------------------------------------
// Low-level access

impl<K, V, C> TotalBTreeMap<K, V, C> {
    /// Returns a view into the underlying [BTreeMap] of a [TotalBTreeMap], which contains the
    /// *uncommon* entries.
    pub fn as_btree_map(&self) -> &BTreeMap<K, V> {
        &self.inner
    }
}

impl<K: Ord, V, C: Commonality<V>> TotalBTreeMap<K, V, C> {
    /// Returns a mutable view into the underlying [BTreeMap] of a [TotalBTreeMap], from which
    /// mutating iterators can be obtained by calling [BTreeMap::values_mut] or
    /// [BTreeMap::iter_mut].
    ///
    /// By directly mutating the underlying [BTreeMap], it is possible to store *uncommon* entries
    /// in the map temporarily. When the returned view is dropped, all *uncommon* entries will be
    /// removed, restoring the invariant of [TotalBTreeMap].
    ///
    /// You don't need this method if you are only mutating individual entries; use the
    /// [entry][Self::entry] method instead.
    pub fn as_btree_map_mut(&mut self) -> AsBTreeMapMut<'_, K, V, C> {
        AsBTreeMapMut { map: &mut self.inner, _commonality: PhantomPtr::default() }
    }
}

/// A mutable view into the underlying [BTreeMap] of a [TotalBTreeMap].
///
/// This view is created by [TotalBTreeMap::as_btree_map_mut].
pub struct AsBTreeMapMut<'a, K: Ord, V, C: Commonality<V> = DefaultCommonality> {
    map: &'a mut BTreeMap<K, V>,
    _commonality: PhantomPtr<C>,
}

impl<K: Ord, V, C: Commonality<V>> Deref for AsBTreeMapMut<'_, K, V, C> {
    type Target = BTreeMap<K, V>;
    fn deref(&self) -> &Self::Target {
        self.map
    }
}
impl<K: Ord, V, C: Commonality<V>> DerefMut for AsBTreeMapMut<'_, K, V, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.map
    }
}

impl<K: Ord, V, C: Commonality<V>> Drop for AsBTreeMapMut<'_, K, V, C> {
    fn drop(&mut self) {
        self.map.retain(|_, value| !C::is_common(value));
    }
}

impl<K: Ord, V: PartialEq, C: Commonality<V>> PartialEq for AsBTreeMapMut<'_, K, V, C> {
    fn eq(&self, other: &Self) -> bool {
        self.map == other.map
        // deliberately ignoring commonality
    }
}
impl<K: Ord, V: Eq, C: Commonality<V>> Eq for AsBTreeMapMut<'_, K, V, C> {}
impl<K: Ord, V: PartialOrd, C: Commonality<V>> PartialOrd for AsBTreeMapMut<'_, K, V, C> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.map.partial_cmp(&other.map)
        // deliberately ignoring commonality
    }
}
impl<K: Ord, V: Ord, C: Commonality<V>> Ord for AsBTreeMapMut<'_, K, V, C> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.map.cmp(&other.map)
        // deliberately ignoring commonality
    }
}
impl<K: Ord + Hash, V: Hash, C: Commonality<V>> Hash for AsBTreeMapMut<'_, K, V, C> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.map.hash(state);
        // deliberately ignoring commonality
    }
}
impl<K: Ord + Debug, V: Debug, C: Commonality<V>> Debug for AsBTreeMapMut<'_, K, V, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AsBTreeMapMut").field(&self.map).finish()
    }
}

// --------------------------------------------------------------------------
// Miscellaneous traits

impl<K: PartialEq, V: PartialEq, C> PartialEq for TotalBTreeMap<K, V, C> {
    fn eq(&self, other: &Self) -> bool {
        // There is no bound on C: Commonality<V>, so we can't assume self.common == other.common
        self.common == other.common && self.inner == other.inner
    }
}
impl<K: Eq, V: Eq, C> Eq for TotalBTreeMap<K, V, C> {}

impl<K: PartialOrd, V: PartialOrd, C> PartialOrd for TotalBTreeMap<K, V, C> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // There is no bound on C: Commonality<V>, so we can't assume self.common == other.common
        match self.common.partial_cmp(&other.common) {
            Some(Ordering::Equal) => {}
            ord => return ord,
        };
        self.inner.partial_cmp(&other.inner)
    }
}
impl<K: Ord, V: Ord, C> Ord for TotalBTreeMap<K, V, C> {
    fn cmp(&self, other: &Self) -> Ordering {
        // There is no bound on C: Commonality<V>, so we can't assume self.common == other.common
        self.common.cmp(&other.common).then_with(|| self.inner.cmp(&other.inner))
    }
}

impl<K: Hash, V: Hash, C> Hash for TotalBTreeMap<K, V, C> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // There is no bound on C: Commonality<V>, so we can't assume self.common == other.common
        self.common.hash(state);
        self.inner.hash(state);
    }
}

impl<K: Debug, V: Debug, C> Debug for TotalBTreeMap<K, V, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        struct Rest;
        impl Debug for Rest {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "...")
            }
        }
        f.debug_map().entries(self.iter()).entry(&Rest, &self.common).finish()
    }
}
