use std::collections::HashMap;

use cabinet_domain::canvas::{
    Canvas, CanvasId, CanvasLifecycleState, CanvasNode, CanvasNodeId, CanvasNodeTarget,
    CanvasPosition, CanvasRevision,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_repository::{CanvasRecord, CanvasRepository, CanvasRepositoryError};

#[derive(Default)]
struct FakeCanvasRepository {
    records: HashMap<(String, String), CanvasRecord>,
}

impl CanvasRepository for FakeCanvasRepository {
    fn create_canvas(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CanvasRecord,
    ) -> Result<(), CanvasRepositoryError> {
        let key = (
            workspace_id.as_str().to_string(),
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
        workspace_id: &WorkspaceId,
        expected_revision: CanvasRevision,
        record: CanvasRecord,
    ) -> Result<(), CanvasRepositoryError> {
        let key = (
            workspace_id.as_str().to_string(),
            record.canvas().id().as_str().to_string(),
        );
        if self.records.get(&key).map(CanvasRecord::revision) != Some(expected_revision) {
            return Err(CanvasRepositoryError::VersionConflict);
        }
        self.records.insert(key, record);
        Ok(())
    }

    fn get_canvas(
        &self,
        workspace_id: &WorkspaceId,
        canvas_id: &CanvasId,
    ) -> Result<Option<CanvasRecord>, CanvasRepositoryError> {
        Ok(self
            .records
            .get(&(
                workspace_id.as_str().to_string(),
                canvas_id.as_str().to_string(),
            ))
            .cloned())
    }
}

#[test]
fn canvas_repository_contract_preserves_workspace_canvas_and_lifecycle_state() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace");
    let canvas_id = CanvasId::new("canvas-1").expect("canvas id");
    let canvas = Canvas::new(
        canvas_id.clone(),
        vec![
            CanvasNode::new(
                CanvasNodeId::new("node-1").expect("node id"),
                CanvasNodeTarget::Document(DocumentId::new("doc-1").expect("document")),
                CanvasPosition::new(10, 20),
            )
            .expect("node"),
        ],
        vec![],
        CanvasLifecycleState::Saved,
    )
    .expect("canvas");
    let record = CanvasRecord::new(canvas).expect("record");
    let mut repository = FakeCanvasRepository::default();

    repository
        .create_canvas(&workspace_id, record)
        .expect("save canvas");

    let stored = repository
        .get_canvas(&workspace_id, &canvas_id)
        .expect("get canvas")
        .expect("stored canvas");

    assert_eq!(stored.canvas().id(), &canvas_id);
    assert_eq!(stored.canvas().state(), CanvasLifecycleState::Saved);
    assert_eq!(stored.revision().value(), 1);
}

#[test]
fn canvas_repository_contract_rejects_duplicate_create_and_stale_replace() {
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let id = CanvasId::new("canvas-1").expect("canvas");
    let canvas =
        Canvas::new(id.clone(), vec![], vec![], CanvasLifecycleState::Draft).expect("canvas");
    let mut repository = FakeCanvasRepository::default();
    repository
        .create_canvas(
            &workspace,
            CanvasRecord::new(canvas.clone()).expect("record"),
        )
        .expect("create");
    assert_eq!(
        repository
            .create_canvas(
                &workspace,
                CanvasRecord::new(canvas.clone()).expect("duplicate")
            )
            .expect_err("duplicate"),
        CanvasRepositoryError::AlreadyExists
    );
    let current = repository
        .get_canvas(&workspace, &id)
        .expect("get")
        .expect("current");
    let next = current
        .next(Canvas::new(id, vec![], vec![], CanvasLifecycleState::Updated).expect("updated"))
        .expect("next");
    assert_eq!(
        repository
            .replace_canvas(&workspace, CanvasRevision::new(2).expect("stale"), next)
            .expect_err("stale"),
        CanvasRepositoryError::VersionConflict
    );
}

#[test]
fn canvas_repository_error_codes_are_stable() {
    assert_eq!(
        CanvasRepositoryError::InvalidInput.code(),
        "canvas_repository.invalid_input",
    );
    assert_eq!(
        CanvasRepositoryError::StorageUnavailable.code(),
        "canvas_repository.storage_unavailable",
    );
    assert_eq!(
        CanvasRepositoryError::CorruptedCanvas.code(),
        "canvas_repository.corrupted_canvas",
    );
    assert_eq!(
        CanvasRepositoryError::VersionConflict.code(),
        "canvas_repository.version_conflict"
    );
}
