use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_document_store_migration::{
    AUTHORITATIVE_DOCUMENT_MIGRATION_MARKER, LEGACY_DOCUMENT_POINTER_ROOT,
    LEGACY_DOCUMENT_VERSION_ROOT, LocalDocumentStoreMigration, LocalDocumentStoreMigrationError,
    LocalDocumentStoreMigrationOutcome,
};
use cabinet_adapters::local_version_store::VERSION_ENTRY_FILE;
use cabinet_desktop_shell::{
    DesktopDocumentAuthoringRequestDto, DesktopDocumentAuthoringRuntime,
    DesktopDocumentMutationRequestDto, DesktopDocumentMutationRuntime,
    DesktopDocumentQueryRequestDto, DesktopDocumentQueryRuntime, DesktopProjectionRuntime,
};
use cabinet_domain::document::DocumentBodyPolicy;

#[test]
fn clean_profile_initializes_authoritative_document_store_targets() {
    let temp = TempRoot::new("clean");
    let migration = migration(&temp);

    let outcome = migration.execute().expect("clean migration result");

    assert_eq!(outcome, LocalDocumentStoreMigrationOutcome::NoLegacyData);
    assert!(temp.path.join("document-versions").is_dir());
    assert!(temp.path.join("document-current-pointers").is_dir());
}

#[test]
fn clean_startup_projection_reads_revisions_created_after_runtime_construction() {
    let temp = TempRoot::new("clean-projection-runtime");
    migration(&temp).execute().expect("clean initialization");
    let projection =
        DesktopProjectionRuntime::new(temp.path.clone(), 4096, 20, 3).expect("projection runtime");
    let mutation =
        DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096).expect("mutation runtime");

    let created = mutation.execute(DesktopDocumentMutationRequestDto::Create {
        operation_id: "clean-operation".into(),
        workspace_id: "workspace-1".into(),
        document_id: "clean-doc".into(),
        body: "Clean profile title\nSearchable body".into(),
        author: "local-user".into(),
        summary: "Create".into(),
    });
    let projected = projection.run_once();

    assert!(created.ok);
    assert!(projected.ok);
    assert_eq!(projected.ready_count, 3);
    assert_eq!(projected.failed_count, 0);
    assert_eq!(
        projection
            .get_freshness("workspace-1", "clean-doc")
            .state
            .as_deref(),
        Some("ready")
    );
}

#[test]
fn migrates_legacy_history_and_pointer_for_authoritative_query_without_changing_source() {
    let temp = TempRoot::new("migrate");
    create_legacy_document(&temp);
    let source_before = tree_snapshot(&temp.path.join(LEGACY_DOCUMENT_VERSION_ROOT));
    let pointer_source_before = tree_snapshot(&temp.path.join(LEGACY_DOCUMENT_POINTER_ROOT));
    let migration = migration(&temp);

    let outcome = migration.execute().expect("migration succeeds");

    assert_eq!(outcome, LocalDocumentStoreMigrationOutcome::Migrated);
    assert_eq!(
        tree_snapshot(&temp.path.join(LEGACY_DOCUMENT_VERSION_ROOT)),
        source_before
    );
    assert_eq!(
        tree_snapshot(&temp.path.join(LEGACY_DOCUMENT_POINTER_ROOT)),
        pointer_source_before
    );
    assert!(
        temp.path
            .join(AUTHORITATIVE_DOCUMENT_MIGRATION_MARKER)
            .exists()
    );
    let query = DesktopDocumentQueryRuntime::new(temp.path.clone(), 4096).unwrap();
    let current = query.execute(DesktopDocumentQueryRequestDto::Current {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
    });
    assert!(current.ok);
    assert_eq!(
        current.data.unwrap().body.as_deref(),
        Some("기존 제목\n기존 본문")
    );
    let history = query.execute(DesktopDocumentQueryRequestDto::History {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        cursor: None,
        limit: 10,
    });
    assert_eq!(history.data.unwrap().entries.len(), 1);

    let target_before = tree_snapshot(&temp.path.join("document-versions"));
    assert_eq!(
        migration.execute().unwrap(),
        LocalDocumentStoreMigrationOutcome::AlreadyMigrated
    );
    assert_eq!(
        tree_snapshot(&temp.path.join("document-versions")),
        target_before
    );
}

#[test]
fn resumes_pointer_publish_after_version_only_partial_state() {
    let temp = TempRoot::new("partial");
    create_legacy_document(&temp);
    let migration = migration(&temp);
    assert_eq!(
        migration.execute().unwrap(),
        LocalDocumentStoreMigrationOutcome::Migrated
    );
    fs::remove_dir_all(temp.path.join("document-current-pointers")).unwrap();
    fs::remove_file(temp.path.join(AUTHORITATIVE_DOCUMENT_MIGRATION_MARKER)).unwrap();

    let resumed = migration.execute().expect("partial migration resumes");

    assert_eq!(resumed, LocalDocumentStoreMigrationOutcome::Migrated);
    let query = DesktopDocumentQueryRuntime::new(temp.path.clone(), 4096).unwrap();
    assert!(
        query
            .execute(DesktopDocumentQueryRequestDto::Current {
                workspace_id: "workspace-1".into(),
                document_id: "doc-1".into(),
            })
            .ok
    );
}

