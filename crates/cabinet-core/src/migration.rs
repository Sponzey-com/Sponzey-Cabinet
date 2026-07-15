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
            Err(error_code) => {
                let _ = store.release_lock();
                return failed_migration_outcome(state, error_code, Vec::new());
            }
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

        if store.release_lock().is_err() {
            return failed_migration_outcome(
                state,
                MigrationErrorCode::LockReleaseFailed,
                applied_versions,
            );
        }
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
    LockReleaseFailed,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase002FixtureRecordKind {
    Workspace,
    User,
    Group,
    GroupMembership,
    RoleAssignment,
    DocumentCurrentSnapshot,
    DocumentVersionHistory,
    DocumentSharingPolicy,
    CommentThread,
    InlineCommentAnchor,
    ReviewWorkflowState,
    DocumentLockState,
    AuditEvent,
    FieldDebugSession,
    ObjectMetadata,
    BackupJobRecord,
    ExportJobRecord,
}

impl Phase002FixtureRecordKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::User => "user",
            Self::Group => "group",
            Self::GroupMembership => "group_membership",
            Self::RoleAssignment => "role_assignment",
            Self::DocumentCurrentSnapshot => "document_current_snapshot",
            Self::DocumentVersionHistory => "document_version_history",
            Self::DocumentSharingPolicy => "document_sharing_policy",
            Self::CommentThread => "comment_thread",
            Self::InlineCommentAnchor => "inline_comment_anchor",
            Self::ReviewWorkflowState => "review_workflow_state",
            Self::DocumentLockState => "document_lock_state",
            Self::AuditEvent => "audit_event",
            Self::FieldDebugSession => "field_debug_session",
            Self::ObjectMetadata => "object_metadata",
            Self::BackupJobRecord => "backup_job_record",
            Self::ExportJobRecord => "export_job_record",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "workspace" => Self::Workspace,
            "user" => Self::User,
            "group" => Self::Group,
            "group_membership" => Self::GroupMembership,
            "role_assignment" => Self::RoleAssignment,
            "document_current_snapshot" => Self::DocumentCurrentSnapshot,
            "document_version_history" => Self::DocumentVersionHistory,
            "document_sharing_policy" => Self::DocumentSharingPolicy,
            "comment_thread" => Self::CommentThread,
            "inline_comment_anchor" => Self::InlineCommentAnchor,
            "review_workflow_state" => Self::ReviewWorkflowState,
            "document_lock_state" => Self::DocumentLockState,
            "audit_event" => Self::AuditEvent,
            "field_debug_session" => Self::FieldDebugSession,
            "object_metadata" => Self::ObjectMetadata,
            "backup_job_record" => Self::BackupJobRecord,
            "export_job_record" => Self::ExportJobRecord,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phase002FixtureRecord {
    kind: Phase002FixtureRecordKind,
    id: String,
    public_fields: Vec<(String, String)>,
    sensitive_payload: Option<String>,
}

impl Phase002FixtureRecord {
    pub fn new(
        kind: Phase002FixtureRecordKind,
        id: &str,
        public_fields: Vec<(&str, &str)>,
        sensitive_payload: Option<&str>,
    ) -> Result<Self, Phase002FixtureError> {
        let id = validate_fixture_text(id)?;
        let public_fields = public_fields
            .into_iter()
            .map(|(key, value)| Ok((validate_fixture_text(key)?, validate_fixture_text(value)?)))
            .collect::<Result<Vec<_>, Phase002FixtureError>>()?;
        let sensitive_payload = sensitive_payload.map(validate_fixture_text).transpose()?;
        Ok(Self {
            kind,
            id,
            public_fields,
            sensitive_payload,
        })
    }

