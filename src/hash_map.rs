//! Provides [TotalHashMap], a hash map in which every possible key has an associated value. Only
//! entries with *uncommon* values are actually stored in the map; all other keys are presumed to be
//! associated with a *common* value.

use std::{
    borrow::Borrow,
    collections::{hash_map, HashMap},
    fmt::{self, Debug, Formatter},
    hash::Hash,
    iter::FusedIterator,
    mem,
    ops::{Deref, DerefMut, Index},
};

use crate::{Commonality, DefaultCommonality, PhantomPtr};

// --------------------------------------------------------------------------

/// A hash map in which every possible key has an associated value. Only entries with *uncommon*
/// values are actually stored in the map; all other keys are presumed to be associated with a
/// *common* value.
///
/// See the [crate documentation](crate) for more information.
///
/// The API more-or-less matches that of [HashMap]. However, methods that treat this type like a
/// collection (for example, [`len()`](Self::len) and [`iter()`](Self::iter)) operate only on the
/// *uncommon* entries.
pub struct TotalHashMap<K, V, C = DefaultCommonality> {
    inner: HashMap<K, V>,
    common: V, // need to store this value so we can return references to it, e.g., in Self::get
    _commonality: PhantomPtr<C>,
}

impl<K: Clone, V: Clone, C> Clone for TotalHashMap<K, V, C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            common: self.common.clone(),
            _commonality: PhantomPtr::default(),
        }
    }
}

impl<K, V, C: Commonality<V>> Default for TotalHashMap<K, V, C> {
    fn default() -> Self {
        Self { inner: HashMap::default(), common: C::common(), _commonality: PhantomPtr::default() }
    }
}
impl<K, V, C: Commonality<V>> TotalHashMap<K, V, C> {
    /// Constructs a `TotalHashMap` in which all keys are associated with the *common* value.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<K, V, C> TotalHashMap<K, V, C> {
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

impl<K: Eq + Hash, V, C> TotalHashMap<K, V, C> {
    /// Returns a reference to the value associated with the given key.
    pub fn get<Q>(&self, key: &Q) -> &V
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.inner.get(key).unwrap_or(&self.common)
    }
    /// Returns true if the map contains an *uncommon* entry with the given key.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.inner.contains_key(key)
    }
}

impl<K: Eq + Hash + Borrow<Q>, Q: Eq + Hash + ?Sized, V, C> Index<&Q> for TotalHashMap<K, V, C> {
    type Output = V;
    fn index(&self, index: &Q) -> &Self::Output {
        self.get(index)
    }
}

impl<K: Eq + Hash, V, C: Commonality<V>> TotalHashMap<K, V, C> {
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
        Q: Eq + Hash + ?Sized,
    {
        self.inner.remove(key).unwrap_or_else(C::common)
    }

    /// Gets the given key's associated entry in the map for in-place manipulation.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V, C> {
        Entry {
            inner: match self.inner.entry(key) {
                hash_map::Entry::Occupied(inner) => EntryInner::Occupied { inner },
                hash_map::Entry::Vacant(inner) => EntryInner::Vacant { inner, value: C::common() },
            },
            _commonality: PhantomPtr::default(),
        }
    }
}

/// A view into a single entry in a [TotalHashMap].
///
/// This view is constructed from [TotalHashMap::entry].
pub struct Entry<'a, K, V, C: Commonality<V> = DefaultCommonality> {
    inner: EntryInner<'a, K, V>,
    _commonality: PhantomPtr<C>,
}

impl<K, V, C: Commonality<V>> Deref for Entry<'_, K, V, C> {
    type Target = V;
    fn deref(&self) -> &Self::Target {
        match &self.inner {
            EntryInner::Occupied { inner } => inner.get(),
            EntryInner::Vacant { value, .. } => value,
            EntryInner::Dropping => unreachable!(),
        }
    }
}
impl<K, V, C: Commonality<V>> DerefMut for Entry<'_, K, V, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.inner {
            EntryInner::Occupied { inner } => inner.get_mut(),
            EntryInner::Vacant { value, .. } => value,
            EntryInner::Dropping => unreachable!(),
        }
    }
}

impl<K, V, C: Commonality<V>> Drop for Entry<'_, K, V, C> {
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
            EntryInner::Dropping => unreachable!(),
        }
    }
}

impl<'a, K: Debug, V: Debug, C: Commonality<V>> Debug for Entry<'a, K, V, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_tuple("Entry");
        match &self.inner {
            EntryInner::Occupied { inner } => f.field(inner.key()).field(inner.get()),
            EntryInner::Vacant { inner, value } => f.field(inner.key()).field(value),
            EntryInner::Dropping => &mut f,
        };
        f.finish()
    }
}

