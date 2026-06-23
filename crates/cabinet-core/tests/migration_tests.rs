use cabinet_core::migration::{
    MigrationError, MigrationErrorCode, MigrationEvent, MigrationPlan, MigrationProductEvent,
    MigrationRunner, MigrationState, MigrationStore, MigrationVersion, transition_migration,
};

#[derive(Debug, Default)]
struct FakeMigrationStore {
    applied_versions: Vec<MigrationVersion>,
    recorded_versions: Vec<MigrationVersion>,
    lock_acquired: bool,
    lock_released: bool,
}

impl FakeMigrationStore {
    fn with_applied(version: MigrationVersion) -> Self {
        Self {
            applied_versions: vec![version],
            recorded_versions: Vec::new(),
            lock_acquired: false,
            lock_released: false,
        }
    }
}

impl MigrationStore for FakeMigrationStore {
    fn acquire_lock(&mut self) -> Result<(), MigrationErrorCode> {
        self.lock_acquired = true;
        Ok(())
    }

    fn applied_versions(&mut self) -> Result<Vec<MigrationVersion>, MigrationErrorCode> {
        Ok(self.applied_versions.clone())
    }

    fn record_version(&mut self, version: MigrationVersion) -> Result<(), MigrationErrorCode> {
        self.recorded_versions.push(version);
        self.applied_versions.push(version);
        Ok(())
    }

    fn release_lock(&mut self) -> Result<(), MigrationErrorCode> {
        self.lock_released = true;
        Ok(())
    }
}

#[test]
fn migration_plan_contains_initial_noop_step() {
    let plan = MigrationPlan::initial();
    let steps = plan.steps();

    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].version, MigrationVersion::new(1));
    assert_eq!(steps[0].name, "initial_noop");
    assert_eq!(steps[0].is_noop, true);
}

#[test]
fn migration_transitions_to_completed_through_explicit_events() {
    let locked = transition_migration(MigrationState::NotStarted, MigrationEvent::AcquireLock)
        .expect("lock should transition");
    assert_eq!(locked.next_state, MigrationState::Locked);

    let running = transition_migration(locked.next_state, MigrationEvent::RunMigration)
        .expect("run should transition");
    assert_eq!(running.next_state, MigrationState::Running);

    let completed = transition_migration(running.next_state, MigrationEvent::MigrationSucceeded)
        .expect("success should transition");
    assert_eq!(completed.next_state, MigrationState::Completed);
}

#[test]
fn migration_rejects_invalid_transition() {
    let error = transition_migration(MigrationState::NotStarted, MigrationEvent::RunMigration)
        .expect_err("invalid transition should fail");

    assert_eq!(
        error,
        MigrationError::InvalidTransition {
            state: MigrationState::NotStarted,
            event: MigrationEvent::RunMigration,
        }
    );
}

#[test]
fn migration_failure_state_carries_error_code_and_retry_policy() {
    let failed = transition_migration(
        MigrationState::Running,
        MigrationEvent::MigrationFailed(MigrationErrorCode::VersionRecordFailed),
    )
    .expect("failure should transition");

    assert_eq!(
        failed.next_state,
        MigrationState::Failed {
            error_code: MigrationErrorCode::VersionRecordFailed,
            retryable: true,
        }
    );

    let retrying = transition_migration(failed.next_state, MigrationEvent::Retry)
        .expect("retryable failure should transition");
    assert_eq!(retrying.next_state, MigrationState::Retrying);
}

#[test]
fn migration_runner_records_initial_noop_version_when_missing() {
    let mut store = FakeMigrationStore::default();
    let runner = MigrationRunner::new(MigrationPlan::initial());

    let outcome = runner.run(&mut store);

    assert_eq!(outcome.final_state, MigrationState::Completed);
    assert_eq!(
        outcome.product_event,
        MigrationProductEvent::MigrationCompleted {
            applied_versions: vec![MigrationVersion::new(1)],
        }
    );
    assert_eq!(outcome.applied_versions, vec![MigrationVersion::new(1)]);
    assert_eq!(store.recorded_versions, vec![MigrationVersion::new(1)]);
    assert!(store.lock_acquired);
    assert!(store.lock_released);
}

#[test]
fn migration_runner_is_idempotent_when_initial_version_is_already_recorded() {
    let mut store = FakeMigrationStore::with_applied(MigrationVersion::new(1));
    let runner = MigrationRunner::new(MigrationPlan::initial());

    let outcome = runner.run(&mut store);

    assert_eq!(outcome.final_state, MigrationState::Completed);
    assert_eq!(
        outcome.product_event,
        MigrationProductEvent::MigrationCompleted {
            applied_versions: vec![],
        }
    );
    assert!(outcome.applied_versions.is_empty());
    assert!(store.recorded_versions.is_empty());
    assert!(store.lock_acquired);
    assert!(store.lock_released);
}
