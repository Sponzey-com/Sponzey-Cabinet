use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_work::{
    ProjectionChangeKind, ProjectionKind, ProjectionWork, ProjectionWorkIdentity,
    ProjectionWorkState,
};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_projection_catalog::{
    CurrentDocumentProjectionCatalog, CurrentDocumentProjectionCatalogError,
    CurrentDocumentProjectionIdentity,
};
use cabinet_ports::projection_work::{
    ProjectionEnqueueOutcome, ProjectionWorkRepository, ProjectionWorkRepositoryError,
};
use cabinet_usecases::restore_projection_rebuild::{
    RebuildRestoreProjectionsInput, RebuildRestoreProjectionsUsecase,
};

#[test]
fn restore_rebuild_enqueues_three_restored_works_per_current_document_idempotently() {
    let catalog = Catalog;
    let mut repository = Repository::default();
    let output = RebuildRestoreProjectionsUsecase::new(100)
        .execute(
            RebuildRestoreProjectionsInput::new("workspace-1"),
            &catalog,
            &mut repository,
        )
        .expect("rebuild request");
    assert_eq!(output.document_count(), 2);
    assert_eq!(output.enqueued_count(), 6);
    assert!(
        repository
            .works
            .iter()
            .all(|work| work.identity().change_kind() == ProjectionChangeKind::Restored)
    );

    let second = RebuildRestoreProjectionsUsecase::new(100)
        .execute(
            RebuildRestoreProjectionsInput::new("workspace-1"),
            &catalog,
            &mut repository,
        )
        .expect("idempotent");
    assert_eq!(second.enqueued_count(), 0);
    assert_eq!(second.duplicate_count(), 6);
}

#[test]
fn restore_rebuild_resets_failed_current_projection_before_enqueuing_restored_work() {
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let document = DocumentId::new("doc-1").unwrap();
    let version = VersionId::new("v1").unwrap();
    let failed = ProjectionWork::restore(
        ProjectionWorkIdentity::for_change(
            workspace,
            document,
            version,
            ProjectionKind::Graph,
            ProjectionChangeKind::Created,
        ),
        ProjectionWorkState::Failed,
        1,
    )
    .unwrap();
    let mut repository = Repository {
        works: vec![failed],
    };

    let output = RebuildRestoreProjectionsUsecase::new(100)
        .execute(
            RebuildRestoreProjectionsInput::new("workspace-1"),
            &Catalog,
            &mut repository,
        )
        .expect("rebuild request");

    assert_eq!(output.reset_count(), 1);
    assert!(repository.works.iter().any(|work| {
        work.identity().document_id().as_str() == "doc-1"
            && work.identity().kind() == ProjectionKind::Graph
            && work.identity().change_kind() == ProjectionChangeKind::Created
            && work.state() == ProjectionWorkState::Pending
    }));
}

struct Catalog;
impl CurrentDocumentProjectionCatalog for Catalog {
    fn list_current_projection_identities(
        &self,
        _: &WorkspaceId,
        _: usize,
    ) -> Result<Vec<CurrentDocumentProjectionIdentity>, CurrentDocumentProjectionCatalogError> {
        Ok([("doc-1", "v1"), ("doc-2", "v2")]
            .into_iter()
            .map(|(doc, version)| {
                CurrentDocumentProjectionIdentity::new(
                    DocumentId::new(doc).unwrap(),
                    VersionId::new(version).unwrap(),
                )
            })
            .collect())
    }
}

#[derive(Default)]
struct Repository {
    works: Vec<cabinet_domain::projection_work::ProjectionWork>,
}
impl ProjectionWorkRepository for Repository {
    fn enqueue(
        &mut self,
        work: cabinet_domain::projection_work::ProjectionWork,
    ) -> Result<ProjectionEnqueueOutcome, ProjectionWorkRepositoryError> {
        if self
            .works
            .iter()
            .any(|existing| existing.identity() == work.identity())
        {
            Ok(ProjectionEnqueueOutcome::AlreadyExists)
        } else {
            self.works.push(work);
            Ok(ProjectionEnqueueOutcome::Enqueued)
        }
    }
    fn get(
        &self,
        identity: &cabinet_domain::projection_work::ProjectionWorkIdentity,
    ) -> Result<
        Option<cabinet_domain::projection_work::ProjectionWork>,
        ProjectionWorkRepositoryError,
    > {
        Ok(self
            .works
            .iter()
            .find(|work| work.identity() == identity)
            .cloned())
    }
    fn replace(
        &mut self,
        work: cabinet_domain::projection_work::ProjectionWork,
        expected: cabinet_domain::projection_work::ProjectionWorkState,
    ) -> Result<(), ProjectionWorkRepositoryError> {
        let current = self
            .works
            .iter_mut()
            .find(|candidate| candidate.identity() == work.identity())
            .ok_or(ProjectionWorkRepositoryError::NotFound)?;
        if current.state() != expected {
            return Err(ProjectionWorkRepositoryError::Conflict);
        }
        *current = work;
        Ok(())
    }
    fn list_resumable(
        &self,
        _: usize,
    ) -> Result<Vec<cabinet_domain::projection_work::ProjectionWork>, ProjectionWorkRepositoryError>
    {
        Ok(self.works.clone())
    }
}
