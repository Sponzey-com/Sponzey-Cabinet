use std::collections::BTreeMap;

use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_availability::{
    AssetAvailability, AssetAvailabilityBatchResolver, AssetAvailabilityResolveError,
};

use crate::attachment_diff::{AttachmentDiff, AttachmentLabelChange};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveAttachmentDiffAvailabilityInput {
    workspace_id: String,
    diff: AttachmentDiff,
}

impl ResolveAttachmentDiffAvailabilityInput {
    pub fn new(workspace_id: &str, diff: AttachmentDiff) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            diff,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedAttachmentDiff {
    Known(ResolvedKnownAttachmentDiff),
    LegacyUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedKnownAttachmentDiff {
    added: Vec<ResolvedAttachmentReference>,
    removed: Vec<ResolvedAttachmentReference>,
    relabeled: Vec<ResolvedAttachmentLabelChange>,
    unchanged_count: usize,
}

impl ResolvedKnownAttachmentDiff {
    pub fn added(&self) -> &[ResolvedAttachmentReference] {
        &self.added
    }

    pub fn removed(&self) -> &[ResolvedAttachmentReference] {
        &self.removed
    }

    pub fn relabeled(&self) -> &[ResolvedAttachmentLabelChange] {
        &self.relabeled
    }

    pub const fn unchanged_count(&self) -> usize {
        self.unchanged_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAttachmentReference {
    reference: AssetReference,
    availability: AssetAvailability,
}

impl ResolvedAttachmentReference {
    pub const fn reference(&self) -> &AssetReference {
        &self.reference
    }

    pub const fn availability(&self) -> AssetAvailability {
        self.availability
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAttachmentLabelChange {
    change: AttachmentLabelChange,
    availability: AssetAvailability,
}

impl ResolvedAttachmentLabelChange {
    pub fn before_label(&self) -> &str {
        self.change.before_label()
    }

    pub fn after_label(&self) -> &str {
        self.change.after_label()
    }

    pub const fn availability(&self) -> AssetAvailability {
        self.availability
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveAttachmentDiffAvailabilityError {
    InvalidInput,
    StorageUnavailable,
    CorruptedData,
}

impl ResolveAttachmentDiffAvailabilityError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "resolve_attachment_diff.invalid_input",
            Self::StorageUnavailable => "resolve_attachment_diff.storage_unavailable",
            Self::CorruptedData => "resolve_attachment_diff.corrupted_data",
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ResolveAttachmentDiffAvailabilityUsecase;

impl ResolveAttachmentDiffAvailabilityUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ResolveAttachmentDiffAvailabilityInput,
        resolver: &impl AssetAvailabilityBatchResolver,
    ) -> Result<ResolvedAttachmentDiff, ResolveAttachmentDiffAvailabilityError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ResolveAttachmentDiffAvailabilityError::InvalidInput)?;
        let AttachmentDiff::Known(diff) = input.diff else {
            return Ok(ResolvedAttachmentDiff::LegacyUnknown);
        };

        let mut requested = BTreeMap::<String, AssetId>::new();
        for reference in diff.added().iter().chain(diff.removed()) {
            requested.insert(
                reference.asset_id().as_str().to_string(),
                reference.asset_id().clone(),
            );
        }
        for change in diff.relabeled() {
            requested.insert(
                change.asset_id().as_str().to_string(),
                change.asset_id().clone(),
            );
        }
        if requested.is_empty() {
            return Ok(ResolvedAttachmentDiff::Known(ResolvedKnownAttachmentDiff {
                added: Vec::new(),
                removed: Vec::new(),
                relabeled: Vec::new(),
                unchanged_count: diff.unchanged_count(),
            }));
        }

        let requested_ids = requested.values().cloned().collect::<Vec<_>>();
        let records = resolver
            .resolve_batch(&workspace_id, &requested_ids)
            .map_err(map_resolver_error)?;
        let mut availability = BTreeMap::new();
        for record in records {
            let key = record.asset_id().as_str().to_string();
            if !requested.contains_key(&key)
                || availability.insert(key, record.availability()).is_some()
            {
                return Err(ResolveAttachmentDiffAvailabilityError::CorruptedData);
            }
        }
        if availability.len() != requested.len() {
            return Err(ResolveAttachmentDiffAvailabilityError::CorruptedData);
        }

        let resolve_reference = |reference: &AssetReference| {
            availability
                .get(reference.asset_id().as_str())
                .copied()
                .map(|value| ResolvedAttachmentReference {
                    reference: reference.clone(),
                    availability: value,
                })
                .ok_or(ResolveAttachmentDiffAvailabilityError::CorruptedData)
        };
        let added = diff
            .added()
            .iter()
            .map(resolve_reference)
            .collect::<Result<Vec<_>, _>>()?;
        let removed = diff
            .removed()
            .iter()
            .map(resolve_reference)
            .collect::<Result<Vec<_>, _>>()?;
        let relabeled = diff
            .relabeled()
            .iter()
            .map(|change| {
                availability
                    .get(change.asset_id().as_str())
                    .copied()
                    .map(|value| ResolvedAttachmentLabelChange {
                        change: change.clone(),
                        availability: value,
                    })
                    .ok_or(ResolveAttachmentDiffAvailabilityError::CorruptedData)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ResolvedAttachmentDiff::Known(ResolvedKnownAttachmentDiff {
            added,
            removed,
            relabeled,
            unchanged_count: diff.unchanged_count(),
        }))
    }
}

const fn map_resolver_error(
    error: AssetAvailabilityResolveError,
) -> ResolveAttachmentDiffAvailabilityError {
    match error {
        AssetAvailabilityResolveError::StorageUnavailable => {
            ResolveAttachmentDiffAvailabilityError::StorageUnavailable
        }
        AssetAvailabilityResolveError::CorruptedData => {
            ResolveAttachmentDiffAvailabilityError::CorruptedData
        }
    }
}