enum EntryInner<'a, K, V> {
    Occupied { inner: hash_map::OccupiedEntry<'a, K, V> },
    Vacant { inner: hash_map::VacantEntry<'a, K, V>, value: V },
    Dropping,
}

// --------------------------------------------------------------------------
// Iteration

impl<K, V, C> TotalHashMap<K, V, C> {
    /// An iterator over all keys associated with *uncommon* values in the map, in arbitrary order.
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys(self.inner.keys())
    }
    /// Creates a consuming iterator over all keys associated with *uncommon* values in the map, in
    /// arbitrary order.
    pub fn into_keys(self) -> IntoKeys<K, V> {
        IntoKeys(self.inner.into_keys())
    }
    /// An iterator over all *uncommon* values in the map, in arbitrary order.
    pub fn values(&self) -> Values<'_, K, V> {
        Values(self.inner.values())
    }
    /// An iterator over all *uncommon* values in the map, in arbitrary order, allowing mutation of
    /// the values.
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V, C>
    where
        C: Commonality<V>,
    {
        ValuesMut::new(self)
    }
    /// Creates a consuming iterator over all *uncommon* values in the map, in arbitrary order.
    pub fn into_values(self) -> IntoValues<K, V> {
        IntoValues(self.inner.into_values())
    }
    /// An iterator over all *uncommon* entries in the map, in arbitrary order.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter(self.inner.iter())
    }
    /// An iterator over all *uncommon* entries in the map, in arbitrary order, allowing mutation of
    /// the values.
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V, C>
    where
        C: Commonality<V>,
    {
        IterMut::new(self)
    }
    /// Resets all entries in the map to the *common* value, and returns all previously *uncommon*
    /// entries as an iterator.
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        Drain(self.inner.drain())
    }
}

impl<K, V, C> IntoIterator for TotalHashMap<K, V, C> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.inner.into_iter())
    }
}
impl<'a, K, V, C> IntoIterator for &'a TotalHashMap<K, V, C> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, K, V, C: Commonality<V>> IntoIterator for &'a mut TotalHashMap<K, V, C> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V, C>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// An iterator over the keys associated with *uncommon* values in a [TotalHashMap].
///
/// This iterator is created by [TotalHashMap::keys].
pub struct Keys<'a, K, V>(hash_map::Keys<'a, K, V>);
impl<K, V> Clone for Keys<'_, K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
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

/// An owning iterator over the keys associated with *uncommon* values in a [TotalHashMap].
///
/// This iterator is created by [TotalHashMap::into_keys].
pub struct IntoKeys<K, V>(hash_map::IntoKeys<K, V>);
impl<K, V> Iterator for IntoKeys<K, V> {
    type Item = K;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<K, V> ExactSizeIterator for IntoKeys<K, V> {
    fn len(&self) -> usize {
        self.0.len()
    }
}
impl<K, V> FusedIterator for IntoKeys<K, V> {}

/// An iterator over the *uncommon* values in a [TotalHashMap].
///
/// This iterator is created by [TotalHashMap::values].
pub struct Values<'a, K, V>(hash_map::Values<'a, K, V>);
impl<K, V> Clone for Values<'_, K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
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

/// An iterator over the *uncommon* values in a [TotalHashMap] that allows mutation of the values.
///
/// This iterator is created by [TotalHashMap::values_mut].
pub struct ValuesMut<'a, K, V, C: Commonality<V> = DefaultCommonality> {
    // Always Some except while dropping
    inner: Option<hash_map::ValuesMut<'a, K, V>>,
    // Used while dropping to restore invariant. Must be a raw pointer because the other field
    // borrows from it mutably
    map: *mut TotalHashMap<K, V, C>,
}
impl<'a, K, V, C: Commonality<V>> ValuesMut<'a, K, V, C> {
    fn new(map: &'a mut TotalHashMap<K, V, C>) -> Self {
        Self { map: map as *mut _, inner: Some(map.inner.values_mut()) }
    }
}
impl<'a, K, V, C: Commonality<V>> Iterator for ValuesMut<'a, K, V, C> {
    type Item = &'a mut V;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.as_mut().unwrap().next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.as_ref().unwrap().size_hint()
    }
}
impl<K, V, C: Commonality<V>> ExactSizeIterator for ValuesMut<'_, K, V, C> {
    fn len(&self) -> usize {
        self.inner.as_ref().unwrap().len()
    }
}
impl<K, V, C: Commonality<V>> FusedIterator for ValuesMut<'_, K, V, C> {}
impl<K, V, C: Commonality<V>> Drop for ValuesMut<'_, K, V, C> {
    fn drop(&mut self) {
        // FIXME: this is unsound because the mutable references yielded by this iterator are
        // allowed to outlive the iterator
        let _ = self.inner.take().unwrap();
        let map = unsafe { &mut *self.map };
        map.inner.retain(|_, value| !C::is_common(value));
    }
}

