use std::collections::HashMap;

use cabinet_domain::asset::{
    AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId, AssetMediaType,
    AssetMetadata, AssetPreviewCapability,
};
use cabinet_domain::canvas::{
    Canvas, CanvasGeometryPolicy, CanvasId, CanvasLifecycleState, CanvasRevision,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_metadata_catalog::{
    AssetMetadataCatalog, AssetMetadataCatalogError, AssetMetadataPage, AssetMetadataPutOutcome,
};
use cabinet_ports::canvas_repository::{CanvasRecord, CanvasRepository, CanvasRepositoryError};
use cabinet_ports::document_existence::{DocumentExistenceError, DocumentExistenceReader};
use cabinet_usecases::canvas_mutation::{
    AddCanvasNodeMutationInput, AddCanvasNodeMutationUsecase, AddValidatedCanvasNodeUsecase,
    AutoArrangeCanvasInput, AutoArrangeCanvasUsecase, CanvasAutoArrangePolicy,
    CanvasMutationPolicy, CanvasMutationProductEvent, CanvasMutationProductLogger,
    CanvasNodeTargetInput, ConnectCanvasEdgeInput, ConnectCanvasEdgeUsecase,
    PreviewAutoArrangeCanvasUsecase, RemoveCanvasEdgeInput, RemoveCanvasEdgeUsecase,
    RemoveCanvasNodeInput, RemoveCanvasNodeUsecase, UpdateCanvasNodeGeometryInput,
    UpdateCanvasNodeGeometryUsecase, UpdateCanvasTextCardInput, UpdateCanvasTextCardUsecase,
    UpdateCanvasViewportInput, UpdateCanvasViewportUsecase,
};

#[test]
fn text_card_edit_is_revisioned_and_preserves_geometry_without_logging_content() {
    let mut repository = repository_with_state(CanvasLifecycleState::Draft);
    let mut logger = RecordingLogger::default();
    AddCanvasNodeMutationUsecase::new()
        .execute(
            AddCanvasNodeMutationInput::new(
                "workspace-1",
                "canvas-1",
                1,
                "memo-1",
                CanvasNodeTargetInput::Text("old text".into()),
                40,
                60,
                320,
                180,
            ),
            &policy(),
            &mut repository,
            &mut logger,
        )
        .expect("add text card");

    let edited = UpdateCanvasTextCardUsecase::new()
        .execute(
            UpdateCanvasTextCardInput::new(
                "workspace-1",
                "canvas-1",
                2,
                "memo-1",
                "new private text",
            ),
            &mut repository,
            &mut logger,
        )
        .expect("edit text card");

    assert_eq!(edited.record().revision().value(), 3);
    let node = &edited.record().canvas().nodes()[0];
    assert_eq!(node.position().x(), 40);
    assert_eq!(node.position().y(), 60);
    assert_eq!(node.geometry().size().width(), 320);
    assert!(
        matches!(node.target(), cabinet_domain::canvas::CanvasNodeTarget::TextCard(value) if value.as_str() == "new private text")
    );
    assert!(matches!(
        logger.events.last(),
        Some(CanvasMutationProductEvent::TextCardUpdated {
            revision: 3,
            changed_node_count: 1,
            ..
        })
    ));
    assert!(!format!("{:?}", logger.events).contains("new private text"));
}

#[test]
fn text_card_edit_rejects_invalid_wrong_kind_stale_and_archived_without_write() {
    let mut repository = repository_with_state(CanvasLifecycleState::Draft);
    let mut logger = RecordingLogger::default();
    AddCanvasNodeMutationUsecase::new()
        .execute(
            AddCanvasNodeMutationInput::new(
                "workspace-1",
                "canvas-1",
                1,
                "document-1",
                CanvasNodeTargetInput::Document("doc-1".into()),
                0,
                0,
                320,
                180,
            ),
            &policy(),
            &mut repository,
            &mut logger,
        )
        .expect("add document card");
    let writes = repository.replace_calls;
    let logs = logger.events.len();

    for (revision, node, text, expected) in [
        (2, "missing", "text", "CANVAS_NODE_NOT_FOUND"),
        (2, "document-1", "text", "CANVAS_NODE_TARGET_MISMATCH"),
        (2, "document-1", "  ", "CANVAS_INVALID_INPUT"),
        (1, "document-1", "text", "CANVAS_VERSION_CONFLICT"),
    ] {
        let error = UpdateCanvasTextCardUsecase::new()
            .execute(
                UpdateCanvasTextCardInput::new("workspace-1", "canvas-1", revision, node, text),
                &mut repository,
                &mut logger,
            )
            .expect_err("edit must fail");
        assert_eq!(error.code(), expected);
    }
    assert_eq!(repository.replace_calls, writes);
    assert_eq!(logger.events.len(), logs);

    let mut archived = repository_with_state(CanvasLifecycleState::Archived);
    let error = UpdateCanvasTextCardUsecase::new()
        .execute(
            UpdateCanvasTextCardInput::new("workspace-1", "canvas-1", 1, "memo-1", "text"),
            &mut archived,
            &mut RecordingLogger::default(),
        )
        .expect_err("archived");
    assert_eq!(error.code(), "CANVAS_INVALID_STATE");
    assert_eq!(archived.replace_calls, 0);
}

#[test]
fn node_edge_and_cascade_remove_are_revisioned_and_safely_logged() {
    let mut repository = repository_with_state(CanvasLifecycleState::Draft);
    let mut logger = RecordingLogger::default();
    let policy = policy();
    let first = AddCanvasNodeMutationUsecase::new()
        .execute(
            AddCanvasNodeMutationInput::new(
                "workspace-1",
                "canvas-1",
                1,
                "node-1",
                CanvasNodeTargetInput::Document("doc-1".into()),
                10,
                20,
                320,
                180,
            ),
            &policy,
            &mut repository,
            &mut logger,
        )
        .expect("first node");
    let second = AddCanvasNodeMutationUsecase::new()
        .execute(
            AddCanvasNodeMutationInput::new(
                "workspace-1",
                "canvas-1",
                2,
                "node-2",
                CanvasNodeTargetInput::Text("Private note".into()),
                500,
                20,
                240,
                120,
            ),
            &policy,
            &mut repository,
            &mut logger,
        )
        .expect("second node");
    let connected = ConnectCanvasEdgeUsecase::new()
        .execute(
            ConnectCanvasEdgeInput::new("workspace-1", "canvas-1", 3, "edge-1", "node-1", "node-2"),
            &policy,
            &mut repository,
            &mut logger,
        )
        .expect("edge");
    let removed = RemoveCanvasNodeUsecase::new()
        .execute(
            RemoveCanvasNodeInput::new("workspace-1", "canvas-1", 4, "node-1"),
            &mut repository,
            &mut logger,
        )
        .expect("remove");

    assert_eq!(first.record().revision().value(), 2);
    assert_eq!(second.record().revision().value(), 3);
    assert_eq!(connected.record().canvas().edges().len(), 1);
    assert_eq!(removed.record().revision().value(), 5);
    assert_eq!(removed.record().canvas().nodes().len(), 1);
    assert!(removed.record().canvas().edges().is_empty());
    assert_eq!(logger.events.len(), 4);
    assert!(!format!("{:?}", logger.events).contains("Private note"));
}

#[test]
fn geometry_viewport_and_auto_arrange_are_deterministic_revisioned_mutations() {
    let mut repository = repository_with_state(CanvasLifecycleState::Draft);
    let mut logger = RecordingLogger::default();
    let mutation = policy();
    for (revision, id) in [(1, "node-b"), (2, "node-a")] {
        AddCanvasNodeMutationUsecase::new()
            .execute(
                AddCanvasNodeMutationInput::new(
                    "workspace-1",
                    "canvas-1",
                    revision,
                    id,
                    CanvasNodeTargetInput::Document(format!("doc-{id}")),
                    0,
                    0,
                    320,
                    180,
                ),
                &mutation,
                &mut repository,
                &mut logger,
            )
            .expect("add");
    }
    let moved = UpdateCanvasNodeGeometryUsecase::new()
        .execute(
            UpdateCanvasNodeGeometryInput::new(
                "workspace-1",
                "canvas-1",
                3,
                "node-a",
                90,
                120,
                400,
                220,
            ),
            &mutation,
            &mut repository,
            &mut logger,
        )
        .expect("geometry");
    assert_eq!(
        moved
            .record()
            .canvas()
            .nodes()
            .iter()
            .find(|n| n.id().as_str() == "node-a")
            .expect("node")
            .geometry()
            .size()
            .width(),
        400
    );
    let viewport = UpdateCanvasViewportUsecase::new()
        .execute(
            UpdateCanvasViewportInput::new("workspace-1", "canvas-1", 4, 300, 240, 125),
            &mutation,
            &mut repository,
            &mut logger,
        )
        .expect("viewport");
    assert_eq!(viewport.record().viewport().zoom_percent(), 125);
    let arranged = AutoArrangeCanvasUsecase::new()
        .execute(
            AutoArrangeCanvasInput::new("workspace-1", "canvas-1", 5),
            &CanvasAutoArrangePolicy::new(2, 40, 60, 400, 260).expect("arrange policy"),
            &mut repository,
            &mut logger,
        )
        .expect("arrange");
    assert_eq!(arranged.record().revision().value(), 6);
    assert_eq!(
        arranged.record().canvas().nodes()[0].id().as_str(),
        "node-a"
    );
    assert_eq!(arranged.record().canvas().nodes()[0].position().x(), 40);
    assert_eq!(arranged.record().canvas().nodes()[1].position().x(), 440);
}

#[test]
fn auto_arrange_preview_is_deterministic_and_does_not_write_or_log() {
    let mut repository = repository_with_state(CanvasLifecycleState::Draft);
    let mut logger = RecordingLogger::default();
    for (revision, id) in [(1, "node-b"), (2, "node-a")] {
        AddCanvasNodeMutationUsecase::new()
            .execute(
                AddCanvasNodeMutationInput::new(
                    "workspace-1",
                    "canvas-1",
                    revision,
                    id,
                    CanvasNodeTargetInput::Document(format!("doc-{id}")),
                    0,
                    0,
                    320,
                    180,
                ),
                &policy(),
                &mut repository,
                &mut logger,
            )
            .expect("add");
    }
    let writes_before_preview = repository.replace_calls;
    let logs_before_preview = logger.events.len();
    let preview = PreviewAutoArrangeCanvasUsecase::new()
        .execute(
            AutoArrangeCanvasInput::new("workspace-1", "canvas-1", 3),
            &CanvasAutoArrangePolicy::new(2, 40, 60, 400, 260).expect("arrange policy"),
            &repository,
        )
        .expect("preview");

    assert_eq!(preview.record().revision().value(), 3);
    assert_eq!(preview.record().canvas().nodes()[0].id().as_str(), "node-a");
    assert_eq!(preview.record().canvas().nodes()[0].position().x(), 40);
    assert_eq!(preview.record().canvas().nodes()[1].position().x(), 440);
    assert_eq!(repository.replace_calls, writes_before_preview);
    assert_eq!(logger.events.len(), logs_before_preview);
}

#[test]
fn validated_target_nodes_reject_missing_and_persist_existing_document_and_asset() {
    let mut repository = repository_with_state(CanvasLifecycleState::Draft);
    let mut logger = RecordingLogger::default();
    let missing_documents = FakeDocuments::value(false);
    let missing_assets = FakeAssets::value(None);
    let asset_id = "a".repeat(64);

    let document_error = AddValidatedCanvasNodeUsecase::new()
        .execute(
            AddCanvasNodeMutationInput::new(
                "workspace-1",
                "canvas-1",
                1,
                "document-1",
                CanvasNodeTargetInput::Document("doc-1".into()),
                0,
                0,
                320,
                180,
            ),
            &policy(),
            &missing_documents,
            &missing_assets,
            &mut repository,
            &mut logger,
        )
        .expect_err("missing document");
    assert_eq!(document_error.code(), "CANVAS_DOCUMENT_TARGET_NOT_FOUND");

    let asset_error = AddValidatedCanvasNodeUsecase::new()
        .execute(
            AddCanvasNodeMutationInput::new(
                "workspace-1",
                "canvas-1",
                1,
                "asset-1",
                CanvasNodeTargetInput::Attachment(asset_id.clone()),
                0,
                0,
                320,
                180,
            ),
            &policy(),
            &FakeDocuments::value(true),
            &missing_assets,
            &mut repository,
            &mut logger,
        )
        .expect_err("missing asset");
    assert_eq!(asset_error.code(), "CANVAS_ASSET_TARGET_NOT_FOUND");
    assert_eq!(repository.replace_calls, 0);
    assert!(logger.events.is_empty());

    let document = AddValidatedCanvasNodeUsecase::new()
        .execute(
            AddCanvasNodeMutationInput::new(
                "workspace-1",
                "canvas-1",
                1,
                "document-1",
                CanvasNodeTargetInput::Document("doc-1".into()),
                0,
                0,
                320,
                180,
            ),
            &policy(),
            &FakeDocuments::value(true),
            &missing_assets,
            &mut repository,
            &mut logger,
        )
        .expect("existing document");
    assert_eq!(document.record().revision().value(), 2);

    let asset = AddValidatedCanvasNodeUsecase::new()
        .execute(
            AddCanvasNodeMutationInput::new(
                "workspace-1",
                "canvas-1",
                2,
                "asset-1",
                CanvasNodeTargetInput::Attachment(asset_id),
                0,
                0,
                320,
                180,
            ),
            &policy(),
            &FakeDocuments::value(true),
            &FakeAssets::value(Some(asset_record())),
            &mut repository,
            &mut logger,
        )
        .expect("existing asset");
    assert_eq!(asset.record().revision().value(), 3);
}

#[test]
fn validated_target_nodes_map_reader_failures_without_canvas_write() {
    let mut repository = repository_with_state(CanvasLifecycleState::Draft);
    let mut logger = RecordingLogger::default();
    let input = || {
        AddCanvasNodeMutationInput::new(
            "workspace-1",
            "canvas-1",
            1,
            "document-1",
            CanvasNodeTargetInput::Document("doc-1".into()),
            0,
            0,
            320,
            180,
        )
    };
    let storage = AddValidatedCanvasNodeUsecase::new()
        .execute(
            input(),
            &policy(),
            &FakeDocuments::error(DocumentExistenceError::StorageUnavailable),
            &FakeAssets::value(None),
            &mut repository,
            &mut logger,
        )
        .expect_err("storage");
    assert_eq!(storage.code(), "CANVAS_STORAGE_UNAVAILABLE");
    let recovery = AddValidatedCanvasNodeUsecase::new()
        .execute(
            input(),
            &policy(),
            &FakeDocuments::error(DocumentExistenceError::CorruptedRecord),
            &FakeAssets::value(None),
            &mut repository,
            &mut logger,
        )
        .expect_err("recovery");
    assert_eq!(recovery.code(), "CANVAS_RECOVERY_REQUIRED");
    assert_eq!(repository.replace_calls, 0);
    assert!(logger.events.is_empty());
}

#[test]
fn stale_archived_duplicate_and_limits_do_not_write_or_log() {
    let mut repository = repository_with_state(CanvasLifecycleState::Draft);
    let mut logger = RecordingLogger::default();
    let one_node = CanvasMutationPolicy::new(1, 1, geometry_policy()).expect("policy");
    AddCanvasNodeMutationUsecase::new()
        .execute(
            AddCanvasNodeMutationInput::new(
                "workspace-1",
                "canvas-1",
                1,
                "node-1",
                CanvasNodeTargetInput::Document("doc-1".into()),
                0,
                0,
                320,
                180,
            ),
            &one_node,
            &mut repository,
            &mut logger,
        )
        .expect("node");
    let limit = AddCanvasNodeMutationUsecase::new().execute(
        AddCanvasNodeMutationInput::new(
            "workspace-1",
            "canvas-1",
            2,
            "node-2",
            CanvasNodeTargetInput::Document("doc-2".into()),
            0,
            0,
            320,
            180,
        ),
        &one_node,
        &mut repository,
        &mut logger,
    );
    assert_eq!(
        limit.expect_err("limit").code(),
        "CANVAS_NODE_LIMIT_EXCEEDED"
    );
    let stale = RemoveCanvasNodeUsecase::new().execute(
        RemoveCanvasNodeInput::new("workspace-1", "canvas-1", 1, "node-1"),
        &mut repository,
        &mut logger,
    );
    assert_eq!(stale.expect_err("stale").code(), "CANVAS_VERSION_CONFLICT");
    assert_eq!(logger.events.len(), 1);

    let mut archived = repository_with_state(CanvasLifecycleState::Archived);
    let blocked = AddCanvasNodeMutationUsecase::new().execute(
        AddCanvasNodeMutationInput::new(
            "workspace-1",
            "canvas-1",
            1,
            "node",
            CanvasNodeTargetInput::Text("note".into()),
            0,
            0,
            320,
            180,
        ),
        &policy(),
        &mut archived,
        &mut RecordingLogger::default(),
    );
    assert_eq!(
        blocked.expect_err("archived").code(),
        "CANVAS_INVALID_STATE"
    );
}

#[test]
fn edge_remove_is_revisioned_preserves_nodes_and_rejects_missing_edge() {
    let mut repository = repository_with_state(CanvasLifecycleState::Draft);
    let mut logger = RecordingLogger::default();
    for (revision, node) in [(1, "node-1"), (2, "node-2")] {
        AddCanvasNodeMutationUsecase::new()
            .execute(
                AddCanvasNodeMutationInput::new(
                    "workspace-1",
                    "canvas-1",
                    revision,
                    node,
                    CanvasNodeTargetInput::Text(node.into()),
                    0,
                    0,
                    320,
                    180,
                ),
                &policy(),
                &mut repository,
                &mut logger,
            )
            .expect("node");
    }
    ConnectCanvasEdgeUsecase::new()
        .execute(
            ConnectCanvasEdgeInput::new("workspace-1", "canvas-1", 3, "edge-1", "node-1", "node-2"),
            &policy(),
            &mut repository,
            &mut logger,
        )
        .expect("connect");
    let removed = RemoveCanvasEdgeUsecase::new()
        .execute(
            RemoveCanvasEdgeInput::new("workspace-1", "canvas-1", 4, "edge-1"),
            &mut repository,
            &mut logger,
        )
        .expect("remove edge");
    assert_eq!(removed.record().revision().value(), 5);
    assert_eq!(removed.record().canvas().nodes().len(), 2);
    assert!(removed.record().canvas().edges().is_empty());
    assert_eq!(
        RemoveCanvasEdgeUsecase::new()
            .execute(
                RemoveCanvasEdgeInput::new("workspace-1", "canvas-1", 5, "edge-1"),
                &mut repository,
                &mut logger,
            )
            .expect_err("missing")
            .code(),
        "CANVAS_EDGE_NOT_FOUND",
    );
}

fn geometry_policy() -> CanvasGeometryPolicy {
    CanvasGeometryPolicy::new(80, 1200, 60, 900, 25, 400).expect("geometry")
}
fn policy() -> CanvasMutationPolicy {
    CanvasMutationPolicy::new(10, 10, geometry_policy()).expect("policy")
}

#[derive(Default)]
struct RecordingLogger {
    events: Vec<CanvasMutationProductEvent>,
}
impl CanvasMutationProductLogger for RecordingLogger {
    fn write_product(&mut self, event: CanvasMutationProductEvent) {
        self.events.push(event);
    }
}

fn repository_with_state(state: CanvasLifecycleState) -> FakeRepository {
    let canvas = Canvas::new(
        CanvasId::new("canvas-1").expect("id"),
        vec![],
        vec![],
        state,
    )
    .expect("canvas");
    let mut repository = FakeRepository::default();
    repository
        .create_canvas(
            &WorkspaceId::new("workspace-1").expect("workspace"),
            CanvasRecord::new(canvas).expect("record"),
        )
        .expect("create");
    repository
}

#[derive(Default)]
struct FakeRepository {
    records: HashMap<(String, String), CanvasRecord>,
    replace_calls: usize,
}
impl CanvasRepository for FakeRepository {
    fn create_canvas(
        &mut self,
        workspace: &WorkspaceId,
        record: CanvasRecord,
    ) -> Result<(), CanvasRepositoryError> {
        self.records.insert(
            (
                workspace.as_str().into(),
                record.canvas().id().as_str().into(),
            ),
            record,
        );
        Ok(())
    }
    fn replace_canvas(
        &mut self,
        workspace: &WorkspaceId,
        expected: CanvasRevision,
        record: CanvasRecord,
    ) -> Result<(), CanvasRepositoryError> {
        self.replace_calls += 1;
        let key = (
            workspace.as_str().into(),
            record.canvas().id().as_str().into(),
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
            .get(&(workspace.as_str().into(), canvas.as_str().into()))
            .cloned())
    }
}

struct FakeDocuments(Result<bool, DocumentExistenceError>);
impl FakeDocuments {
    fn value(value: bool) -> Self {
        Self(Ok(value))
    }
    fn error(error: DocumentExistenceError) -> Self {
        Self(Err(error))
    }
}
impl DocumentExistenceReader for FakeDocuments {
    fn exists(
        &self,
        _: &WorkspaceId,
        _: &cabinet_domain::document::DocumentId,
    ) -> Result<bool, DocumentExistenceError> {
        self.0
    }
}

struct FakeAssets(Result<Option<AssetCatalogRecord>, AssetMetadataCatalogError>);
impl FakeAssets {
    fn value(value: Option<AssetCatalogRecord>) -> Self {
        Self(Ok(value))
    }
}
impl AssetMetadataCatalog for FakeAssets {
    fn put(
        &mut self,
        _: &WorkspaceId,
        _: AssetCatalogRecord,
    ) -> Result<AssetMetadataPutOutcome, AssetMetadataCatalogError> {
        unreachable!("read-only test fake")
    }
    fn get(
        &self,
        _: &WorkspaceId,
        _: &AssetId,
    ) -> Result<Option<AssetCatalogRecord>, AssetMetadataCatalogError> {
        self.0.clone()
    }
    fn list(
        &self,
        _: &WorkspaceId,
        _: Option<&str>,
        _: usize,
    ) -> Result<AssetMetadataPage, AssetMetadataCatalogError> {
        unreachable!("read-only test fake")
    }
}

fn asset_record() -> AssetCatalogRecord {
    let id = AssetId::from_sha256_hex(&"a".repeat(64)).expect("asset id");
    let media = AssetMediaType::new("text/plain").expect("media");
    let metadata = AssetMetadata::new(
        id,
        AssetFileName::new("note.txt").expect("name"),
        media.clone(),
        4,
    )
    .expect("metadata");
    AssetCatalogRecord::new(
        metadata,
        1,
        AssetPreviewCapability::for_media_type(&media),
        AssetExtractionStatus::NotRequested,
    )
    .expect("record")
}
