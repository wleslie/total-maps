//! Maps where every possible key has an associated value.
//!
//! Only entries with *uncommon* values are actually stored in the map; all other keys are presumed
//! to be associated with a *common* value. The definition of "common" and "uncommon" is determined
//! by the map's optional [Commonality] type parameter; if unspecified, the map will use
//! [DefaultCommonality], which uses the standard [Default] trait to provide the common value.
//!
//! [TotalHashMap] is the main data structure provided by this crate.

use num_traits::Zero;

pub use self::hash_map::{NonZeroHashMap, TotalHashMap};

pub mod hash_map;

// --------------------------------------------------------------------------

/// Defines a notion of "common" vs. "uncommon" values for the type `V`, used to determine which
/// entries are stored in a [TotalHashMap].
///
/// There could be multiple definitions of commonality for the same type. Two standard
/// implementations are provided: [DefaultCommonality] (based on the [Default] trait) and
/// [ZeroCommonality] (based on the [Zero] trait).
pub trait Commonality<V> {
    /// The common value of type `V`.
    fn common() -> V;

    /// Returns true if `value` is the common value of type `V`. `Self::is_common(Self::common())`
    /// must be true.
    ///
    /// If `V` implements [PartialEq], then this function should be consistent with it. That is to
    /// say, `Self::is_common(x) && x == y` should imply `Self::is_common(y)`. Furthermore, if `V`
    /// implements [Eq], then `Self::is_common(x) && Self::is_common(y)` should imply `x == y`.
    fn is_common(value: &V) -> bool;
}

/// A [commonality](Commonality) based on the [Default] trait.
///
/// A [TotalHashMap] using this commonality only stores entries with non-default values.
pub struct DefaultCommonality(());
impl<T: Eq + Default> Commonality<T> for DefaultCommonality {
    fn common() -> T {
        T::default()
    }
    fn is_common(value: &T) -> bool {
        value == &T::default()
    }
}

/// A [commonality](Commonality) based on the [Zero] trait.
///
/// A [TotalHashMap] using this commonality only stores entries with nonzero values.
pub struct ZeroCommonality(());
impl<T: Zero> Commonality<T> for ZeroCommonality {
    fn common() -> T {
        T::zero()
    }
    fn is_common(value: &T) -> bool {
        value.is_zero()
    }
}
