use cabinet_domain::asset::AssetId;
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetAvailability {
    Available,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetAvailabilityRecord {
    asset_id: AssetId,
    availability: AssetAvailability,
}

impl AssetAvailabilityRecord {
    pub const fn new(asset_id: AssetId, availability: AssetAvailability) -> Self {
        Self {
            asset_id,
            availability,
        }
    }

    pub const fn asset_id(&self) -> &AssetId {
        &self.asset_id
    }

    pub const fn availability(&self) -> AssetAvailability {
        self.availability
    }
}

pub trait AssetAvailabilityBatchResolver {
    fn resolve_batch(
        &self,
        workspace_id: &WorkspaceId,
        asset_ids: &[AssetId],
    ) -> Result<Vec<AssetAvailabilityRecord>, AssetAvailabilityResolveError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetAvailabilityResolveError {
    StorageUnavailable,
    CorruptedData,
}

impl AssetAvailabilityResolveError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "asset_availability.storage_unavailable",
            Self::CorruptedData => "asset_availability.corrupted_data",
        }
    }
}
