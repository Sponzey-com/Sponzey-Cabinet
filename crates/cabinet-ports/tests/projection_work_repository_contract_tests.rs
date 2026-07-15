use std::collections::BTreeMap;

use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_work::{
    ProjectionKind, ProjectionWork, ProjectionWorkIdentity, ProjectionWorkState,
};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::projection_work::{
    ProjectionEnqueueOutcome, ProjectionWorkRepository, ProjectionWorkRepositoryError,
};

#[test]
fn projection_work_port_exposes_idempotent_enqueue_guarded_replace_and_resume() {
    let mut repository = FakeProjectionWorkRepository::default();
    let work = ProjectionWork::pending(identity());

    assert_eq!(
        repository.enqueue(work.clone()).expect("enqueue"),
        ProjectionEnqueueOutcome::Enqueued
    );
    assert_eq!(
        repository.enqueue(work.clone()).expect("duplicate"),
        ProjectionEnqueueOutcome::AlreadyExists
    );
    assert_eq!(
        repository.list_resumable(10).expect("resume"),
        vec![work.clone()]
    );
    assert_eq!(repository.get(work.identity()).expect("get"), Some(work));
    assert_eq!(
        ProjectionWorkRepositoryError::CorruptedRecord.code(),
        "projection_work.corrupted"
    );
    assert_eq!(
        ProjectionWorkRepositoryError::UnsupportedSchema.code(),
        "projection_work.unsupported_schema"
    );
}

#[derive(Default)]
struct FakeProjectionWorkRepository {
    records: BTreeMap<String, ProjectionWork>,
}

impl ProjectionWorkRepository for FakeProjectionWorkRepository {
    fn enqueue(
        &mut self,
        work: ProjectionWork,
    ) -> Result<ProjectionEnqueueOutcome, ProjectionWorkRepositoryError> {
        let key = work.identity().idempotency_key();
        if self.records.contains_key(&key) {
            return Ok(ProjectionEnqueueOutcome::AlreadyExists);
        }
        self.records.insert(key, work);
        Ok(ProjectionEnqueueOutcome::Enqueued)
    }

    fn get(
        &self,
        identity: &ProjectionWorkIdentity,
    ) -> Result<Option<ProjectionWork>, ProjectionWorkRepositoryError> {
        Ok(self.records.get(&identity.idempotency_key()).cloned())
    }

    fn replace(
        &mut self,
        work: ProjectionWork,
        expected_state: ProjectionWorkState,
    ) -> Result<(), ProjectionWorkRepositoryError> {
        let key = work.identity().idempotency_key();
        let current = self
            .records
            .get(&key)
            .ok_or(ProjectionWorkRepositoryError::NotFound)?;
        if current.state() != expected_state {
            return Err(ProjectionWorkRepositoryError::Conflict);
        }
        self.records.insert(key, work);
        Ok(())
    }

    fn list_resumable(
        &self,
        limit: usize,
    ) -> Result<Vec<ProjectionWork>, ProjectionWorkRepositoryError> {
        if limit == 0 {
            return Err(ProjectionWorkRepositoryError::InvalidLimit);
        }
        Ok(self
            .records
            .values()
            .filter(|work| work.state().is_resumable())
            .take(limit)
            .cloned()
            .collect())
    }
}

fn identity() -> ProjectionWorkIdentity {
    ProjectionWorkIdentity::new(
        WorkspaceId::new("workspace-1").expect("workspace"),
        DocumentId::new("doc-1").expect("document"),
        VersionId::new("version-1").expect("version"),
        ProjectionKind::Graph,
    )
}
