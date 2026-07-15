use std::collections::HashMap;

use cabinet_domain::canvas::{CanvasId, CanvasLifecycleState, CanvasRevision};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_repository::{CanvasRecord, CanvasRepository, CanvasRepositoryError};
use cabinet_usecases::canvas_lifecycle::{
    ArchiveCanvasInput, ArchiveCanvasUsecase, CanvasLifecycleProductEvent,
    CanvasLifecycleProductLogger, CreateCanvasRecordInput, CreateCanvasRecordUsecase,
    GetCanvasRecordInput, GetCanvasRecordUsecase, RenameCanvasInput, RenameCanvasUsecase,
};

#[test]
fn create_load_and_duplicate_return_explicit_results_and_safe_log() {
    let mut repository = FakeRepository::default();
    let mut logger = RecordingLogger::default();
    let created = CreateCanvasRecordUsecase::new()
        .execute(
            CreateCanvasRecordInput::new("workspace-1", "canvas-1", "Product map"),
            &mut repository,
            &mut logger,
        )
        .expect("create");
    let loaded = GetCanvasRecordUsecase::new()
        .execute(
            GetCanvasRecordInput::new("workspace-1", "canvas-1"),
            &repository,
        )
        .expect("load");

    assert_eq!(created.record().revision().value(), 1);
    assert_eq!(loaded.record().title().as_str(), "Product map");
    assert_eq!(
        loaded.record().canvas().state(),
        CanvasLifecycleState::Draft
    );
    assert_eq!(logger.events.len(), 1);
    assert!(matches!(
        logger.events[0],
        CanvasLifecycleProductEvent::Created { revision: 1, .. }
    ));
    let duplicate = CreateCanvasRecordUsecase::new().execute(
        CreateCanvasRecordInput::new("workspace-1", "canvas-1", "Duplicate"),
        &mut repository,
        &mut logger,
    );
    assert_eq!(
        duplicate.expect_err("duplicate").code(),
        "CANVAS_ALREADY_EXISTS"
    );
    assert_eq!(logger.events.len(), 1);
}

#[test]
fn rename_increments_revision_preserves_content_and_rejects_stale_write() {
    let mut repository = FakeRepository::default();
    let mut logger = RecordingLogger::default();
    CreateCanvasRecordUsecase::new()
        .execute(
            CreateCanvasRecordInput::new("workspace-1", "canvas-1", "Before"),
            &mut repository,
            &mut logger,
        )
        .expect("create");
    let renamed = RenameCanvasUsecase::new()
        .execute(
            RenameCanvasInput::new("workspace-1", "canvas-1", 1, "After"),
            &mut repository,
            &mut logger,
        )
        .expect("rename");

    assert_eq!(renamed.record().revision().value(), 2);
    assert_eq!(renamed.record().title().as_str(), "After");
    assert_eq!(renamed.record().canvas().nodes().len(), 0);
    assert_eq!(renamed.record().viewport().zoom_percent(), 100);
    let stale = RenameCanvasUsecase::new().execute(
        RenameCanvasInput::new("workspace-1", "canvas-1", 1, "Stale"),
        &mut repository,
        &mut logger,
    );
    assert_eq!(stale.expect_err("stale").code(), "CANVAS_VERSION_CONFLICT");
    assert_eq!(logger.events.len(), 2);
}

#[test]
fn archive_is_terminal_revisioned_and_logged_once() {
    let mut repository = FakeRepository::default();
    let mut logger = RecordingLogger::default();
    CreateCanvasRecordUsecase::new()
        .execute(
            CreateCanvasRecordInput::new("workspace-1", "canvas-1", "Archive me"),
            &mut repository,
            &mut logger,
        )
        .expect("create");
    let archived = ArchiveCanvasUsecase::new()
        .execute(
            ArchiveCanvasInput::new("workspace-1", "canvas-1", 1),
            &mut repository,
            &mut logger,
        )
        .expect("archive");
    assert_eq!(
        archived.record().canvas().state(),
        CanvasLifecycleState::Archived
    );
    assert_eq!(archived.record().revision().value(), 2);
    let again = ArchiveCanvasUsecase::new().execute(
        ArchiveCanvasInput::new("workspace-1", "canvas-1", 2),
        &mut repository,
        &mut logger,
    );
    assert_eq!(again.expect_err("terminal").code(), "CANVAS_INVALID_STATE");
    assert_eq!(logger.events.len(), 2);
}

#[derive(Default)]
struct RecordingLogger {
    events: Vec<CanvasLifecycleProductEvent>,
}
impl CanvasLifecycleProductLogger for RecordingLogger {
    fn write_product(&mut self, event: CanvasLifecycleProductEvent) {
        self.events.push(event);
    }
}

#[derive(Default)]
struct FakeRepository {
    records: HashMap<(String, String), CanvasRecord>,
}
impl CanvasRepository for FakeRepository {
    fn create_canvas(
        &mut self,
        workspace: &WorkspaceId,
        record: CanvasRecord,
    ) -> Result<(), CanvasRepositoryError> {
        let key = (
            workspace.as_str().to_string(),
            record.canvas().id().as_str().to_string(),
        );
        if self.records.contains_key(&key) {
            return Err(CanvasRepositoryError::AlreadyExists);
        }
        self.records.insert(key, record);
        Ok(())
    }
    fn replace_canvas(
        &mut self,
        workspace: &WorkspaceId,
        expected: CanvasRevision,
        record: CanvasRecord,
    ) -> Result<(), CanvasRepositoryError> {
        let key = (
            workspace.as_str().to_string(),
            record.canvas().id().as_str().to_string(),
        );
        if self.records.get(&key).map(CanvasRecord::revision) != Some(expected) {
            return Err(CanvasRepositoryError::VersionConflict);
        }
        self.records.insert(key, record);
        Ok(())
    }
    fn get_canvas(
        &self,
        workspace: &WorkspaceId,
        canvas: &CanvasId,
    ) -> Result<Option<CanvasRecord>, CanvasRepositoryError> {
        Ok(self
            .records
            .get(&(workspace.as_str().to_string(), canvas.as_str().to_string()))
            .cloned())
    }
}
