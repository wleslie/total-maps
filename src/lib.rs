//! Maps where every possible key has an associated value.
//!
//! Only entries with *uncommon* values are actually stored in the map; all other keys are presumed
//! to be associated with a *common* value. The definition of "common" and "uncommon" is determined
//! by the map's optional [Commonality] type parameter; if unspecified, the map will use
//! [DefaultCommonality], which uses the standard [Default] trait to provide the common value.
//!
//! [TotalHashMap] is the main data structure provided by this crate.

use std::{
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
};

#[cfg(feature = "num-traits")]
pub use self::nonzero::{NonZeroBTreeMap, NonZeroHashMap, ZeroCommonality};
pub use self::{btree_map::TotalBTreeMap, hash_map::TotalHashMap};

pub mod btree_map;
pub mod hash_map;
#[cfg(feature = "num-traits")]
pub mod nonzero;

// --------------------------------------------------------------------------

/// Defines a notion of "common" vs. "uncommon" values for the type `V`, used to determine which
/// entries are stored in a [TotalHashMap] or [TotalBTreeMap].
///
/// There could be multiple definitions of commonality for the same type. The basic implementation,
/// [DefaultCommonality], is based on the [Default] trait.
#[cfg_attr(
    feature = "num-traits",
    doc = "Likewise, [ZeroCommonality] is based on the [num_traits::Zero] trait."
)]
pub trait Commonality<V> {
    /// The common value of type `V`.
    fn common() -> V;

    /// Returns true if `value` is the common value of type `V`. `Self::is_common(Self::common())`
    /// must be true.
    fn is_common(value: &V) -> bool;
}

/// A [commonality](Commonality) based on the [Default] trait.
///
/// *Important:* This type's implementation of [Commonality] is valid only if `T::default() ==
/// T::default()`. Any type with a valid [Eq] implementation satisfies this requirement, but many
/// types like `f64` and `Vec<f64>` also satisfy this requirement, despite not implementing [Eq].
///
/// A [TotalHashMap] or [TotalBTreeMap] using this commonality only stores entries with non-default
/// values.
pub struct DefaultCommonality(());
impl<T: PartialEq + Default> Commonality<T> for DefaultCommonality {
    // The bound on T is PartialEq (instead of Eq) to allow non-Eq types like f64 and Vec<f64>. The
    // unchecked reflexivity requirement specified in the docs is gross, but unlikely to be violated
    // in normal usage.
    //
    // A more principled way to handle this requirement would be to define a marker trait
    // `DefaultEq: PartialEq + Default` and specify the requirement as a "law" of the trait, and
    // then insist that users implement this trait only when the law holds (similar to how Eq itself
    // is a marker trait with its own reflexivity law). But implementing this marker trait would be
    // an annoyance in the best case, and impossible (due to the orphan rule) in the worst case.

    fn common() -> T {
        T::default()
    }
    fn is_common(value: &T) -> bool {
        value == &T::default()
    }
}

struct PhantomPtr<T>(PhantomData<*const T>);
impl<T> Default for PhantomPtr<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
impl<T> Debug for PhantomPtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PhantomPtr").field(&self.0).finish()
    }
}
unsafe impl<T> Send for PhantomPtr<T> {}
unsafe impl<T> Sync for PhantomPtr<T> {}
