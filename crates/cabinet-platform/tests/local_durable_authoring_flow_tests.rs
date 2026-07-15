use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_document_repository::LocalDocumentRepository;
use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_domain::document::DocumentBodyPolicy;
use cabinet_usecases::document::{
    CreateDocumentInput, CreateDocumentProductEvent, CreateDocumentUsecase, DocumentChangeEvent,
    DocumentChangeEventPublisher, DocumentProductLogger, GetCurrentDocumentInput,
    GetCurrentDocumentUsecase, GetDocumentHistoryInput, GetDocumentHistoryUsecase,
    GetDocumentVersionInput, GetDocumentVersionUsecase, PreviewDocumentRestoreInput,
    PreviewDocumentRestoreUsecase, RestoreDocumentVersionInput, RestoreDocumentVersionState,
    RestoreDocumentVersionUsecase, UpdateDocumentInput, UpdateDocumentUsecase,
};

struct TempAppDataRoot {
    path: PathBuf,
}

impl TempAppDataRoot {
    fn new(test_name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = PathBuf::from("/tmp").join(format!(
            "sponzey-cabinet-durable-authoring-{test_name}-{}-{nanos}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temp app data root");
        Self { path }
    }
}

impl Drop for TempAppDataRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Default)]
struct RecordingPublisher {
    events: Vec<DocumentChangeEvent>,
}

impl DocumentChangeEventPublisher for RecordingPublisher {
    fn publish(&mut self, event: DocumentChangeEvent) {
        self.events.push(event);
    }
}

#[derive(Default)]
struct RecordingProductLogger {
    events: Vec<CreateDocumentProductEvent>,
}

impl DocumentProductLogger for RecordingProductLogger {
    fn write_product(&mut self, event: CreateDocumentProductEvent) {
        self.events.push(event);
    }
}

#[test]
fn local_durable_authoring_survives_runtime_reconstruction_and_restores_version() {
    let temp = TempAppDataRoot::new("restart-flow");
    let body_policy = DocumentBodyPolicy::new(4096).expect("body policy");
    let mut documents = LocalDocumentRepository::new(temp.path.clone());
    let mut versions = LocalVersionStore::new(temp.path.clone());
    let mut publisher = RecordingPublisher::default();
    let mut logger = RecordingProductLogger::default();

    let created = CreateDocumentUsecase::new(body_policy)
        .execute(
            CreateDocumentInput::new(
                "workspace-1",
                "doc-1",
                "docs/daily-notes.md",
                "initial line\n",
                "version-1",
                "snapshot-1",
                "local-user",
                "Create daily notes",
            ),
            &mut documents,
            &mut versions,
            &mut publisher,
            &mut logger,
        )
        .expect("create document");
    assert_eq!(created.version_id().as_str(), "version-1");

    let updated = UpdateDocumentUsecase::new(body_policy)
        .execute(
            UpdateDocumentInput::new(
                "workspace-1",
                "doc-1",
                "initial line\nupdated line\n",
                "version-2",
                "snapshot-2",
                "local-user",
                "Update daily notes",
            ),
            &mut documents,
            &mut versions,
            &mut publisher,
            &mut logger,
        )
        .expect("update document");
    assert_eq!(updated.version_id().as_str(), "version-2");

    drop(documents);
    drop(versions);

    let mut documents = LocalDocumentRepository::new(temp.path.clone());
    let mut versions = LocalVersionStore::new(temp.path.clone());

    let current = GetCurrentDocumentUsecase::new()
        .execute(
            GetCurrentDocumentInput::by_id("workspace-1", "doc-1"),
            &documents,
        )
        .expect("current after restart");
    assert_eq!(
        current.record().body().as_str(),
        "initial line\nupdated line\n"
    );

    let first_history_page = GetDocumentHistoryUsecase::new()
        .execute(
            GetDocumentHistoryInput::new("workspace-1", "doc-1", None, 1),
            &versions,
        )
        .expect("first history page");
    assert_eq!(first_history_page.page().entries().len(), 1);
    assert_eq!(
        first_history_page.page().entries()[0].version_id().as_str(),
        "version-1"
    );
    let next_cursor = first_history_page
        .page()
        .next_cursor()
        .expect("next cursor")
        .as_str()
        .to_string();
    let second_history_page = GetDocumentHistoryUsecase::new()
        .execute(
            GetDocumentHistoryInput::new("workspace-1", "doc-1", Some(&next_cursor), 1),
            &versions,
        )
        .expect("second history page");
    assert_eq!(second_history_page.page().entries().len(), 1);
    assert_eq!(
        second_history_page.page().entries()[0]
            .version_id()
            .as_str(),
        "version-2"
    );

    let initial_snapshot = GetDocumentVersionUsecase::new()
        .execute(
            GetDocumentVersionInput::new("workspace-1", "doc-1", "version-1"),
            &versions,
        )
        .expect("specific version");
    assert_eq!(
        initial_snapshot.snapshot().body().as_str(),
        "initial line\n"
    );

    let preview = PreviewDocumentRestoreUsecase::new()
        .execute(
            PreviewDocumentRestoreInput::new("workspace-1", "doc-1", "version-1"),
            &documents,
            &versions,
        )
        .expect("restore preview");
    assert!(preview.can_restore());
    assert_eq!(preview.target_version_id().as_str(), "version-1");
    assert!(
        preview
            .lines()
            .iter()
            .any(|line| line.text() == "updated line")
    );

    let current_after_preview = GetCurrentDocumentUsecase::new()
        .execute(
            GetCurrentDocumentInput::by_id("workspace-1", "doc-1"),
            &documents,
        )
        .expect("current after preview");
    assert_eq!(
        current_after_preview.record().body().as_str(),
        "initial line\nupdated line\n"
    );

    let restored = RestoreDocumentVersionUsecase::new()
        .execute(
            RestoreDocumentVersionInput::new(
                "workspace-1",
                "doc-1",
                "version-1",
                "version-restore-1",
                "snapshot-restore-1",
                "local-user",
                "Restore version-1",
            ),
            &mut documents,
            &mut versions,
            &mut publisher,
            &mut logger,
        )
        .expect("restore apply");
    assert_eq!(
        restored.final_state(),
        RestoreDocumentVersionState::Completed
    );
    assert_eq!(restored.restored_version_id().as_str(), "version-restore-1");

    drop(documents);
    drop(versions);

    let documents = LocalDocumentRepository::new(temp.path.clone());
    let versions = LocalVersionStore::new(temp.path.clone());
    let current_after_restore = GetCurrentDocumentUsecase::new()
        .execute(
            GetCurrentDocumentInput::by_id("workspace-1", "doc-1"),
            &documents,
        )
        .expect("current after restore restart");
    assert_eq!(
        current_after_restore.record().body().as_str(),
        "initial line\n"
    );

    let history_after_restore = GetDocumentHistoryUsecase::new()
        .execute(
            GetDocumentHistoryInput::new("workspace-1", "doc-1", None, 10),
            &versions,
        )
        .expect("history after restore");
    let version_ids: Vec<_> = history_after_restore
        .page()
        .entries()
        .iter()
        .map(|entry| entry.version_id().as_str().to_string())
        .collect();
    assert_eq!(
        version_ids,
        vec![
            "version-1".to_string(),
            "version-2".to_string(),
            "version-restore-1".to_string()
        ]
    );
    assert!(format!("{:?}", logger.events).contains("DocumentRestored"));
    assert!(!format!("{:?}", logger.events).contains("initial line"));
    assert!(!format!("{:?}", logger.events).contains("updated line"));
}

