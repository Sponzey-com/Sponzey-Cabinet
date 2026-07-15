use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_work::{
    ProjectionChangeKind, ProjectionKind, ProjectionWork, ProjectionWorkIdentity,
    ProjectionWorkState,
};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::projection_work::{
    ProjectionEnqueueOutcome, ProjectionWorkRepository, ProjectionWorkRepositoryError,
};
use cabinet_usecases::reindex_asset_graph_projection::{
    ReindexAssetGraphProjectionInput, ReindexAssetGraphProjectionUsecase,
};

#[test]
fn asset_graph_reindex_enqueues_requested_change_and_resets_same_terminal_identity() {
    let pointer = Pointer;
    let mut repository = Repository::default();
    let input = ReindexAssetGraphProjectionInput::new(
        "workspace-1",
        "doc-1",
        ProjectionChangeKind::AssetAttached,
    )
    .unwrap();

    let first = ReindexAssetGraphProjectionUsecase::new()
        .execute(input.clone(), &pointer, &mut repository)
        .unwrap();
    assert_eq!(first.enqueued_count(), 1);
    repository.records[0] = ProjectionWork::restore(
        repository.records[0].identity().clone(),
        ProjectionWorkState::Ready,
        1,
    )
    .unwrap();
    let second = ReindexAssetGraphProjectionUsecase::new()
        .execute(input, &pointer, &mut repository)
        .unwrap();
    assert_eq!(second.reset_count(), 1);
    assert_eq!(repository.records[0].state(), ProjectionWorkState::Pending);
}

#[test]
fn asset_graph_reindex_supports_attach_detach_attach_on_one_document_version() {
    let pointer = Pointer;
    let mut repository = Repository::default();
    for change in [
        ProjectionChangeKind::AssetAttached,
        ProjectionChangeKind::AssetDetached,
        ProjectionChangeKind::AssetAttached,
    ] {
        let output = ReindexAssetGraphProjectionUsecase::new()
            .execute(
                ReindexAssetGraphProjectionInput::new("workspace-1", "doc-1", change).unwrap(),
                &pointer,
                &mut repository,
            )
            .unwrap();
        assert_eq!(output.enqueued_count() + output.reset_count(), 1);
        let work = repository
            .records
            .iter_mut()
            .find(|work| work.identity().change_kind() == change)
            .unwrap();
        *work = ProjectionWork::restore(work.identity().clone(), ProjectionWorkState::Ready, 1)
            .unwrap();
    }
    assert_eq!(repository.records.len(), 2);
}

#[test]
fn asset_graph_reindex_reuses_any_active_graph_work_without_duplicate_enqueue() {
    let pointer = Pointer;
    let mut repository = Repository {
        records: vec![ProjectionWork::pending(identity(
            ProjectionChangeKind::Updated,
        ))],
    };
    let output = ReindexAssetGraphProjectionUsecase::new()
        .execute(
            ReindexAssetGraphProjectionInput::new(
                "workspace-1",
                "doc-1",
                ProjectionChangeKind::AssetDetached,
            )
            .unwrap(),
            &pointer,
            &mut repository,
        )
        .unwrap();
    assert_eq!(output.already_active_count(), 1);
    assert_eq!(repository.records.len(), 1);
}

#[test]
fn asset_graph_ensure_keeps_ready_handoff_terminal_without_resetting_it() {
    let pointer = Pointer;
    let mut repository = Repository::default();
    let input = ReindexAssetGraphProjectionInput::new(
        "workspace-1",
        "doc-1",
        ProjectionChangeKind::AssetAttached,
    )
    .unwrap();
    let first = ReindexAssetGraphProjectionUsecase::new()
        .ensure(input.clone(), &pointer, &mut repository)
        .unwrap();
    assert_eq!(first.enqueued_count(), 1);
    repository.records[0] = ProjectionWork::restore(
        repository.records[0].identity().clone(),
        ProjectionWorkState::Ready,
        1,
    )
    .unwrap();

    let second = ReindexAssetGraphProjectionUsecase::new()
        .ensure(input, &pointer, &mut repository)
        .unwrap();

    assert_eq!(second.already_ready_count(), 1);
    assert_eq!(second.reset_count(), 0);
    assert_eq!(repository.records[0].state(), ProjectionWorkState::Ready);
}

struct Pointer;
impl CurrentDocumentVersionPointerPort for Pointer {
    fn load_current_version(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<Option<VersionId>, CurrentDocumentVersionPointerError> {
        Ok(Some(VersionId::new("v2").unwrap()))
    }
    fn compare_and_set_current_version(
        &mut self,
        _: &WorkspaceId,
        _: &DocumentId,
        _: Option<&VersionId>,
        _: VersionId,
    ) -> Result<(), CurrentDocumentVersionPointerError> {
        unreachable!()
    }
}

#[derive(Default)]
struct Repository {
    records: Vec<ProjectionWork>,
}
impl ProjectionWorkRepository for Repository {
    fn enqueue(
        &mut self,
        work: ProjectionWork,
    ) -> Result<ProjectionEnqueueOutcome, ProjectionWorkRepositoryError> {
        self.records.push(work);
        Ok(ProjectionEnqueueOutcome::Enqueued)
    }
    fn get(
        &self,
        identity: &ProjectionWorkIdentity,
    ) -> Result<Option<ProjectionWork>, ProjectionWorkRepositoryError> {
        Ok(self
            .records
            .iter()
            .find(|work| work.identity().idempotency_key() == identity.idempotency_key())
            .cloned())
    }
    fn replace(
        &mut self,
        work: ProjectionWork,
        expected: ProjectionWorkState,
    ) -> Result<(), ProjectionWorkRepositoryError> {
        let stored = self
            .records
            .iter_mut()
            .find(|stored| stored.identity().idempotency_key() == work.identity().idempotency_key())
            .ok_or(ProjectionWorkRepositoryError::NotFound)?;
        if stored.state() != expected {
            return Err(ProjectionWorkRepositoryError::Conflict);
        }
        *stored = work;
        Ok(())
    }
    fn list_resumable(
        &self,
        _: usize,
    ) -> Result<Vec<ProjectionWork>, ProjectionWorkRepositoryError> {
        Ok(vec![])
    }
}

fn identity(change: ProjectionChangeKind) -> ProjectionWorkIdentity {
    ProjectionWorkIdentity::for_change(
        WorkspaceId::new("workspace-1").unwrap(),
        DocumentId::new("doc-1").unwrap(),
        VersionId::new("v2").unwrap(),
        ProjectionKind::Graph,
        change,
    )
}
