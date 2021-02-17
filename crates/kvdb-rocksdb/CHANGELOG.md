# Changelog

The format is based on [Keep a Changelog].

[Keep a Changelog]: http://keepachangelog.com/en/1.0.0/

## [Unreleased]

## [0.9.1] - 2020-08-26
- Updated rocksdb to 0.15. [#424](https://github.com/paritytech/parity-common/pull/424)
- Set `format_version` to 5. [#395](https://github.com/paritytech/parity-common/pull/395) 

## [0.9.0] - 2020-06-24
- Updated `kvdb` to 0.7. [#402](https://github.com/paritytech/parity-common/pull/402)

## [0.8.0] - 2020-05-05
- Updated RocksDB to 6.7.3. [#379](https://github.com/paritytech/parity-common/pull/379)
### Breaking
- Updated to the new `kvdb` interface. [#313](https://github.com/paritytech/parity-common/pull/313)
- Rename and optimize prefix iteration. [#365](https://github.com/paritytech/parity-common/pull/365)
- Added Secondary Instance API. [#384](https://github.com/paritytech/parity-common/pull/384) 

## [0.7.0] - 2020-03-16
- Updated dependencies. [#361](https://github.com/paritytech/parity-common/pull/361)

## [0.6.0] - 2020-02-28
- License changed from GPL3 to dual MIT/Apache2. [#342](https://github.com/paritytech/parity-common/pull/342)
- Added `get_statistics` method and `enable_statistics` config parameter. [#347](https://github.com/paritytech/parity-common/pull/347)

## [0.5.0] - 2019-02-05
- Bump parking_lot to 0.10. [#332](https://github.com/paritytech/parity-common/pull/332)

## [0.4.2] - 2019-02-04
### Fixes
- Fixed `iter_from_prefix` being slow. [#326](https://github.com/paritytech/parity-common/pull/326)

## [0.4.1] - 2019-01-06
- Updated features and feature dependencies. [#307](https://github.com/paritytech/parity-common/pull/307)

## [0.4.0] - 2019-01-03
- Add I/O statistics for RocksDB. [#294](https://github.com/paritytech/parity-common/pull/294)
- Support querying memory footprint via `MallocSizeOf` trait. [#292](https://github.com/paritytech/parity-common/pull/292)

## [0.3.0] - 2019-12-19
- Use `get_pinned` API to save one allocation for each call to `get()`. [#274](https://github.com/paritytech/parity-common/pull/274)
- Rename `drop_column` to `remove_last_column`. [#274](https://github.com/paritytech/parity-common/pull/274)
- Rename `get_cf` to `cf`. [#274](https://github.com/paritytech/parity-common/pull/274)
- Default column support removed from the API. [#278](https://github.com/paritytech/parity-common/pull/278)
  - Column argument type changed from `Option<u32>` to `u32`
  - Migration
    - Column index `None` -> unsupported, `Some(0)` -> `0`, `Some(1)` -> `1`, etc.
    - Database must be opened with at least one column and existing DBs has to be opened with a number of columns increased by 1 to avoid having to migrate the data, e.g. before: `Some(9)`, after: `10`.
  - `DatabaseConfig::default()` defaults to 1 column
  - `Database::with_columns` still accepts `u32`, but panics if `0` is provided
  - `Database::open` panics if configuration with 0 columns is provided
- Add `num_keys(col)` to get an estimate of the number of keys in a column. [#285](https://github.com/paritytech/parity-common/pull/285)
- Remove `ElasticArray` and use the new `DBValue` (alias for `Vec<u8>`) and `DBKey` types from `kvdb`. [#282](https://github.com/paritytech/parity-common/pull/282)

## [0.2.0] - 2019-11-28
- Switched away from using [parity-rocksdb](https://crates.io/crates/parity-rocksdb) in favour of upstream [rust-rocksdb](https://crates.io/crates/rocksdb). [#257](https://github.com/paritytech/parity-common/pull/257)
- Revamped configuration handling, allowing per-column memory budgeting. [#256](https://github.com/paritytech/parity-common/pull/256)
### Dependencies
- rust-rocksdb v0.13

## [0.1.6] - 2019-10-24
- Updated to 2018 edition idioms. [#237](https://github.com/paritytech/parity-common/pull/237)
### Dependencies
- Updated dependencies. [#239](https://github.com/paritytech/parity-common/pull/239)