#[test]
fn local_durable_authoring_read_paths_keep_p95_under_300ms() {
    let temp = TempAppDataRoot::new("budget");
    let body_policy = DocumentBodyPolicy::new(4096).expect("body policy");
    let mut documents = LocalDocumentRepository::new(temp.path.clone());
    let mut versions = LocalVersionStore::new(temp.path.clone());
    let mut publisher = RecordingPublisher::default();
    let mut logger = RecordingProductLogger::default();

    CreateDocumentUsecase::new(body_policy)
        .execute(
            CreateDocumentInput::new(
                "workspace-1",
                "doc-1",
                "docs/budget-fixture.md",
                "line 1\nline 2\n",
                "version-1",
                "snapshot-1",
                "local-user",
                "Create budget fixture",
            ),
            &mut documents,
            &mut versions,
            &mut publisher,
            &mut logger,
        )
        .expect("create document");
    UpdateDocumentUsecase::new(body_policy)
        .execute(
            UpdateDocumentInput::new(
                "workspace-1",
                "doc-1",
                "line 1\nline 2\nline 3\n",
                "version-2",
                "snapshot-2",
                "local-user",
                "Update budget fixture",
            ),
            &mut documents,
            &mut versions,
            &mut publisher,
            &mut logger,
        )
        .expect("update document");

    let current_p95 = p95_ms(repeat_measurements(|| {
        GetCurrentDocumentUsecase::new()
            .execute(
                GetCurrentDocumentInput::by_id("workspace-1", "doc-1"),
                &documents,
            )
            .expect("current read");
    }));
    let history_p95 = p95_ms(repeat_measurements(|| {
        GetDocumentHistoryUsecase::new()
            .execute(
                GetDocumentHistoryInput::new("workspace-1", "doc-1", None, 20),
                &versions,
            )
            .expect("history read");
    }));
    let version_p95 = p95_ms(repeat_measurements(|| {
        GetDocumentVersionUsecase::new()
            .execute(
                GetDocumentVersionInput::new("workspace-1", "doc-1", "version-1"),
                &versions,
            )
            .expect("version read");
    }));
    let restore_preview_p95 = p95_ms(repeat_measurements(|| {
        PreviewDocumentRestoreUsecase::new()
            .execute(
                PreviewDocumentRestoreInput::new("workspace-1", "doc-1", "version-1"),
                &documents,
                &versions,
            )
            .expect("restore preview");
    }));

    assert!(current_p95 <= 300, "current_p95={current_p95}");
    assert!(history_p95 <= 300, "history_p95={history_p95}");
    assert!(version_p95 <= 300, "version_p95={version_p95}");
    assert!(
        restore_preview_p95 <= 300,
        "restore_preview_p95={restore_preview_p95}"
    );
}

fn repeat_measurements(mut operation: impl FnMut()) -> Vec<u128> {
    let mut measurements = Vec::with_capacity(40);
    for _ in 0..40 {
        let started = Instant::now();
        operation();
        measurements.push(started.elapsed().as_millis());
    }
    measurements
}

fn p95_ms(mut measurements: Vec<u128>) -> u128 {
    measurements.sort_unstable();
    let index = ((measurements.len() as f64 * 0.95).ceil() as usize).saturating_sub(1);
    measurements[index]
}
