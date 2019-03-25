## TimePlot  [![Build Status](https://travis-ci.org/vn971/rm_rf.svg?branch=master)](https://travis-ci.org/vn971/rm_rf)  [![crates.io](https://img.shields.io/crates/v/rm_rf.svg)](https://crates.io/crates/rm_rf)

## rm -rf

Force-removes a file/directory and all descendants.

In contrast to `std::fs::remove_dir_all`, it will remove
empty directories that lack read access on Linux,
and will remove "read-only" files and directories on Windows.


## Usage

```rust
extern crate rm_rf;

fn main() {
    // Failure may still happen, in situations identical to where `rm -rf` would fail.
    rm_rf::force_remove_all("target", true).expect("Failed to remove file/directory");
}
```


## Other

Licensed as (at your choice): MIT, Apache2 and CC0 ("public domain").
