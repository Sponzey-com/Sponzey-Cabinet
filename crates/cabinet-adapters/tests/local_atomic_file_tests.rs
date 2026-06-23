use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_atomic_file::{
    AtomicWriteError, AtomicWriteState, atomic_temp_path, recover_stale_temp,
    write_bytes_atomically,
};

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(test_name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = PathBuf::from("/tmp").join(format!(
            "sponzey-cabinet-atomic-{test_name}-{}-{nanos}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temp root");
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn atomic_write_writes_bytes_and_reports_completed_state() {
    let temp = TempRoot::new("complete");
    let target = temp.path.join("nested").join("body.md");

    let outcome = write_bytes_atomically(&target, b"hello").expect("atomic write");

    assert_eq!(outcome.final_state(), AtomicWriteState::Completed);
    assert_eq!(fs::read(&target).expect("target bytes"), b"hello");
    assert!(!atomic_temp_path(&target).expect("temp path").exists());
}

#[test]
fn atomic_recovery_removes_stale_temp_without_touching_target() {
    let temp = TempRoot::new("recover");
    let target = temp.path.join("metadata.txt");
    fs::write(&target, b"stable").expect("target");
    let temp_path = atomic_temp_path(&target).expect("temp path");
    fs::write(&temp_path, b"stale").expect("stale temp");

    let outcome = recover_stale_temp(&target).expect("recover");

    assert_eq!(outcome.final_state(), AtomicWriteState::Completed);
    assert!(outcome.removed_temp());
    assert_eq!(fs::read(&target).expect("target"), b"stable");
    assert!(!temp_path.exists());
}

#[test]
fn atomic_write_reports_prepare_failure_when_parent_path_is_file() {
    let temp = TempRoot::new("prepare-fail");
    let parent_file = temp.path.join("not-a-directory");
    fs::write(&parent_file, b"file").expect("parent file");
    let target = parent_file.join("body.md");

    let error = write_bytes_atomically(&target, b"hello").expect_err("write must fail");

    assert_eq!(error, AtomicWriteError::PrepareFailed);
    assert_eq!(error.failed_state(), AtomicWriteState::Failed);
}