#[test]
fn rejects_conflicting_target_and_corrupt_or_symlinked_legacy_without_overwrite() {
    let conflict_root = TempRoot::new("conflict");
    create_legacy_document(&conflict_root);
    let authoritative =
        DesktopDocumentMutationRuntime::new(conflict_root.path.clone(), 4096).unwrap();
    assert!(
        authoritative
            .execute(DesktopDocumentMutationRequestDto::Create {
                operation_id: "new-operation".into(),
                workspace_id: "workspace-1".into(),
                document_id: "new-doc".into(),
                body: "새 저장소\n본문".into(),
                author: "local-user".into(),
                summary: "Create".into(),
            })
            .ok
    );
    let target_before = tree_snapshot(&conflict_root.path.join("document-versions"));

    let conflict = migration(&conflict_root).execute().unwrap_err();

    assert_eq!(conflict, LocalDocumentStoreMigrationError::Conflict);
    assert_eq!(
        tree_snapshot(&conflict_root.path.join("document-versions")),
        target_before
    );

    let corrupt_root = TempRoot::new("corrupt");
    create_legacy_document(&corrupt_root);
    corrupt_named_files(
        &corrupt_root.path.join(LEGACY_DOCUMENT_VERSION_ROOT),
        VERSION_ENTRY_FILE,
    );
    assert_eq!(
        migration(&corrupt_root).execute().unwrap_err(),
        LocalDocumentStoreMigrationError::CorruptedLegacy
    );
    assert!(!corrupt_root.path.join("document-versions").exists());

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let symlink_root = TempRoot::new("symlink");
        create_legacy_document(&symlink_root);
        symlink(
            symlink_root.path.join(LEGACY_DOCUMENT_VERSION_ROOT),
            symlink_root
                .path
                .join(LEGACY_DOCUMENT_VERSION_ROOT)
                .join("unsafe-link"),
        )
        .unwrap();
        assert_eq!(
            migration(&symlink_root).execute().unwrap_err(),
            LocalDocumentStoreMigrationError::CorruptedLegacy
        );
    }
}

#[test]
fn desktop_startup_runs_document_store_migration_before_authoritative_runtimes() {
    let source = include_str!("../src/main.rs");
    let migration = source.find("LocalDocumentStoreMigration::new").unwrap();
    let mutation = source.find("DesktopDocumentMutationRuntime::new").unwrap();
    let query = source.find("DesktopDocumentQueryRuntime::new").unwrap();
    assert!(migration < mutation);
    assert!(migration < query);
}

fn migration(temp: &TempRoot) -> LocalDocumentStoreMigration {
    LocalDocumentStoreMigration::new(temp.path.clone(), DocumentBodyPolicy::new(4096).unwrap())
}

fn create_legacy_document(temp: &TempRoot) {
    let runtime = DesktopDocumentAuthoringRuntime::new(temp.path.clone(), 4096).unwrap();
    let response = runtime.execute(DesktopDocumentAuthoringRequestDto::Create {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        path: "notes/legacy.md".into(),
        body: "기존 제목\n기존 본문".into(),
        version_id: "legacy-v1".into(),
        snapshot_ref: "snapshot:legacy-v1".into(),
        author: "local-user".into(),
        summary: "Create".into(),
    });
    assert!(response.ok);
    assert!(temp.path.join(LEGACY_DOCUMENT_POINTER_ROOT).exists());
}

fn tree_snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut output = Vec::new();
    collect_tree(root, root, &mut output);
    output.sort_by(|left, right| left.0.cmp(&right.0));
    output
}

fn collect_tree(root: &Path, current: &Path, output: &mut Vec<(PathBuf, Vec<u8>)>) {
    for entry in fs::read_dir(current).expect("read tree") {
        let path = entry.expect("tree entry").path();
        if path.is_dir() {
            collect_tree(root, &path, output);
        } else {
            output.push((
                path.strip_prefix(root).unwrap().to_path_buf(),
                fs::read(path).expect("read file"),
            ));
        }
    }
}

fn corrupt_named_files(root: &Path, file_name: &str) {
    for entry in fs::read_dir(root).expect("read legacy") {
        let path = entry.expect("legacy entry").path();
        if path.is_dir() {
            corrupt_named_files(&path, file_name);
        } else if path.file_name().and_then(|value| value.to_str()) == Some(file_name) {
            fs::write(path, "corrupted\n").unwrap();
        }
    }
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cabinet-document-migration-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
