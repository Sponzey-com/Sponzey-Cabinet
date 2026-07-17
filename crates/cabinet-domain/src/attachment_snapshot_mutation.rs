use crate::asset::{AssetId, AssetReference};
use crate::version::AttachmentSnapshotState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentSnapshotMutation {
    Link(AssetReference),
    Unlink(AssetId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentSnapshotDelta {
    Linked,
    Relabeled,
    Unlinked,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentSnapshotMutationOutcome {
    state: AttachmentSnapshotState,
    delta: AttachmentSnapshotDelta,
}

impl AttachmentSnapshotMutationOutcome {
    pub const fn state(&self) -> &AttachmentSnapshotState {
        &self.state
    }

    pub const fn delta(&self) -> AttachmentSnapshotDelta {
        self.delta
    }

    pub const fn changed(&self) -> bool {
        !matches!(self.delta, AttachmentSnapshotDelta::Unchanged)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentSnapshotMutationError {
    LegacyBaselineRequired,
    InvalidBaseline,
}

impl AttachmentSnapshotMutationError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::LegacyBaselineRequired => "attachment_snapshot_mutation.legacy_baseline_required",
            Self::InvalidBaseline => "attachment_snapshot_mutation.invalid_baseline",
        }
    }
}

pub fn transition_attachment_snapshot(
    current: &AttachmentSnapshotState,
    mutation: AttachmentSnapshotMutation,
    legacy_baseline: Option<Vec<AssetReference>>,
) -> Result<AttachmentSnapshotMutationOutcome, AttachmentSnapshotMutationError> {
    let mut references = match current.references() {
        Some(references) => references.to_vec(),
        None => legacy_baseline.ok_or(AttachmentSnapshotMutationError::LegacyBaselineRequired)?,
    };
    validate_references(&references)?;

    let delta = match mutation {
        AttachmentSnapshotMutation::Link(next) => match references
            .iter_mut()
            .find(|reference| reference.asset_id() == next.asset_id())
        {
            Some(existing) if existing.label() == next.label() => {
                AttachmentSnapshotDelta::Unchanged
            }
            Some(existing) => {
                *existing = next;
                AttachmentSnapshotDelta::Relabeled
            }
            None => {
                references.push(next);
                AttachmentSnapshotDelta::Linked
            }
        },
        AttachmentSnapshotMutation::Unlink(asset_id) => {
            let previous_len = references.len();
            references.retain(|reference| reference.asset_id() != &asset_id);
            if references.len() == previous_len {
                AttachmentSnapshotDelta::Unchanged
            } else {
                AttachmentSnapshotDelta::Unlinked
            }
        }
    };

    let state = AttachmentSnapshotState::known(references)
        .map_err(|_| AttachmentSnapshotMutationError::InvalidBaseline)?;
    Ok(AttachmentSnapshotMutationOutcome { state, delta })
}

fn validate_references(
    references: &[AssetReference],
) -> Result<(), AttachmentSnapshotMutationError> {
    AttachmentSnapshotState::known(references.to_vec())
        .map(|_| ())
        .map_err(|_| AttachmentSnapshotMutationError::InvalidBaseline)
}
