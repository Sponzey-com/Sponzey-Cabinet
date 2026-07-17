use std::collections::BTreeMap;

use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::version::AttachmentSnapshotState;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_availability::{
    AssetAvailability, AssetAvailabilityBatchResolver, AssetAvailabilityResolveError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreTargetAssetPreflightInput {
    workspace_id: String,
    attachment_state: AttachmentSnapshotState,
}

impl RestoreTargetAssetPreflightInput {
    pub fn new(workspace_id: &str, attachment_state: AttachmentSnapshotState) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            attachment_state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreTargetAssetPreflightOutcome {
    Available,
    LegacyPreserved,
    BlockedMissingAssets(Vec<AssetReference>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreTargetAssetPreflightError {
    InvalidInput,
    StorageUnavailable,
    CorruptedData,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RestoreTargetAssetPreflightUsecase;

impl RestoreTargetAssetPreflightUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: RestoreTargetAssetPreflightInput,
        resolver: &impl AssetAvailabilityBatchResolver,
    ) -> Result<RestoreTargetAssetPreflightOutcome, RestoreTargetAssetPreflightError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| RestoreTargetAssetPreflightError::InvalidInput)?;
        let Some(references) = input.attachment_state.references() else {
            return Ok(RestoreTargetAssetPreflightOutcome::LegacyPreserved);
        };
        if references.is_empty() {
            return Ok(RestoreTargetAssetPreflightOutcome::Available);
        }
        let requested = references
            .iter()
            .map(|reference| {
                (
                    reference.asset_id().as_str().to_string(),
                    reference.asset_id().clone(),
                )
            })
            .collect::<BTreeMap<String, AssetId>>();
        let ids = requested.values().cloned().collect::<Vec<_>>();
        let records = resolver
            .resolve_batch(&workspace_id, &ids)
            .map_err(map_resolver_error)?;
        let mut availability = BTreeMap::new();
        for record in records {
            let key = record.asset_id().as_str().to_string();
            if !requested.contains_key(&key)
                || availability.insert(key, record.availability()).is_some()
            {
                return Err(RestoreTargetAssetPreflightError::CorruptedData);
            }
        }
        if availability.len() != requested.len() {
            return Err(RestoreTargetAssetPreflightError::CorruptedData);
        }
        let missing = references
            .iter()
            .filter(|reference| {
                availability.get(reference.asset_id().as_str()) == Some(&AssetAvailability::Missing)
            })
            .cloned()
            .collect::<Vec<_>>();
        if missing.is_empty() {
            Ok(RestoreTargetAssetPreflightOutcome::Available)
        } else {
            Ok(RestoreTargetAssetPreflightOutcome::BlockedMissingAssets(
                missing,
            ))
        }
    }
}

const fn map_resolver_error(
    error: AssetAvailabilityResolveError,
) -> RestoreTargetAssetPreflightError {
    match error {
        AssetAvailabilityResolveError::StorageUnavailable => {
            RestoreTargetAssetPreflightError::StorageUnavailable
        }
        AssetAvailabilityResolveError::CorruptedData => {
            RestoreTargetAssetPreflightError::CorruptedData
        }
    }
}
