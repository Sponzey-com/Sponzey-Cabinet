#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreProductEvent {
    Requested,
    Conflict,
    BlockedMissingAsset,
    PrimaryCommitted,
    Completed,
    RecoveryRequired,
    Failed,
}

impl RestoreProductEvent {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Requested => "document.restore.requested",
            Self::Conflict => "document.restore.conflict",
            Self::BlockedMissingAsset => "document.restore.blocked_missing_asset",
            Self::PrimaryCommitted => "document.restore.primary_committed",
            Self::Completed => "document.restore.completed",
            Self::RecoveryRequired => "document.restore.recovery_required",
            Self::Failed => "document.restore.failed",
        }
    }
}

pub trait RestoreProductLogger {
    fn write_restore_product(&mut self, event: RestoreProductEvent);
}

#[derive(Debug, Default)]
pub struct NoopRestoreProductLogger;

impl RestoreProductLogger for NoopRestoreProductLogger {
    fn write_restore_product(&mut self, _event: RestoreProductEvent) {}
}