    pub const fn kind(&self) -> Phase002FixtureRecordKind {
        self.kind
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn public_fields(&self) -> &[(String, String)] {
        &self.public_fields
    }

    pub fn sensitive_payload(&self) -> Option<&str> {
        self.sensitive_payload.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phase002MigrationFixture {
    records: Vec<Phase002FixtureRecord>,
}

impl Phase002MigrationFixture {
    pub fn self_host_sample() -> Self {
        Self::new(vec![
            record(
                Phase002FixtureRecordKind::Workspace,
                "workspace-1",
                vec![("name", "Phase 002 Workspace")],
                None,
            ),
            record(
                Phase002FixtureRecordKind::User,
                "user-owner",
                vec![("login", "owner@example.test")],
                Some("phase002-token-fixture-should-not-log"),
            ),
            record(
                Phase002FixtureRecordKind::Group,
                "group-editors",
                vec![("workspaceId", "workspace-1")],
                None,
            ),
            record(
                Phase002FixtureRecordKind::GroupMembership,
                "membership-1",
                vec![("groupId", "group-editors"), ("userId", "user-owner")],
                None,
            ),
            record(
                Phase002FixtureRecordKind::RoleAssignment,
                "role-assignment-1",
                vec![("subject", "group:group-editors"), ("role", "editor")],
                None,
            ),
            record(
                Phase002FixtureRecordKind::DocumentCurrentSnapshot,
                "doc-1-current",
                vec![("documentId", "doc-1"), ("versionId", "version-2")],
                Some("phase002 document body fixture should not be logged"),
            ),
            record(
                Phase002FixtureRecordKind::DocumentVersionHistory,
                "doc-1-history",
                vec![("documentId", "doc-1"), ("versionCount", "2")],
                None,
            ),
            record(
                Phase002FixtureRecordKind::DocumentSharingPolicy,
                "sharing-doc-1",
                vec![("documentId", "doc-1"), ("permission", "read")],
                None,
            ),
            record(
                Phase002FixtureRecordKind::CommentThread,
                "comment-thread-1",
                vec![("documentId", "doc-1"), ("state", "open")],
                Some("phase002 comment body fixture should not be logged"),
            ),
            record(
                Phase002FixtureRecordKind::InlineCommentAnchor,
                "inline-anchor-1",
                vec![("versionId", "version-2"), ("range", "10:20")],
                None,
            ),
            record(
                Phase002FixtureRecordKind::ReviewWorkflowState,
                "review-1",
                vec![("documentId", "doc-1"), ("state", "approved")],
                None,
            ),
            record(
                Phase002FixtureRecordKind::DocumentLockState,
                "lock-doc-1",
                vec![("documentId", "doc-1"), ("state", "unlocked")],
                None,
            ),
            record(
                Phase002FixtureRecordKind::AuditEvent,
                "audit-1",
                vec![("event", "document.publish.completed")],
                None,
            ),
            record(
                Phase002FixtureRecordKind::FieldDebugSession,
                "field-debug-1",
                vec![("scope", "workspace:workspace-1"), ("state", "expired")],
                Some("phase002-secret-fixture-should-not-log"),
            ),
            record(
                Phase002FixtureRecordKind::ObjectMetadata,
                "object-1",
                vec![("key", "sha256:phase002-object"), ("byteSize", "42")],
                Some("phase002 asset content fixture should not be logged"),
            ),
            record(
                Phase002FixtureRecordKind::BackupJobRecord,
                "backup-job-1",
                vec![("state", "completed")],
                None,
            ),
            record(
                Phase002FixtureRecordKind::ExportJobRecord,
                "export-job-1",
                vec![("state", "completed")],
                None,
            ),
        ])
        .expect("static Phase 002 fixture must be valid")
    }

    pub fn new(records: Vec<Phase002FixtureRecord>) -> Result<Self, Phase002FixtureError> {
        let fixture = Self { records };
        fixture.validate()?;
        Ok(fixture)
    }

    pub fn records(&self) -> &[Phase002FixtureRecord] {
        &self.records
    }

    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    pub fn contains_kind(&self, kind: Phase002FixtureRecordKind) -> bool {
        self.records.iter().any(|record| record.kind() == kind)
    }

    pub fn sensitive_values(&self) -> Vec<&str> {
        self.records
            .iter()
            .filter_map(Phase002FixtureRecord::sensitive_payload)
            .collect()
    }

    pub fn validate(&self) -> Result<(), Phase002FixtureError> {
        if self.records.is_empty() {
            return Err(Phase002FixtureError::EmptyFixture);
        }
        for kind in required_phase002_fixture_kinds() {
            if !self.contains_kind(*kind) {
                return Err(Phase002FixtureError::MissingRequiredRecordKind(*kind));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase002FixtureError {
    EmptyFixture,
    EmptyText,
    ControlCharacter,
    MissingRequiredRecordKind(Phase002FixtureRecordKind),
    CorruptedRecord,
}

pub fn required_phase002_fixture_kinds() -> &'static [Phase002FixtureRecordKind] {
    &[
        Phase002FixtureRecordKind::Workspace,
        Phase002FixtureRecordKind::User,
        Phase002FixtureRecordKind::Group,
        Phase002FixtureRecordKind::GroupMembership,
        Phase002FixtureRecordKind::RoleAssignment,
        Phase002FixtureRecordKind::DocumentCurrentSnapshot,
        Phase002FixtureRecordKind::DocumentVersionHistory,
        Phase002FixtureRecordKind::DocumentSharingPolicy,
        Phase002FixtureRecordKind::CommentThread,
        Phase002FixtureRecordKind::InlineCommentAnchor,
        Phase002FixtureRecordKind::ReviewWorkflowState,
        Phase002FixtureRecordKind::DocumentLockState,
        Phase002FixtureRecordKind::AuditEvent,
        Phase002FixtureRecordKind::FieldDebugSession,
        Phase002FixtureRecordKind::ObjectMetadata,
        Phase002FixtureRecordKind::BackupJobRecord,
        Phase002FixtureRecordKind::ExportJobRecord,
    ]
}

fn record(
    kind: Phase002FixtureRecordKind,
    id: &str,
    public_fields: Vec<(&str, &str)>,
    sensitive_payload: Option<&str>,
) -> Phase002FixtureRecord {
    Phase002FixtureRecord::new(kind, id, public_fields, sensitive_payload)
        .expect("static Phase 002 fixture record must be valid")
}

fn validate_fixture_text(value: &str) -> Result<String, Phase002FixtureError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Phase002FixtureError::EmptyText);
    }
    if trimmed.chars().any(char::is_control) {
        return Err(Phase002FixtureError::ControlCharacter);
    }
    Ok(trimmed.to_string())
}
