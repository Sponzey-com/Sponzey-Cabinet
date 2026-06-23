#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MigrationVersion(u32);

impl MigrationVersion {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationStep {
    pub version: MigrationVersion,
    pub name: &'static str,
    pub is_noop: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationPlan {
    steps: Vec<MigrationStep>,
}

impl MigrationPlan {
    pub fn initial() -> Self {
        Self {
            steps: vec![MigrationStep {
                version: MigrationVersion::new(1),
                name: "initial_noop",
                is_noop: true,
            }],
        }
    }

    pub fn steps(&self) -> &[MigrationStep] {
        &self.steps
    }
}

pub trait MigrationStore {
    fn acquire_lock(&mut self) -> Result<(), MigrationErrorCode>;
    fn applied_versions(&mut self) -> Result<Vec<MigrationVersion>, MigrationErrorCode>;
    fn record_version(&mut self, version: MigrationVersion) -> Result<(), MigrationErrorCode>;
    fn release_lock(&mut self) -> Result<(), MigrationErrorCode>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRunner {
    plan: MigrationPlan,
}

impl MigrationRunner {
    pub fn new(plan: MigrationPlan) -> Self {
        Self { plan }
    }

    pub fn run<S: MigrationStore>(&self, store: &mut S) -> MigrationOutcome {
        let mut state = MigrationState::NotStarted;
        if store.acquire_lock().is_err() {
            return failed_migration_outcome(
                state,
                MigrationErrorCode::LockAcquireFailed,
                Vec::new(),
            );
        }
        state = transition_or_failed(
            state,
            MigrationEvent::AcquireLock,
            MigrationErrorCode::LockAcquireFailed,
        );

        let already_applied = match store.applied_versions() {
            Ok(versions) => versions,
            Err(error_code) => return failed_migration_outcome(state, error_code, Vec::new()),
        };

        state = transition_or_failed(
            state,
            MigrationEvent::RunMigration,
            MigrationErrorCode::VersionReadFailed,
        );

        let mut applied_versions = Vec::new();
        for step in self.plan.steps() {
            if already_applied.contains(&step.version) {
                continue;
            }

            if store.record_version(step.version).is_err() {
                let _ = store.release_lock();
                return failed_migration_outcome(
                    state,
                    MigrationErrorCode::VersionRecordFailed,
                    applied_versions,
                );
            }
            applied_versions.push(step.version);
        }

        let _ = store.release_lock();
        state = transition_or_failed(
            state,
            MigrationEvent::MigrationSucceeded,
            MigrationErrorCode::VersionRecordFailed,
        );

        MigrationOutcome {
            final_state: state,
            product_event: MigrationProductEvent::MigrationCompleted {
                applied_versions: applied_versions.clone(),
            },
            applied_versions,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationState {
    NotStarted,
    Locked,
    Running,
    Completed,
    Failed {
        error_code: MigrationErrorCode,
        retryable: bool,
    },
    Retrying,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationEvent {
    AcquireLock,
    RunMigration,
    MigrationSucceeded,
    MigrationFailed(MigrationErrorCode),
    Retry,
    ReleaseLock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationErrorCode {
    LockAcquireFailed,
    VersionReadFailed,
    VersionRecordFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationTransition {
    pub previous_state: MigrationState,
    pub event: MigrationEvent,
    pub next_state: MigrationState,
    pub retryable: bool,
    pub error_code: Option<MigrationErrorCode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationError {
    InvalidTransition {
        state: MigrationState,
        event: MigrationEvent,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationProductEvent {
    MigrationCompleted {
        applied_versions: Vec<MigrationVersion>,
    },
    MigrationFailed {
        error_code: MigrationErrorCode,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationOutcome {
    pub final_state: MigrationState,
    pub product_event: MigrationProductEvent,
    pub applied_versions: Vec<MigrationVersion>,
}

pub fn transition_migration(
    state: MigrationState,
    event: MigrationEvent,
) -> Result<MigrationTransition, MigrationError> {
    let next_state = match (state, event) {
        (MigrationState::NotStarted, MigrationEvent::AcquireLock) => MigrationState::Locked,
        (MigrationState::Retrying, MigrationEvent::AcquireLock) => MigrationState::Locked,
        (MigrationState::Locked, MigrationEvent::RunMigration) => MigrationState::Running,
        (MigrationState::Running, MigrationEvent::MigrationSucceeded) => MigrationState::Completed,
        (MigrationState::Completed, MigrationEvent::ReleaseLock) => MigrationState::Completed,
        (
            MigrationState::NotStarted
            | MigrationState::Locked
            | MigrationState::Running
            | MigrationState::Retrying,
            MigrationEvent::MigrationFailed(error_code),
        ) => MigrationState::Failed {
            error_code,
            retryable: true,
        },
        (
            MigrationState::Failed {
                retryable: true, ..
            },
            MigrationEvent::Retry,
        ) => MigrationState::Retrying,
        _ => return Err(MigrationError::InvalidTransition { state, event }),
    };

    let (retryable, error_code) = match next_state {
        MigrationState::Failed {
            error_code,
            retryable,
        } => (retryable, Some(error_code)),
        _ => (false, None),
    };

    Ok(MigrationTransition {
        previous_state: state,
        event,
        next_state,
        retryable,
        error_code,
    })
}

fn transition_or_failed(
    state: MigrationState,
    event: MigrationEvent,
    fallback_error_code: MigrationErrorCode,
) -> MigrationState {
    transition_migration(state, event)
        .map(|transition| transition.next_state)
        .unwrap_or(MigrationState::Failed {
            error_code: fallback_error_code,
            retryable: true,
        })
}

fn failed_migration_outcome(
    state: MigrationState,
    error_code: MigrationErrorCode,
    applied_versions: Vec<MigrationVersion>,
) -> MigrationOutcome {
    let failed_state = transition_migration(state, MigrationEvent::MigrationFailed(error_code))
        .map(|transition| transition.next_state)
        .unwrap_or(MigrationState::Failed {
            error_code,
            retryable: true,
        });

    MigrationOutcome {
        final_state: failed_state,
        product_event: MigrationProductEvent::MigrationFailed { error_code },
        applied_versions,
    }
}
