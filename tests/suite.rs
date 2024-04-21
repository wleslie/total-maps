use itertools::Itertools;
use total_maps::{Commonality, TotalBTreeMap, TotalHashMap};

macro_rules! common {
    (mod $mod:ident, $Map:ident, $iter_eq:expr $(,)?) => {
        mod $mod {
            use super::*;

            #[test]
            fn populate_basic() {
                let mut m = $Map::<_, _>::new();
                assert_eq!(m.insert("foo", "v_foo"), "");
                assert_eq!(m.insert("baz", ""), "");
                assert_eq!(m.insert("bar", "v_bar"), "");
                assert_eq!(m.insert("foo", "v_foo_2"), "v_foo");

                assert!(!m.is_empty());
                assert_eq!(m.len(), 2);
                assert!($iter_eq(m.keys(), [&"bar", &"foo"]));
                assert!($iter_eq(m.values(), [&"v_bar", &"v_foo_2"]));
                assert!($iter_eq(m.iter(), [(&"bar", &"v_bar"), (&"foo", &"v_foo_2")]));
                assert!($iter_eq(m.clone().into_keys(), ["bar", "foo"]));
                assert!($iter_eq(m.clone().into_values(), ["v_bar", "v_foo_2"]));
                assert!($iter_eq(m, [("bar", "v_bar"), ("foo", "v_foo_2")]));
            }

            #[test]
            fn populate_common_only() {
                let mut m = $Map::<_, _>::new();
                assert_eq!(m.insert("foo", ""), "");
                assert_eq!(m.insert("bar", ""), "");
                assert!(m.is_empty());
                assert_eq!(m.len(), 0);
                assert!(m.into_iter().next().is_none());
            }

            #[test]
            fn removal() {
                let mut m = $Map::<_, _>::new();

                assert_eq!(m.insert("foo", "bar"), "");
                assert_eq!(m.insert("baz", "quux"), "");
                assert_eq!(m.len(), 2);
                assert!($iter_eq(m.iter(), [(&"baz", &"quux"), (&"foo", &"bar")]));

                assert_eq!(m.remove(&"foo"), "bar");
                assert_eq!(m.remove(&"xyzzy"), "");
                assert_eq!(m.len(), 1);
                assert!($iter_eq(m.iter(), [(&"baz", &"quux")]));

                m.clear();
                assert_eq!(m.len(), 0);
                assert!(m.into_iter().next().is_none());
            }

            #[test]
            fn access() {
                let mut m = $Map::<_, _>::new();
                assert_eq!(m.insert("foo", "bar"), "");
                assert_eq!(m.insert("baz", ""), "");

                assert!(m.contains_key(&"foo"));
                assert_eq!(m.get(&"foo"), &"bar");
                assert_eq!(m[&"foo"], "bar");

                assert!(!m.contains_key(&"baz"));
                assert_eq!(m.get(&"baz"), &"");
                assert_eq!(m[&"baz"], "");

                assert!(!m.contains_key(&"quux"));
                assert_eq!(m.get(&"quux"), &"");
                assert_eq!(m[&"quux"], "");
            }

            #[test]
            fn entry_mut() {
                let mut m = $Map::<_, _>::new();

                let entry = m.entry("foo");
                assert_eq!(*entry, "");
                drop(entry);
                assert!(!m.contains_key(&"foo"));

                let mut entry = m.entry("foo");
                assert_eq!(*entry, "");
                *entry = "bar";
                drop(entry);
                assert_eq!(m.get(&"foo"), &"bar");

                let mut entry = m.entry("foo");
                assert_eq!(*entry, "bar");
                *entry = "baz";
                drop(entry);
                assert_eq!(m.get(&"foo"), &"baz");

                let mut entry = m.entry("foo");
                assert_eq!(*entry, "baz");
                *entry = "";
                drop(entry);
                assert!(!m.contains_key(&"foo"));
            }

            #[test]
            #[ignore = "mutating iterators are currently broken/unsound"]
            fn values_mut() {
                let mut m = $Map::<_, _>::new();
                assert_eq!(m.insert("foo", "bar"), "");
                assert_eq!(m.insert("baz", "quux"), "");

                let it = m.values_mut();
                let mut values = it.collect::<Vec<_>>();
                // FIXME: holding on to the mutable value references after dropping the iterator
                // makes it possible to break the map invariant. ValuesMut probably needs to be made
                // into a "streaming iterator"
                values.sort();
                assert_eq!(values.len(), 2);
                assert_eq!(values[0], &mut "bar");
                *values[0] = "bar2";
                assert_eq!(values[1], &mut "quux");
                *values[1] = "";

                assert_eq!(m.len(), 1);
                assert!($iter_eq(m.iter(), [(&"foo", &"bar2")]));
            }

            #[test]
            fn from_iter() {
                let elems =
                    [("foo", "bar"), ("baz", "quux"), ("xyzzy", ""), ("foo", "bar2"), ("baz", "")];

                let m = elems.into_iter().collect::<$Map<_, _>>();
                assert_eq!(m.len(), 1);
                assert!($iter_eq(m.iter(), [(&"foo", &"bar2")]));

                let mut m = $Map::<_, _>::new();
                m.insert("xyzzy", "plugh");
                m.insert("monkey", "banana");
                m.extend(elems);
                assert_eq!(m.len(), 2);
                assert!($iter_eq(m.iter(), [(&"foo", &"bar2"), (&"monkey", &"banana")]));
            }

            #[test]
            fn map_eq() {
                assert_eq!(
                    [("foo", "bar"), ("baz", "quux")].into_iter().collect::<$Map<_, _>>(),
                    [("baz", "quux"), ("foo", "bar")].into_iter().collect::<$Map<_, _>>()
                );

                let nan_map = $Map::<&str, _, NaNCommonality>::new();
                assert_ne!(nan_map, nan_map);
            }
        }
    };
}

common!(mod btree_map, TotalBTreeMap, iter_eq);
common!(mod hash_map, TotalHashMap, unordered_iter_eq);

fn iter_eq<I, J>(lhs: I, rhs: J) -> bool
where
    I: IntoIterator,
    J: IntoIterator<Item = I::Item>,
    I::Item: PartialEq,
{
    lhs.into_iter().eq(rhs)
}

fn unordered_iter_eq<I, J>(lhs: I, rhs: J) -> bool
where
    I: IntoIterator,
    J: IntoIterator<Item = I::Item>,
    I::Item: Ord,
{
    lhs.into_iter().sorted().eq(rhs)
}

struct NaNCommonality;
impl Commonality<f64> for NaNCommonality {
    fn common() -> f64 {
        f64::NAN
    }
    fn is_common(value: &f64) -> bool {
        value.is_nan()
    }
}
