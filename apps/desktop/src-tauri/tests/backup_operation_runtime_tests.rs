use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use cabinet_desktop_shell::{
    DesktopBackupOperationRequestDto, DesktopBackupRecoveryRuntime,
    DesktopDocumentAuthoringRequestDto, DesktopDocumentAuthoringRuntime,
};

#[test]
fn native_operation_runtime_starts_runs_reads_and_cancels_durable_status() {
    let root = temp_root();
    seed_workspace(&root, "workspace-1");
    let runtime =
        DesktopBackupRecoveryRuntime::new(root.clone(), 10_000, 1024 * 1024 * 1024).unwrap();
    let request = DesktopBackupOperationRequestDto {
        workspace_id: "workspace-1".into(),
        operation_id: "backup-op-1".into(),
    };

    let queued = runtime.start_operation(request.clone());
    assert!(queued.ok);
    assert_eq!(queued.state, "Queued");
    assert_eq!(queued.operation_id, "backup-op-1");
    let completed = runtime.run_operation(request.clone());
    assert!(completed.ok);
    assert_eq!(completed.state, "Completed");
    assert_eq!(runtime.operation_status(request.clone()).state, "Completed");
    assert_eq!(runtime.cancel_operation(request).state, "Completed");

    let second = DesktopBackupOperationRequestDto {
        workspace_id: "workspace-1".into(),
        operation_id: "backup-op-2".into(),
    };
    assert_eq!(runtime.start_operation(second.clone()).state, "Queued");
    let cancelled = runtime.cancel_operation(second.clone());
    assert_eq!(cancelled.state, "Abandoned");
    assert_eq!(runtime.run_operation(second).state, "Abandoned");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn backup_worker_does_not_block_normal_document_reads() {
    let root = temp_root();
    seed_workspace(&root, "workspace-1");
    let authoring = DesktopDocumentAuthoringRuntime::new(root.clone(), 1024 * 1024).unwrap();
    assert!(
        authoring
            .execute(DesktopDocumentAuthoringRequestDto::Create {
                workspace_id: "workspace-1".into(),
                document_id: "doc-1".into(),
                path: "notes/readable.md".into(),
                body: "read during backup".into(),
                version_id: "v1".into(),
                snapshot_ref: "snapshot-v1".into(),
                author: "local-user".into(),
                summary: "Created".into(),
            })
            .ok
    );
    let object_root = root.join("assets/objects").join(hex("workspace-1"));
    for index in 0..128 {
        fs::write(
            object_root.join(format!("large-{index}.bin")),
            vec![index as u8; 128 * 1024],
        )
        .unwrap();
    }

    let runtime =
        DesktopBackupRecoveryRuntime::new(root.clone(), 10_000, 1024 * 1024 * 1024).unwrap();
    let request = DesktopBackupOperationRequestDto {
        workspace_id: "workspace-1".into(),
        operation_id: "backup-concurrent".into(),
    };
    assert_eq!(runtime.start_operation(request.clone()).state, "Queued");
    let worker_runtime = runtime.clone();
    let worker_request = request.clone();
    let worker = std::thread::spawn(move || worker_runtime.run_operation(worker_request));

    let mut samples = Vec::new();
    for _ in 0..30 {
        let started = Instant::now();
        let current = authoring.execute(DesktopDocumentAuthoringRequestDto::GetCurrent {
            workspace_id: "workspace-1".into(),
            document_id: "doc-1".into(),
        });
        assert!(current.ok);
        samples.push(started.elapsed().as_micros());
        let status = runtime.operation_status(request.clone());
        assert!(status.ok);
    }
    let completed = worker.join().unwrap();
    assert_eq!(completed.state, "Completed");
    samples.sort_unstable();
    let p95 = samples[(samples.len() * 95).div_ceil(100) - 1];
    assert!(p95 < 300_000, "document read p95 exceeded budget: {p95}us");
    let _ = fs::remove_dir_all(root);
}

fn seed_workspace(root: &Path, workspace: &str) {
    let encoded = hex(workspace);
    for relative in [
        "authoring-current",
        "authoring-versions",
        "canvases",
        "assets/metadata",
        "assets/objects",
        "assets/associations",
    ] {
        let directory = root.join(relative).join(&encoded);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("record.data"), relative).unwrap();
    }
}
fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn temp_root() -> PathBuf {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "cabinet-backup-runtime-op-{}-{}-{sequence}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