/// An owning iterator over the *uncommon* values in a [TotalHashMap].
///
/// This iterator is created by [TotalHashMap::into_values].
pub struct IntoValues<K, V>(hash_map::IntoValues<K, V>);
impl<K, V> Iterator for IntoValues<K, V> {
    type Item = V;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<K, V> ExactSizeIterator for IntoValues<K, V> {
    fn len(&self) -> usize {
        self.0.len()
    }
}
impl<K, V> FusedIterator for IntoValues<K, V> {}

/// An iterator over the *uncommon* entries in a [TotalHashMap].
///
/// This iterator is created by [TotalHashMap::iter].
pub struct Iter<'a, K, V>(hash_map::Iter<'a, K, V>);
impl<K, V> Clone for Iter<'_, K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
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

/// An iterator over the *uncommon* entries in a [TotalHashMap] that allows mutation of the values.
///
/// This iterator is created by [TotalHashMap::iter_mut].
pub struct IterMut<'a, K, V, C: Commonality<V> = DefaultCommonality> {
    // Always Some except while dropping
    inner: Option<hash_map::IterMut<'a, K, V>>,
    // Used while dropping to restore invariant. Must be a raw pointer because the other field
    // borrows from it mutably
    map: *mut TotalHashMap<K, V, C>,
}
impl<'a, K, V, C: Commonality<V>> IterMut<'a, K, V, C> {
    fn new(map: &'a mut TotalHashMap<K, V, C>) -> Self {
        Self { map: map as *mut _, inner: Some(map.inner.iter_mut()) }
    }
}
impl<'a, K, V, C: Commonality<V>> Iterator for IterMut<'a, K, V, C> {
    type Item = (&'a K, &'a mut V);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.as_mut().unwrap().next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.as_ref().unwrap().size_hint()
    }
}
impl<K, V, C: Commonality<V>> ExactSizeIterator for IterMut<'_, K, V, C> {
    fn len(&self) -> usize {
        self.inner.as_ref().unwrap().len()
    }
}
impl<K, V, C: Commonality<V>> FusedIterator for IterMut<'_, K, V, C> {}
impl<K, V, C: Commonality<V>> Drop for IterMut<'_, K, V, C> {
    fn drop(&mut self) {
        // FIXME: this is unsound because the mutable references yielded by this iterator are
        // allowed to outlive the iterator
        let _ = self.inner.take().unwrap();
        let map = unsafe { &mut *self.map };
        map.inner.retain(|_, value| !C::is_common(value));
    }
}

/// An owning iterator over the *uncommon* entries in a [TotalHashMap].
///
/// This iterator is created by [TotalHashMap]'s implementation of [IntoIterator].
pub struct IntoIter<K, V>(hash_map::IntoIter<K, V>);
impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<K, V> ExactSizeIterator for IntoIter<K, V> {
    fn len(&self) -> usize {
        self.0.len()
    }
}
impl<K, V> FusedIterator for IntoIter<K, V> {}

/// A draining iterator over the *uncommon* entries in a [TotalHashMap].
///
/// This iterator is created by [TotalHashMap::drain].
pub struct Drain<'a, K, V>(hash_map::Drain<'a, K, V>);
impl<K, V> Iterator for Drain<'_, K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<K, V> ExactSizeIterator for Drain<'_, K, V> {
    fn len(&self) -> usize {
        self.0.len()
    }
}
impl<K, V> FusedIterator for Drain<'_, K, V> {}

// --------------------------------------------------------------------------
// Population from iterators

impl<K: Eq + Hash, V, C: Commonality<V>> Extend<(K, V)> for TotalHashMap<K, V, C> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}
impl<K: Eq + Hash, V, C: Commonality<V>> FromIterator<(K, V)> for TotalHashMap<K, V, C> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut this = Self::default();
        this.extend(iter);
        this
    }
}

// --------------------------------------------------------------------------
// Miscellaneous traits

impl<K: Eq + Hash, V: PartialEq, C> PartialEq for TotalHashMap<K, V, C> {
    fn eq(&self, other: &Self) -> bool {
        // Although both self.common and other.common should have the same value (namely,
        // C::common()), we still need to compare them because V's PartialEq impl might not be
        // reflexive
        self.common == other.common && self.inner == other.inner
    }
}
impl<K: Eq + Hash, V: Eq, C> Eq for TotalHashMap<K, V, C> {}

impl<K: Debug, V: Debug, C> Debug for TotalHashMap<K, V, C> {
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

// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iter_mut_drop() {
        let mut m = TotalHashMap::<i32, i32>::new();
        m.iter_mut();
    }

    #[test]
    #[ignore = "mutating iterators are currently broken/unsound"]
    fn iter_mut_use_after_drop() {
        // FIXME: undefined behavior that is caught with Miri
        let mut m = TotalHashMap::<i32, i32>::new();
        m.insert(1, 2);
        let mut it = m.iter_mut();
        let (_, val) = it.next().unwrap();
        *val = 3;
        drop(it);
        *val = 4;
    }
}
