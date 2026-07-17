use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::version::AttachmentSnapshotState;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_availability::{
    AssetAvailability, AssetAvailabilityBatchResolver, AssetAvailabilityRecord,
    AssetAvailabilityResolveError,
};
use cabinet_usecases::restore_target_asset_preflight::{
    RestoreTargetAssetPreflightError, RestoreTargetAssetPreflightInput,
    RestoreTargetAssetPreflightOutcome, RestoreTargetAssetPreflightUsecase,
};

struct Resolver {
    records: Vec<AssetAvailabilityRecord>,
}

impl AssetAvailabilityBatchResolver for Resolver {
    fn resolve_batch(
        &self,
        _workspace_id: &WorkspaceId,
        _asset_ids: &[AssetId],
    ) -> Result<Vec<AssetAvailabilityRecord>, AssetAvailabilityResolveError> {
        Ok(self.records.clone())
    }
}

#[test]
fn known_target_reports_missing_references_without_losing_labels() {
    let state = AttachmentSnapshotState::known(vec![
        reference('a', "Available"),
        reference('b', "Missing"),
    ])
    .unwrap();
    let resolver = Resolver {
        records: vec![
            AssetAvailabilityRecord::new(asset('a'), AssetAvailability::Available),
            AssetAvailabilityRecord::new(asset('b'), AssetAvailability::Missing),
        ],
    };

    let outcome = RestoreTargetAssetPreflightUsecase::new()
        .execute(
            RestoreTargetAssetPreflightInput::new("workspace-1", state),
            &resolver,
        )
        .unwrap();

    let RestoreTargetAssetPreflightOutcome::BlockedMissingAssets(missing) = outcome else {
        panic!("blocked outcome");
    };
    assert_eq!(missing, vec![reference('b', "Missing")]);
}

#[test]
fn available_and_legacy_targets_have_distinct_success_outcomes() {
    let available = RestoreTargetAssetPreflightUsecase::new()
        .execute(
            RestoreTargetAssetPreflightInput::new(
                "workspace-1",
                AttachmentSnapshotState::known(vec![reference('a', "A")]).unwrap(),
            ),
            &Resolver {
                records: vec![AssetAvailabilityRecord::new(
                    asset('a'),
                    AssetAvailability::Available,
                )],
            },
        )
        .unwrap();
    let legacy = RestoreTargetAssetPreflightUsecase::new()
        .execute(
            RestoreTargetAssetPreflightInput::new(
                "workspace-1",
                AttachmentSnapshotState::legacy_unknown(),
            ),
            &Resolver {
                records: Vec::new(),
            },
        )
        .unwrap();

    assert_eq!(available, RestoreTargetAssetPreflightOutcome::Available);
    assert_eq!(legacy, RestoreTargetAssetPreflightOutcome::LegacyPreserved);
}

#[test]
fn incomplete_duplicate_or_unknown_resolver_results_are_corrupted() {
    let state = AttachmentSnapshotState::known(vec![reference('a', "A")]).unwrap();
    for records in [
        Vec::new(),
        vec![
            AssetAvailabilityRecord::new(asset('a'), AssetAvailability::Available),
            AssetAvailabilityRecord::new(asset('a'), AssetAvailability::Available),
        ],
        vec![AssetAvailabilityRecord::new(
            asset('b'),
            AssetAvailability::Available,
        )],
    ] {
        let error = RestoreTargetAssetPreflightUsecase::new()
            .execute(
                RestoreTargetAssetPreflightInput::new("workspace-1", state.clone()),
                &Resolver { records },
            )
            .expect_err("corrupted response");
        assert_eq!(error, RestoreTargetAssetPreflightError::CorruptedData);
    }
}

fn reference(value: char, label: &str) -> AssetReference {
    AssetReference::new(asset(value), label).unwrap()
}

fn asset(value: char) -> AssetId {
    AssetId::from_sha256_hex(&value.to_string().repeat(64)).unwrap()
}
