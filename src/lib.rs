mod error;

use crate::error::Error;
use crate::error::Result;
extern crate stacker;
use std::fs;
use std::io;
use std::io::ErrorKind;
use std::path::Path;

/// Force-removes a file/directory and all descendants.
///
/// In contrast to `std::fs::remove_dir_all`, it will remove
/// empty directories that lack read access on Linux,
/// and will remove "read-only" files and directories on Windows.
pub fn remove<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    let parent: &Path = path
        .parent()
        .ok_or_else(|| Error::InvalidTarget("Invalid path, cannot get parent".to_string()))?;
    let last_segment = path.components().last().ok_or_else(|| {
        Error::InvalidTarget("Invalid path, cannot get last file path component".to_string())
    })?;
    let last_segment_str = last_segment.as_os_str().to_str().ok_or_else(|| {
        Error::InvalidTarget("Invalid path, cannot convert last segment to string".to_string())
    })?;

    if cfg!(not(target_os = "windows")) && (last_segment_str == "." || last_segment_str == "..") {
        return Err(Error::InvalidTarget(
            "Invalid path, last path segment cannot be \".\" or \"..\"".to_string(),
        ));
    }
    let path = parent.join(last_segment);
    match path.symlink_metadata() {
        Ok(_) => recursive_remove(&path).map_err(Error::IoError),
        Err(err) => match err.kind() {
            ErrorKind::NotFound => Err(Error::NotFound),
            _ => Err(Error::IoError(err)),
        },
    }
}

/// same as `remove` above, but succeeds for non-existent target, similar to `rm -rf`.
pub fn ensure_removed<P: AsRef<Path>>(path: P) -> Result<()> {
    if let Err(err) = path.as_ref().symlink_metadata() {
        if err.kind() == ErrorKind::NotFound {
            return Ok(());
        }
    };
    remove(path)
}

fn recursive_remove(path: &Path) -> io::Result<()> {
    fix_permissions(path)?;
    let metadata = path.symlink_metadata()?;
    if !metadata.is_dir() {
        fs::remove_file(path)
    } else if fs::remove_dir(path).is_ok() {
        Ok(())
    } else {
        for child in fs::read_dir(&path)? {
            let child = child?;
            let path = child.path();
            stacker::maybe_grow(4 * 1024, 16 * 1024, ||
        // don't die with stack overflow for deeply nested directories
        recursive_remove(&path))?;
        }
        fs::remove_dir(path)
    }
}

#[cfg(target_os = "windows")]
fn fix_permissions(path: &Path) -> io::Result<()> {
    let mut permissions = fs::symlink_metadata(&path)?.permissions();
    permissions.set_readonly(false);
    fs::set_permissions(&path, permissions)
}

#[cfg(not(target_os = "windows"))]
fn fix_permissions(_: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
#[cfg(not(target_os = "windows"))] // windows may not have `rm`, `sh` and `chmod`
mod tests {
    use crate::ensure_removed;
    use crate::error::Error;
    use crate::remove;
    use std::ops::Not;
    use std::process::{Command, ExitStatus};
    use std::sync::Once;

    static INITIALIZATION: Once = Once::new();

    fn initialize() {
        INITIALIZATION.call_once(|| {
            sh_exec("mkdir -p target/testdir");
            std::env::set_current_dir("target/testdir").unwrap();
        });
    }

    #[test]
    fn remove_parent_directory_test() {
        initialize();
        sh_exec("mkdir -p parentdirtest");

        assert_invalid_target(remove("parentdirtest/.."));
        assert_invalid_target(remove(".."));

        sh_exec("rmdir parentdirtest");
    }

    #[test]
    fn remove_current_directory_test() {
        initialize();
        sh_exec("mkdir -p dotdir");

        let remove_result = remove("dotdir/.");
        assert!(
            remove_result.is_ok(),
            "in contrast to `rm -rf`, removing a path with last component being . is allowed.\
             The parent should still be computable though."
        );
        assert_invalid_target(remove(".")); // removing "." is not allowed, however.

        sh_exec("rm -rf dotdir");
    }

    fn assert_invalid_target(remove_result: Result<(), Error>) {
        match remove_result {
            Err(Error::InvalidTarget(_)) => (),
            _ => panic!(
                "removal target '..' should be considered invalid, actual result is: {:?}",
                remove_result
            ),
        };
    }

    #[test]
    fn remove_inner_symlink_test() {
        initialize();
        sh_exec("mkdir inner");
        sh_exec("touch inner/real_file");
        sh_exec("ln -s inner/real_file inner/symlink");
        assert!(remove("inner/symlink").is_ok());
        sh_exec("test -f inner/real_file");
        sh_exec("! test -e inner/symlink");
        sh_exec("rm -rf inner");
    }

    #[test]
    fn remove_outer_symlink_test() {
        initialize();
        sh_exec("mkdir -p dir1/dir2");
        sh_exec("ln -s dir1/dir2 symlink");
        assert!(remove("symlink").is_ok());
        sh_exec("! test -e symlink");
        sh_exec("test -e dir1");
        sh_exec("rm -rf dir1");
    }

    #[test]
    fn behavior_test() {
        initialize();
        test_eq_behavior("");
        test_eq_behavior("touch target");
        test_eq_behavior("touch target; chmod 000 target");
        test_eq_behavior("touch target; chmod 777 target");

        test_eq_behavior("mkdir target");
        test_eq_behavior("mkdir target; ln -s unexistent target/link");
        test_eq_behavior("mkdir target; chmod 000 target");
        test_eq_behavior("mkdir target; chmod 777 target");

        test_eq_behavior("mkdir -p target/subdir; chmod 000 target");
        test_eq_behavior("mkdir -p target/subdir; chmod 777 target");
        test_eq_behavior("mkdir -p target/subdir; chmod 444 target");
        test_eq_behavior("mkdir -p target/subdir; chmod 222 target");
        test_eq_behavior("mkdir -p target/subdir; chmod 111 target");

        test_eq_behavior("ln -s unexistent target");
        test_eq_behavior("ln -s /abc/def target");
        test_eq_behavior("ln -s / target");
    }

    fn test_eq_behavior(up: &str) {
        test_eq_behavior_fn(
            &|| {
                assert!(sh_exec_status(up).success());
            },
            up,
        );
    }

    fn sh_exec(script: &str) {
        if sh_exec_status(script).success().not() {
            panic!("Non-zero exit status of `{}`", script)
        }
    }

    fn sh_exec_status(script: &str) -> ExitStatus {
        Command::new("sh")
            .arg("-c")
            .arg(script)
            .status()
            .expect("failed to execute `sh`")
    }

    fn test_eq_behavior_fn<F>(up: &F, test_name: &str)
    where
        F: Fn() -> (),
    {
        clean();
        up();
        let rm_success = rm_rf_success();
        clean();
        up();
        let rust_success = rust_remove_success();
        clean();
        assert_eq!(
            rm_success, rust_success,
            "`rm -rf` and `force_remove` behaved differently for test: {}",
            test_name
        );
    }

    fn clean() {
        Command::new("sh")
            .arg("-c")
            .arg("chmod 777 target; rm -rf target")
            .output()
            .unwrap();
    }

    fn rm_rf_success() -> bool {
        Command::new("rm")
            .arg("-rf")
            .arg("target")
            .output()
            .unwrap()
            .status
            .success()
    }

    fn rust_remove_success() -> bool {
        ensure_removed("target").is_ok()
    }
}
