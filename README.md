## rm -rf

Force-removes a file/directory and all descendants.

In contrast to `std::fs::remove_dir_all`, it will remove
empty directories that lack read access on Linux,
and will remove "read-only" files and directories on Windows.


## Other

Licensed as (at your choice): MIT, Apache2 and CC0 ("public domain").
