## rm -rf  [![Build Status](https://travis-ci.org/vn971/rm_rf.svg?branch=master)](https://travis-ci.org/vn971/rm_rf)  [![crates.io](https://img.shields.io/crates/v/rm_rf.svg)](https://crates.io/crates/rm_rf)

Force-remove a file/directory and all descendants.

In contrast to `std::fs::remove_dir_all`, it will remove
empty directories that lack read access on Linux,
and will remove "read-only" files and directories on Windows.


## Usage

```rust
rm_rf::remove("target")?; // remove, fail if target doesn't exists
rm_rf::ensure_removed("target")?; // remove, but ignore if target doesn't exist
```

Note: to avoid stack overflow for deeply nested directories, this library uses [stacker](https://crates.io/crates/stacker).

## Other

Licensed as (at your choice): MIT, Apache2 and CC0 ("public domain").
