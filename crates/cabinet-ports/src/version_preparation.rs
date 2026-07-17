use cabinet_domain::document_revision::DocumentOperationId;
use cabinet_domain::workspace::WorkspaceId;

use crate::version_store::VersionRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedVersion {
    operation_id: DocumentOperationId,
    record: VersionRecord,
}

impl PreparedVersion {
    pub const fn new(operation_id: DocumentOperationId, record: VersionRecord) -> Self {
        Self {
            operation_id,
            record,
        }
    }

    pub const fn operation_id(&self) -> &DocumentOperationId {
        &self.operation_id
    }

    pub const fn record(&self) -> &VersionRecord {
        &self.record
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionPreparationOutcomeKind {
    Prepared,
    Existing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionPreparationOutcome {
    Prepared(PreparedVersion),
    Existing(PreparedVersion),
}

impl VersionPreparationOutcome {
    pub const fn kind(&self) -> VersionPreparationOutcomeKind {
        match self {
            Self::Prepared(_) => VersionPreparationOutcomeKind::Prepared,
            Self::Existing(_) => VersionPreparationOutcomeKind::Existing,
        }
    }

    pub const fn prepared_version(&self) -> &PreparedVersion {
        match self {
            Self::Prepared(prepared) | Self::Existing(prepared) => prepared,
        }
    }
}

pub trait VersionPreparationPort {
    fn prepare_version(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &DocumentOperationId,
        record: VersionRecord,
    ) -> Result<VersionPreparationOutcome, VersionPreparationError>;

    fn load_prepared(
        &self,
        workspace_id: &WorkspaceId,
        operation_id: &DocumentOperationId,
    ) -> Result<Option<PreparedVersion>, VersionPreparationError>;

    fn discard_prepared(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &DocumentOperationId,
    ) -> Result<(), VersionPreparationError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionPreparationError {
    InvalidRecord,
    Conflict,
    StorageUnavailable,
    CorruptedPrepared,
}

impl VersionPreparationError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidRecord => "version_preparation.invalid_record",
            Self::Conflict => "version_preparation.conflict",
            Self::StorageUnavailable => "version_preparation.storage_unavailable",
            Self::CorruptedPrepared => "version_preparation.corrupted",
        }
    }
}
