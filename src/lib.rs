use std::path::Path;
use std::io;
use std::fs;

fn set_writeable(path: &Path) -> io::Result<()> {
	let mut permissions = fs::metadata(&path)?.permissions();
	permissions.set_readonly(false);
	fs::set_permissions(&path, permissions)
}


/// Force-removes a file/directory and all descendants.
///
/// In contrast to `std::fs::remove_dir_all`, it will remove
/// empty directories that lack read access on Linux,
/// and will remove "read-only" files and directories on Windows.
///
/// The current implementation may be not the most efficient one, but it should work.
pub fn force_remove_all<P: AsRef<Path>>(path: P, ignore_not_existing: bool) -> io::Result<()> {
	let path = path.as_ref();
	if ignore_not_existing && !path.exists() {
		Ok(())
	} else if !path.is_dir() {
		set_writeable(path)?;
		fs::remove_file(path)
	} else {
		set_writeable(path)?;
		if fs::remove_dir(path).is_ok() {
			Ok(())
		} else {
			for child in fs::read_dir(&path)? {
				let child = child?;
				let path = child.path();
				force_remove_all(&path, false)?;
			}
			fs::remove_dir(path)
		}
	}
}


#[cfg(not(target_os = "windows"))]  // windows may not have `rm`, `sh` and `chmod`
#[cfg(test)]
mod tests {
	use std::process::Command;

	fn rm_rf_success() -> bool {
		Command::new("rm").arg("-rf").arg("target").output().unwrap().status.success()
	}

	fn rust_remove_success() -> bool {
		super::force_remove_all("target", true).is_ok()
	}

	fn clean() {
		Command::new("sh").arg("-c").arg("chmod 777 target; rm -rf target").output().unwrap();
	}

	fn test_eq_behavior_fn<F>(up: &F, test_name: &str) where F: Fn() -> () {
		std::env::set_current_dir("target").unwrap();
		clean();
		up();
		let rm_success = rm_rf_success();
		clean();
		up();
		let rust_success = rust_remove_success();
		clean();
		eprintln!("Running test {}. `rm -rf` success: {}, `force_remove` success: {}",
			test_name, rm_success, rust_success);
		assert_eq!(rm_success, rust_success,
			"`rm -rf` and `force_remove` behaved differently for test: {}",
			test_name
		);
		std::env::set_current_dir("..").unwrap();
	}

	fn test_eq_behavior(up: &str) {
		test_eq_behavior_fn(&|| {
			assert!(Command::new("sh").arg("-c").arg(up).status().unwrap().success());
		}, up);
	}

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
}
