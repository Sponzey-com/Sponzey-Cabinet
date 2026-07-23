use std::cell::RefCell;

use cabinet_domain::backup::{
    BackupDataClass, BackupJobId, BackupManifestEntry, BackupPackageManifest,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_catalog::{
    BackupCatalogError, BackupCatalogPage, BackupCatalogPort, BackupCatalogRecord,
};
use cabinet_usecases::backup_catalog::{
    ListBackupCatalogError, ListBackupCatalogInput, ListBackupCatalogUsecase,
};

#[test]
fn bounded_catalog_returns_safe_newest_first_summaries_and_cursor() {
    let catalog = Catalog::page(BackupCatalogPage::new(
        vec![
            record("backup-new", Some(300)),
            record("backup-old", Some(100)),
        ],
        Some("cursor-next".to_string()),
    ));

    let output = ListBackupCatalogUsecase::new()
        .execute(
            ListBackupCatalogInput::new("workspace-1", None, 20),
            &catalog,
        )
        .unwrap();

    assert_eq!(catalog.calls.borrow().as_slice(), &[(None, 20)]);
    assert_eq!(output.records().len(), 2);
    assert_eq!(output.records()[0].package_id(), "backup-new");
    assert_eq!(output.records()[0].created_at_epoch_ms(), Some(300));
    assert_eq!(output.records()[0].summary().entry_count(), 8);
    assert_eq!(output.next_cursor(), Some("cursor-next"));
    assert!(!format!("{output:?}").contains("aaaaaaaaaaaaaaaa"));
}

#[test]
fn legacy_packages_follow_timestamped_packages_deterministically() {
    let catalog = Catalog::page(BackupCatalogPage::new(
        vec![
            record("backup-new", Some(300)),
            record("backup-legacy", None),
        ],
        None,
    ));

    let output = ListBackupCatalogUsecase::new()
        .execute(
            ListBackupCatalogInput::new("workspace-1", Some("cursor-1"), 2),
            &catalog,
        )
        .unwrap();

    assert_eq!(output.records()[1].created_at_epoch_ms(), None);
    assert_eq!(
        catalog.calls.borrow().as_slice(),
        &[(Some("cursor-1".into()), 2)]
    );
}

#[test]
fn invalid_limit_and_cursor_are_rejected_before_catalog_io() {
    for input in [
        ListBackupCatalogInput::new("workspace-1", None, 0),
        ListBackupCatalogInput::new("workspace-1", None, 51),
        ListBackupCatalogInput::new("workspace-1", Some(""), 20),
        ListBackupCatalogInput::new("workspace-1", Some(&"x".repeat(257)), 20),
    ] {
        let catalog = Catalog::page(BackupCatalogPage::new(vec![], None));
        let error = ListBackupCatalogUsecase::new()
            .execute(input, &catalog)
            .unwrap_err();
        assert_eq!(error, ListBackupCatalogError::InvalidInput);
        assert!(catalog.calls.borrow().is_empty());
    }
}

#[test]
fn over_limit_bad_ordering_and_port_failure_are_fail_closed() {
    let overflow = Catalog::page(BackupCatalogPage::new(
        vec![record("backup-a", Some(300)), record("backup-b", Some(200))],
        None,
    ));
    assert_eq!(
        ListBackupCatalogUsecase::new()
            .execute(
                ListBackupCatalogInput::new("workspace-1", None, 1),
                &overflow
            )
            .unwrap_err(),
        ListBackupCatalogError::CorruptedCatalog,
    );

    let bad_order = Catalog::page(BackupCatalogPage::new(
        vec![
            record("backup-old", Some(100)),
            record("backup-new", Some(300)),
        ],
        None,
    ));
    assert_eq!(
        ListBackupCatalogUsecase::new()
            .execute(
                ListBackupCatalogInput::new("workspace-1", None, 20),
                &bad_order
            )
            .unwrap_err(),
        ListBackupCatalogError::CorruptedCatalog,
    );

    let unavailable = Catalog::failed(BackupCatalogError::StorageUnavailable);
    let error = ListBackupCatalogUsecase::new()
        .execute(
            ListBackupCatalogInput::new("workspace-1", None, 20),
            &unavailable,
        )
        .unwrap_err();
    assert_eq!(error, ListBackupCatalogError::CatalogUnavailable);
    assert!(error.retryable());
    assert_eq!(error.code(), "backup_catalog.unavailable");
}

fn record(package_id: &str, created_at_epoch_ms: Option<u64>) -> BackupCatalogRecord {
    let entries = BackupDataClass::ALL
        .into_iter()
        .map(|data_class| {
            BackupManifestEntry::new(
                data_class,
                data_class.expected_ownership(),
                1,
                10,
                &"a".repeat(64),
            )
            .unwrap()
        })
        .collect();
    let manifest = BackupPackageManifest::new(1, entries).unwrap();
    let manifest = match created_at_epoch_ms {
        Some(value) => manifest.with_created_at_epoch_ms(value).unwrap(),
        None => manifest,
    };
    BackupCatalogRecord::new(BackupJobId::new(package_id).unwrap(), manifest)
}

struct Catalog {
    result: Result<BackupCatalogPage, BackupCatalogError>,
    calls: RefCell<Vec<(Option<String>, usize)>>,
}

impl Catalog {
    fn page(page: BackupCatalogPage) -> Self {
        Self {
            result: Ok(page),
            calls: RefCell::new(vec![]),
        }
    }

    fn failed(error: BackupCatalogError) -> Self {
        Self {
            result: Err(error),
            calls: RefCell::new(vec![]),
        }
    }
}

impl BackupCatalogPort for Catalog {
    fn list_backup_packages(
        &self,
        _: &WorkspaceId,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<BackupCatalogPage, BackupCatalogError> {
        self.calls
            .borrow_mut()
            .push((cursor.map(str::to_string), limit));
        self.result.clone()
    }
}
