use cabinet_domain::canvas::{CanvasId, CanvasRevision};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_recovery::{CanvasRecoveryRepository, CanvasRecoveryRepositoryError};
use cabinet_usecases::canvas_recovery::{
    CanvasRecoveryError, CanvasRecoveryEvent, CanvasRecoveryLogger, RecoverCanvasInput,
    RecoverCanvasUsecase,
};

#[test]
fn recovery_selects_latest_valid_revision_and_logs_only_identity() {
    let mut repository = FakeRecoveryRepository {
        candidates: vec![revision(2), revision(5), revision(3)],
        ..FakeRecoveryRepository::default()
    };
    let mut logger = RecordingLogger::default();

    let output = RecoverCanvasUsecase::new(32)
        .expect("policy")
        .execute(
            RecoverCanvasInput::new("workspace-1", "canvas-1"),
            &mut repository,
            &mut logger,
        )
        .expect("recover");

    assert_eq!(output.revision().value(), 5);
    assert_eq!(repository.activated, Some(revision(5)));
    assert_eq!(repository.observed_limit, Some(32));
    assert_eq!(logger.events[0].event_name(), "canvas.recovery.completed");
    assert_eq!(logger.events[0].canvas_id(), "canvas-1");
    assert_eq!(logger.events[0].revision(), Some(5));
    assert_eq!(logger.events[0].error_code(), None);
    assert!(!format!("{:?}", logger.events[0]).contains("path"));
}

#[test]
fn recovery_without_valid_revision_fails_without_writing() {
    let mut repository = FakeRecoveryRepository::default();
    let mut logger = RecordingLogger::default();

    assert_eq!(
        RecoverCanvasUsecase::new(8).expect("policy").execute(
            RecoverCanvasInput::new("workspace-1", "canvas-1"),
            &mut repository,
            &mut logger,
        ),
        Err(CanvasRecoveryError::NoValidRevision)
    );
    assert_eq!(repository.activated, None);
    assert_eq!(logger.events.len(), 1);
    assert_eq!(logger.events[0].event_name(), "canvas.recovery.failed");
    assert_eq!(
        logger.events[0].error_code(),
        Some("CANVAS_RECOVERY_NO_VALID_REVISION")
    );
    assert_eq!(logger.events[0].revision(), None);
}

#[derive(Default)]
struct FakeRecoveryRepository {
    candidates: Vec<CanvasRevision>,
    activated: Option<CanvasRevision>,
    observed_limit: Option<usize>,
}

impl CanvasRecoveryRepository for FakeRecoveryRepository {
    fn list_valid_revisions(
        &mut self,
        _: &WorkspaceId,
        _: &CanvasId,
        limit: usize,
    ) -> Result<Vec<CanvasRevision>, CanvasRecoveryRepositoryError> {
        self.observed_limit = Some(limit);
        Ok(self.candidates.clone())
    }

    fn activate_revision(
        &mut self,
        _: &WorkspaceId,
        _: &CanvasId,
        revision: CanvasRevision,
    ) -> Result<(), CanvasRecoveryRepositoryError> {
        self.activated = Some(revision);
        Ok(())
    }
}

#[derive(Default)]
struct RecordingLogger {
    events: Vec<CanvasRecoveryEvent>,
}

impl CanvasRecoveryLogger for RecordingLogger {
    fn write_product(&mut self, event: CanvasRecoveryEvent) {
        self.events.push(event);
    }
}

fn revision(value: u64) -> CanvasRevision {
    CanvasRevision::new(value).expect("revision")
}
