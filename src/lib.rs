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
pub fn force_remove_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref();
    match path.metadata() {
        Ok(_) => force_remove_all_fail_if_not_exist(path),
        Err(err) => match err.kind() {
            ErrorKind::NotFound => Ok(()),
            _ => Err(err),
        },
    }
}

/// same as `force_remove_all` above, but fail if the original destination does not exist
pub fn force_remove_all_fail_if_not_exist(path: &Path) -> io::Result<()> {
    fix_permissions(path)?;
    if path.is_file() {
        fs::remove_file(path)
    } else if fs::remove_dir(path).is_ok() {
        Ok(())
    } else {
        for child in fs::read_dir(&path)? {
            let child = child?;
            let path = child.path();
            stacker::maybe_grow(4 * 1024, 16 * 1024, ||
        // don't die with stack overflow for deeply nested directories
        force_remove_all_fail_if_not_exist(&path))?;
        }
        fs::remove_dir(path)
    }
}

#[cfg(target_os = "windows")]
fn fix_permissions(path: &Path) -> io::Result<()> {
    let mut permissions = fs::metadata(&path)?.permissions();
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
    use std::process::Command;

    #[test]
    fn behavior_test() {
        test_eq_behavior("touch target");
        test_eq_behavior("touch target; chmod 000 target");
        test_eq_behavior("touch target; chmod 777 target");

        test_eq_behavior("mkdir target");
        test_eq_behavior("mkdir target; chmod 000 target");
        test_eq_behavior("mkdir target; chmod 777 target");

        test_eq_behavior("mkdir -p target/subdir; chmod 000 target");
        test_eq_behavior("mkdir -p target/subdir; chmod 777 target");
        test_eq_behavior("mkdir -p target/subdir; chmod 444 target");
        test_eq_behavior("mkdir -p target/subdir; chmod 222 target");
        test_eq_behavior("mkdir -p target/subdir; chmod 111 target");

        test_eq_behavior("");
    }

    fn test_eq_behavior(up: &str) {
        test_eq_behavior_fn(
            &|| {
                assert!(Command::new("sh")
                    .arg("-c")
                    .arg(up)
                    .status()
                    .unwrap()
                    .success());
            },
            up,
        );
    }

    fn test_eq_behavior_fn<F>(up: &F, test_name: &str)
    where
        F: Fn() -> (),
    {
        std::env::set_current_dir("target").unwrap();
        clean();
        up();
        let rm_success = rm_rf_success();
        clean();
        up();
        let rust_success = rust_remove_success();
        clean();
        eprintln!(
            "Running test {}. `rm -rf` success: {}, `force_remove` success: {}",
            test_name, rm_success, rust_success
        );
        assert_eq!(
            rm_success, rust_success,
            "`rm -rf` and `force_remove` behaved differently for test: {}",
            test_name
        );
        std::env::set_current_dir("..").unwrap();
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
        super::force_remove_all("target").is_ok()
    }
}
