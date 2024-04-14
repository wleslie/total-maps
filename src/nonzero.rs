//! Maps that only store entries with nonzero values.
//!
//! These types are recommended whenever it is important that the common value is zero, even though
//! most numeric types' [Default] implementations also yield a zero value.

use num_traits::Zero;

use crate::{Commonality, TotalBTreeMap, TotalHashMap};

/// A hash map that only stores entries with non-zero values. All other keys are presumed to be
/// associated with the zero value.
pub type NonZeroHashMap<K, V> = TotalHashMap<K, V, ZeroCommonality>;

/// An ordered map that only stores entries with non-zero values. All other keys are presumed to be
/// associated with the zero value.
pub type NonZeroBTreeMap<K, V> = TotalBTreeMap<K, V, ZeroCommonality>;

/// A [commonality](Commonality) based on the [Zero] trait.
///
/// A [TotalHashMap] or [TotalBTreeMap] using this commonality only stores entries with nonzero
/// values.
pub struct ZeroCommonality(());

impl<T: Zero> Commonality<T> for ZeroCommonality {
    fn common() -> T {
        T::zero()
    }
    fn is_common(value: &T) -> bool {
        value.is_zero()
    }
}
