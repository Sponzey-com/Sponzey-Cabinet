use cabinet_core::migration::{
    MigrationError, MigrationErrorCode, MigrationEvent, MigrationPlan, MigrationProductEvent,
    MigrationRunner, MigrationState, MigrationStore, MigrationVersion, Phase002FixtureError,
    Phase002FixtureRecord, Phase002FixtureRecordKind, Phase002MigrationFixture,
    required_phase002_fixture_kinds, transition_migration,
};

#[derive(Debug, Default)]
struct FakeMigrationStore {
    applied_versions: Vec<MigrationVersion>,
    recorded_versions: Vec<MigrationVersion>,
    lock_acquired: bool,
    lock_released: bool,
    version_read_error: Option<MigrationErrorCode>,
    version_record_error: Option<MigrationErrorCode>,
    lock_release_error: Option<MigrationErrorCode>,
}

impl FakeMigrationStore {
    fn with_applied(version: MigrationVersion) -> Self {
        Self {
            applied_versions: vec![version],
            recorded_versions: Vec::new(),
            lock_acquired: false,
            lock_released: false,
            version_read_error: None,
            version_record_error: None,
            lock_release_error: None,
        }
    }

    fn failing_version_read() -> Self {
        Self {
            version_read_error: Some(MigrationErrorCode::VersionReadFailed),
            ..Self::default()
        }
    }

    fn failing_version_record() -> Self {
        Self {
            version_record_error: Some(MigrationErrorCode::VersionRecordFailed),
            ..Self::default()
        }
    }
}

impl MigrationStore for FakeMigrationStore {
    fn acquire_lock(&mut self) -> Result<(), MigrationErrorCode> {
        self.lock_acquired = true;
        Ok(())
    }

    fn applied_versions(&mut self) -> Result<Vec<MigrationVersion>, MigrationErrorCode> {
        if let Some(error) = self.version_read_error {
            return Err(error);
        }
        Ok(self.applied_versions.clone())
    }

    fn record_version(&mut self, version: MigrationVersion) -> Result<(), MigrationErrorCode> {
        if let Some(error) = self.version_record_error {
            return Err(error);
        }
        self.recorded_versions.push(version);
        self.applied_versions.push(version);
        Ok(())
    }

    fn release_lock(&mut self) -> Result<(), MigrationErrorCode> {
        self.lock_released = true;
        self.lock_release_error.map_or(Ok(()), Err)
    }
}

#[test]
fn migration_runner_releases_lock_when_version_read_fails() {
    let mut store = FakeMigrationStore::failing_version_read();
    let runner = MigrationRunner::new(MigrationPlan::initial());

    let outcome = runner.run(&mut store);

    assert_eq!(
        outcome.final_state,
        MigrationState::Failed {
            error_code: MigrationErrorCode::VersionReadFailed,
            retryable: true,
        }
    );
    assert_eq!(
        outcome.product_event,
        MigrationProductEvent::MigrationFailed {
            error_code: MigrationErrorCode::VersionReadFailed,
        }
    );
    assert!(store.lock_acquired);
    assert!(store.lock_released);
}

#[test]
fn migration_runner_releases_lock_when_version_record_fails() {
    let mut store = FakeMigrationStore::failing_version_record();
    let runner = MigrationRunner::new(MigrationPlan::initial());

    let outcome = runner.run(&mut store);

    assert_eq!(
        outcome.final_state,
        MigrationState::Failed {
            error_code: MigrationErrorCode::VersionRecordFailed,
            retryable: true,
        }
    );
    assert!(store.lock_acquired);
    assert!(store.lock_released);
}

#[test]
fn migration_runner_reports_lock_release_failure_after_versions_are_recorded() {
    let mut store = FakeMigrationStore {
        lock_release_error: Some(MigrationErrorCode::LockReleaseFailed),
        ..FakeMigrationStore::default()
    };
    let runner = MigrationRunner::new(MigrationPlan::initial());

    let outcome = runner.run(&mut store);

    assert_eq!(
        outcome.final_state,
        MigrationState::Failed {
            error_code: MigrationErrorCode::LockReleaseFailed,
            retryable: true,
        }
    );
    assert_eq!(
        outcome.product_event,
        MigrationProductEvent::MigrationFailed {
            error_code: MigrationErrorCode::LockReleaseFailed,
        }
    );
    assert_eq!(outcome.applied_versions, vec![MigrationVersion::new(1)]);
    assert!(store.lock_released);
}

#[test]
fn migration_runner_preserves_original_failure_when_cleanup_also_fails() {
    let mut store = FakeMigrationStore {
        version_read_error: Some(MigrationErrorCode::VersionReadFailed),
        lock_release_error: Some(MigrationErrorCode::LockReleaseFailed),
        ..FakeMigrationStore::default()
    };
    let runner = MigrationRunner::new(MigrationPlan::initial());

    let outcome = runner.run(&mut store);

    assert_eq!(
        outcome.product_event,
        MigrationProductEvent::MigrationFailed {
            error_code: MigrationErrorCode::VersionReadFailed,
        }
    );
    assert!(store.lock_released, "cleanup must still be attempted");
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

#[test]
fn phase002_self_host_fixture_contains_required_runtime_data_categories() {
    let fixture = Phase002MigrationFixture::self_host_sample();

    for kind in required_phase002_fixture_kinds() {
        assert!(fixture.contains_kind(*kind), "missing {kind:?}");
    }
    assert_eq!(
        fixture.record_count(),
        required_phase002_fixture_kinds().len()
    );
    assert!(
        fixture
            .sensitive_values()
            .iter()
            .any(|value| value.contains("document body"))
    );
    assert!(
        fixture
            .sensitive_values()
            .iter()
            .any(|value| value.contains("comment body"))
    );
    assert!(
        fixture
            .sensitive_values()
            .iter()
            .any(|value| value.contains("token"))
    );
    assert!(
        fixture
            .sensitive_values()
            .iter()
            .any(|value| value.contains("secret"))
    );
    assert!(
        fixture
            .sensitive_values()
            .iter()
            .any(|value| value.contains("asset content"))
    );
}

#[test]
fn phase002_fixture_rejects_missing_required_record_kind() {
    let fixture = Phase002MigrationFixture::new(vec![
        Phase002FixtureRecord::new(
            Phase002FixtureRecordKind::Workspace,
            "workspace-1",
            vec![("name", "Only Workspace")],
            None,
        )
        .expect("record"),
    ])
    .expect_err("incomplete fixture must fail");

    assert_eq!(
        fixture,
        Phase002FixtureError::MissingRequiredRecordKind(Phase002FixtureRecordKind::User)
    );
}

#[test]
fn migration_product_event_never_contains_phase002_sensitive_fixture_values() {
    let fixture = Phase002MigrationFixture::self_host_sample();
    let product_event = MigrationProductEvent::MigrationCompleted {
        applied_versions: vec![MigrationVersion::new(1)],
    };
    let rendered = format!("{product_event:?}");

    for sensitive in fixture.sensitive_values() {
        assert!(!rendered.contains(sensitive));
    }
}
