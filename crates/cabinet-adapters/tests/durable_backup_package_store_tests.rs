use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_backup_package_store::{
    LocalBackupPackagePolicy, LocalBackupPackageStore,
};
use cabinet_domain::backup::{BackupDataClass, BackupJobId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_package::{BackupPackageStore, BackupPackageStoreError};

const FIXED_CREATED_AT_EPOCH_MS: u64 = 1_784_064_000_000;

fn fixed_clock() -> u64 {
    FIXED_CREATED_AT_EPOCH_MS
}

#[test]
fn package_build_covers_authoritative_data_and_validates_after_restart() {
    let fixture = Fixture::new("complete");
    fixture.seed_complete_workspace();
    let source_before = fixture.source_fingerprint();
    let workspace = workspace();
    let package = package();

    let manifest = fixture
        .store()
        .build_package(&workspace, &package)
        .expect("build package");

    for data_class in BackupDataClass::ALL {
        let entry = manifest.entry(data_class).expect("required entry");
        assert_eq!(
            entry.record_count(),
            if data_class == BackupDataClass::CurrentDocuments {
                2
            } else {
                1
            },
            "{data_class:?}"
        );
        assert!(entry.byte_count() > 0);
    }
    assert!(
        package_root(&fixture)
            .join("data/current_documents/.version-pointers/doc/current.pointer")
            .is_file()
    );
    assert_eq!(fixture.source_fingerprint(), source_before);
    let package_root = fixture.package_root();
    assert!(package_root.join("manifest.tsv").is_file());
    assert!(!package_root.join("data/graph-projections").exists());
    assert!(!package_root.join("data/search-projections").exists());

    let mut restarted = fixture.store();
    let inspected = restarted
        .inspect_manifest(&workspace, &package)
        .expect("inspect after restart");
    assert_eq!(inspected, manifest);
    assert!(
        restarted
            .validate_package(&workspace, &package, &inspected)
            .expect("validate")
            .is_valid()
    );
}

#[test]
fn package_creation_time_is_injected_persisted_and_legacy_compatible() {
    let fixture = Fixture::new("creation-time");
    fixture.seed_complete_workspace();
    let mut store = fixture.store_with_clock(fixed_clock);

    let manifest = store
        .build_package(&workspace(), &package())
        .expect("build timestamped package");
    assert_eq!(
        manifest.created_at_epoch_ms(),
        Some(FIXED_CREATED_AT_EPOCH_MS)
    );
    let manifest_path = fixture.package_root().join("manifest.tsv");
    let persisted = fs::read_to_string(&manifest_path).expect("timestamped manifest");
    assert!(persisted.contains(&format!(
        "created_at_epoch_ms\t{FIXED_CREATED_AT_EPOCH_MS}\n"
    )));
    assert_eq!(
        fixture
            .store()
            .inspect_manifest(&workspace(), &package())
            .expect("inspect after restart")
            .created_at_epoch_ms(),
        Some(FIXED_CREATED_AT_EPOCH_MS)
    );

    let legacy = persisted
        .lines()
        .filter(|line| !line.starts_with("created_at_epoch_ms\t"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(&manifest_path, legacy).expect("legacy manifest");
    assert_eq!(
        fixture
            .store()
            .inspect_manifest(&workspace(), &package())
            .expect("inspect legacy manifest")
            .created_at_epoch_ms(),
        None
    );
}

#[test]
fn package_validation_detects_asset_object_tampering() {
    let fixture = Fixture::new("tamper");
    fixture.seed_complete_workspace();
    let workspace = workspace();
    let package = package();
    let mut store = fixture.store();
    let manifest = store
        .build_package(&workspace, &package)
        .expect("build package");
    let object = only_file(&fixture.package_root().join("data/asset_objects"));
    fs::write(object, b"tampered bytes").expect("tamper package object");

    let validation = store
        .validate_package(&workspace, &package, &manifest)
        .expect("validation result");

    assert!(!validation.is_valid());
    assert_eq!(
        validation.error_code(),
        Some("BACKUP_PACKAGE_CHECKSUM_MISMATCH")
    );
}

#[test]
fn package_publish_rejects_existing_identity_without_overwrite() {
    let fixture = Fixture::new("conflict");
    fixture.seed_complete_workspace();
    let mut store = fixture.store();
    store
        .build_package(&workspace(), &package())
        .expect("first package");
    let manifest_before = fs::read(fixture.package_root().join("manifest.tsv")).expect("manifest");

    assert_eq!(
        store.build_package(&workspace(), &package()),
        Err(BackupPackageStoreError::Conflict)
    );
    assert_eq!(
        fs::read(fixture.package_root().join("manifest.tsv")).expect("manifest remains"),
        manifest_before
    );
}

#[test]
fn package_discard_removes_published_package_and_is_idempotent() {
    let fixture = Fixture::new("discard");
    fixture.seed_complete_workspace();
    let mut store = fixture.store();
    store
        .build_package(&workspace(), &package())
        .expect("build package");
    assert!(fixture.package_root().is_dir());

    let mut restarted = fixture.store();
    restarted
        .discard_package(&workspace(), &package())
        .expect("discard package after restart");
    assert!(!fixture.package_root().exists());

    restarted
        .discard_package(&workspace(), &package())
        .expect("repeated discard is idempotent");
}

#[cfg(unix)]
#[test]
fn package_discard_rejects_symlink_package_root_without_following_it() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new("discard-symlink");
    let external = fixture.root.join("outside");
    fs::create_dir_all(&external).expect("external directory");
    fs::write(external.join("keep.txt"), b"keep").expect("external file");
    let package_root = fixture.package_root();
    fs::create_dir_all(package_root.parent().expect("package parent")).expect("package parent");
    symlink(&external, &package_root).expect("package symlink");

    assert_eq!(
        fixture.store().discard_package(&workspace(), &package()),
        Err(BackupPackageStoreError::CorruptedPackage)
    );
    assert_eq!(
        fs::read(external.join("keep.txt")).expect("external remains"),
        b"keep"
    );
}

#[cfg(unix)]
#[test]
fn package_build_rejects_symlink_in_authoritative_source() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new("symlink");
    fixture.seed_complete_workspace();
    let external = fixture.root.join("outside-secret.txt");
    fs::write(&external, b"must not be copied").expect("external file");
    let link = fixture
        .root
        .join("authoring-current")
        .join("workspace-1")
        .join("documents/unsafe-link");
    symlink(&external, &link).expect("source symlink");

    assert_eq!(
        fixture.store().build_package(&workspace(), &package()),
        Err(BackupPackageStoreError::CorruptedPackage)
    );
    assert!(!fixture.package_root().exists());
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cabinet-backup-package-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("fixture root");
        Self { root }
    }

    fn store(&self) -> LocalBackupPackageStore {
        LocalBackupPackageStore::new(
            self.root.clone(),
            LocalBackupPackagePolicy::new(100, 1024 * 1024).expect("policy"),
        )
    }

    fn store_with_clock(&self, clock: fn() -> u64) -> LocalBackupPackageStore {
        LocalBackupPackageStore::with_clock(
            self.root.clone(),
            LocalBackupPackagePolicy::new(100, 1024 * 1024).expect("policy"),
            clock,
        )
    }

    fn seed_complete_workspace(&self) {
        let workspace_hex = hex("workspace-1");
        self.write(
            "authoring-current/workspace-1/documents/by-id/doc/body.md",
            b"# current document",
        );
        self.write(
            &format!("document-current-pointers/{workspace_hex}/doc/current.pointer"),
            b"schema=1\nversion=7631\n",
        );
        self.write(
            "document-versions/workspace-1/documents/doc/history.txt",
            b"version-1",
        );
        self.write(
            &format!("canvases/{workspace_hex}/canvas/current.canvas"),
            b"schema\t1\ncanvas",
        );
        self.write(
            &format!("assets/metadata/{workspace_hex}/asset.asset"),
            b"schema\t1\nmetadata",
        );
        self.write(
            &format!("assets/objects/{workspace_hex}/aa/asset.bin"),
            b"asset object bytes",
        );
        self.write(
            &format!("assets/associations/{workspace_hex}/by-asset/asset/doc.link"),
            b"schema\t1\nassociation",
        );
        self.write(
            &format!("graph-projections/{workspace_hex}/doc.snapshot"),
            b"rebuildable graph projection",
        );
        self.write(
            &format!("search-projections/{workspace_hex}.snapshot"),
            b"rebuildable search projection",
        );
    }

    fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("source parent");
        fs::write(path, bytes).expect("source file");
    }

    fn package_root(&self) -> PathBuf {
        self.root
            .join("backup-packages")
            .join(hex("workspace-1"))
            .join(hex("package-1"))
    }

    fn source_fingerprint(&self) -> Vec<(String, Vec<u8>)> {
        let mut values = Vec::new();
        for root in [
            "authoring-current",
            "document-versions",
            "canvases",
            "assets",
        ] {
            collect_files(&self.root.join(root), &self.root, &mut values);
        }
        values.sort_by(|left, right| left.0.cmp(&right.0));
        values
    }
}

fn package_root(fixture: &Fixture) -> PathBuf {
    fixture.package_root()
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn collect_files(root: &Path, base: &Path, values: &mut Vec<(String, Vec<u8>)>) {
    if !root.exists() {
        return;
    }
    for entry in fs::read_dir(root).expect("read fixture") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            collect_files(&path, base, values);
        } else {
            values.push((
                path.strip_prefix(base)
                    .expect("relative")
                    .to_string_lossy()
                    .to_string(),
                fs::read(path).expect("read source"),
            ));
        }
    }
}

fn only_file(root: &Path) -> PathBuf {
    let mut files = Vec::new();
    collect_paths(root, &mut files);
    assert_eq!(files.len(), 1);
    files.remove(0)
}

fn collect_paths(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).expect("read package class") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            collect_paths(&path, files);
        } else {
            files.push(path);
        }
    }
}

fn workspace() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace")
}

fn package() -> BackupJobId {
    BackupJobId::new("package-1").expect("package")
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
