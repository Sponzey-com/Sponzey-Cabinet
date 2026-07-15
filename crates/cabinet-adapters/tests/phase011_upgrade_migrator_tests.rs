use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::phase011_upgrade_migrator::{
    Phase011UpgradeError, Phase011UpgradeMigrator, Phase011UpgradeOutcome, Phase011UpgradePolicy,
};

#[test]
fn copied_upgrade_preserves_source_and_is_idempotent() {
    let fixture = Fixture::new("idempotent");
    fixture.seed("schema\t1\ncanvas");
    let before = fingerprint(&fixture.source);
    let migrator = fixture.migrator();

    let first = migrator
        .migrate(&fixture.source, &fixture.destination)
        .expect("first migration");
    let destination_before = fingerprint(&fixture.destination);
    let second = migrator
        .migrate(&fixture.source, &fixture.destination)
        .expect("idempotent migration");

    assert_eq!(first, Phase011UpgradeOutcome::Migrated);
    assert_eq!(second, Phase011UpgradeOutcome::AlreadyCurrent);
    assert_eq!(fingerprint(&fixture.source), before);
    assert_eq!(fingerprint(&fixture.destination), destination_before);
    assert_eq!(
        fs::read_to_string(fixture.destination.join("phase-upgrade.tsv")).expect("marker"),
        "schema\t1\nsource_phase\t11\ntarget_phase\t12\n"
    );
}

#[test]
fn copied_upgrade_rejects_future_store_schema_without_publishing_destination() {
    let fixture = Fixture::new("future-schema");
    fixture.seed("schema\t99\ncanvas");

    assert_eq!(
        fixture
            .migrator()
            .migrate(&fixture.source, &fixture.destination),
        Err(Phase011UpgradeError::UnsupportedFutureSchema)
    );
    assert!(!fixture.destination.exists());
    assert!(!fixture.preparing().exists());
}

#[cfg(unix)]
#[test]
fn copied_upgrade_rejects_source_symlink() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new("symlink");
    fixture.seed("schema\t1\ncanvas");
    let outside = fixture.root.join("outside.txt");
    fs::write(&outside, b"outside").expect("outside");
    symlink(&outside, fixture.source.join("assets/unsafe-link")).expect("symlink");

    assert_eq!(
        fixture
            .migrator()
            .migrate(&fixture.source, &fixture.destination),
        Err(Phase011UpgradeError::UnsafeSource)
    );
    assert!(!fixture.destination.exists());
}

#[test]
fn copied_upgrade_does_not_overwrite_destination_with_corrupt_marker() {
    let fixture = Fixture::new("corrupt-marker");
    fixture.seed("schema\t1\ncanvas");
    fs::create_dir_all(&fixture.destination).expect("destination");
    fs::write(fixture.destination.join("phase-upgrade.tsv"), b"broken").expect("marker");

    assert_eq!(
        fixture
            .migrator()
            .migrate(&fixture.source, &fixture.destination),
        Err(Phase011UpgradeError::CorruptedDestination)
    );
    assert_eq!(
        fs::read(fixture.destination.join("phase-upgrade.tsv")).expect("unchanged"),
        b"broken"
    );
}

struct Fixture {
    root: PathBuf,
    source: PathBuf,
    destination: PathBuf,
}
impl Fixture {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cabinet-phase011-upgrade-{name}-{}-{nonce}",
            std::process::id()
        ));
        let source = root.join("phase011-source");
        let destination = root.join("phase012-working");
        fs::create_dir_all(&source).expect("source");
        Self {
            root,
            source,
            destination,
        }
    }
    fn seed(&self, canvas: &str) {
        for (relative, content) in [
            ("authoring-current/workspace/document.md", "document"),
            ("authoring-versions/workspace/version.md", "version"),
            ("canvases/workspace/current.canvas", canvas),
            ("assets/metadata/workspace/item.asset", "schema\t1\nasset"),
            ("assets/objects/workspace/item.bin", "bytes"),
            ("assets/associations/workspace/item.link", "schema\t1\nlink"),
        ] {
            let path = self.source.join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("dir");
            fs::write(path, content).expect("fixture");
        }
    }
    fn migrator(&self) -> Phase011UpgradeMigrator {
        Phase011UpgradeMigrator::new(Phase011UpgradePolicy::new(100, 1024 * 1024).expect("policy"))
    }
    fn preparing(&self) -> PathBuf {
        self.root.join(".phase012-working.preparing")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn fingerprint(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn collect(root: &Path, base: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(root).expect("read") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                collect(&path, base, out);
            } else {
                out.push((
                    path.strip_prefix(base)
                        .expect("relative")
                        .to_string_lossy()
                        .into(),
                    fs::read(path).expect("file"),
                ));
            }
        }
    }
    let mut values = Vec::new();
    collect(root, root, &mut values);
    values.sort_by(|a, b| a.0.cmp(&b.0));
    values
}
