# total-maps

Maps where every possible key has an associated value.

Only entries with *uncommon* values are actually stored in the map; all other keys are presumed to
be associated with a *common* value. The definition of "common" and "uncommon" can be customized via
the `Commonality` trait.

## Cargo features

- `num-traits`: provides a commonality implemented in terms of [`num_traits::Zero`].
- `serde`: provides [`serde::Serialize`] and [`serde::Deserialize`] implementations.

[`num_traits::Zero`]: https://docs.rs/num-traits/latest/num_traits/identities/trait.Zero.html
[`serde::Serialize`]: https://docs.rs/serde/latest/serde/trait.Serialize.html
[`serde::Deserialize`]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
