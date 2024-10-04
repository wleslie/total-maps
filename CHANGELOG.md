In these change descriptions, `XMap` stands for both `HashMap` and `BTreeMap`. Likewise, `x_map`
stands for both `hash_map` and `btree_map`.

## 0.2.1

- Added `serde` feature, and implemented `Serialize` and `Deserialize` for `TotalXMap`.
- Added `TotalXMap::as_x_map`.

## 0.2.0

- *Breaking:* `x_map::Entry` has additional generic type parameters and bounds.
- Added `EmptyCommonality`.
- Added `TotalXMap::uncommon_entry`.

## 0.1.0

Initial release.
