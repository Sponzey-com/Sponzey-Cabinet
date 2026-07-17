use std::cell::{Cell, RefCell};

use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::version::AttachmentSnapshotState;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_availability::{
    AssetAvailability, AssetAvailabilityBatchResolver, AssetAvailabilityRecord,
    AssetAvailabilityResolveError,
};
use cabinet_usecases::attachment_diff::{AttachmentDiff, compare_attachment_snapshots};
use cabinet_usecases::resolve_attachment_diff_availability::{
    ResolveAttachmentDiffAvailabilityError, ResolveAttachmentDiffAvailabilityInput,
    ResolveAttachmentDiffAvailabilityUsecase, ResolvedAttachmentDiff,
};

struct FakeResolver {
    calls: Cell<usize>,
    requested: RefCell<Vec<String>>,
    records: Vec<AssetAvailabilityRecord>,
    error: Option<AssetAvailabilityResolveError>,
}

impl AssetAvailabilityBatchResolver for FakeResolver {
    fn resolve_batch(
        &self,
        _workspace_id: &WorkspaceId,
        asset_ids: &[AssetId],
    ) -> Result<Vec<AssetAvailabilityRecord>, AssetAvailabilityResolveError> {
        self.calls.set(self.calls.get() + 1);
        self.requested.replace(
            asset_ids
                .iter()
                .map(|asset_id| asset_id.as_str().to_string())
                .collect(),
        );
        self.error.map_or_else(|| Ok(self.records.clone()), Err)
    }
}

#[test]
fn resolves_all_changed_assets_once_and_preserves_delta_order_and_labels() {
    let diff = compare_attachment_snapshots(
        &known(vec![
            reference('a', "초안"),
            reference('b', "제거"),
            reference('c', "유지"),
        ]),
        &known(vec![
            reference('a', "최종"),
            reference('c', "유지"),
            reference('d', "추가"),
        ]),
    );
    let resolver = FakeResolver {
        calls: Cell::new(0),
        requested: RefCell::new(Vec::new()),
        records: vec![
            availability('d', AssetAvailability::Available),
            availability('b', AssetAvailability::Missing),
            availability('a', AssetAvailability::Available),
        ],
        error: None,
    };

    let output = ResolveAttachmentDiffAvailabilityUsecase::new()
        .execute(
            ResolveAttachmentDiffAvailabilityInput::new("workspace-1", diff),
            &resolver,
        )
        .unwrap();

    let ResolvedAttachmentDiff::Known(known) = output else {
        panic!("known output");
    };
    assert_eq!(resolver.calls.get(), 1);
    assert_eq!(
        resolver.requested.borrow().as_slice(),
        [
            asset_id('a').as_str(),
            asset_id('b').as_str(),
            asset_id('d').as_str()
        ]
    );
    assert_eq!(known.added()[0].reference().label(), "추가");
    assert_eq!(
        known.added()[0].availability(),
        AssetAvailability::Available
    );
    assert_eq!(known.removed()[0].reference().label(), "제거");
    assert_eq!(
        known.removed()[0].availability(),
        AssetAvailability::Missing
    );
    assert_eq!(known.relabeled()[0].before_label(), "초안");
    assert_eq!(known.relabeled()[0].after_label(), "최종");
    assert_eq!(
        known.relabeled()[0].availability(),
        AssetAvailability::Available
    );
    assert_eq!(known.unchanged_count(), 1);
}

#[test]
fn legacy_unknown_and_known_empty_do_not_call_the_resolver() {
    let resolver = empty_resolver();
    let legacy = ResolveAttachmentDiffAvailabilityUsecase::new()
        .execute(
            ResolveAttachmentDiffAvailabilityInput::new(
                "workspace-1",
                AttachmentDiff::LegacyUnknown,
            ),
            &resolver,
        )
        .unwrap();
    assert_eq!(legacy, ResolvedAttachmentDiff::LegacyUnknown);

    let empty = compare_attachment_snapshots(&known(Vec::new()), &known(Vec::new()));
    let output = ResolveAttachmentDiffAvailabilityUsecase::new()
        .execute(
            ResolveAttachmentDiffAvailabilityInput::new("workspace-1", empty),
            &resolver,
        )
        .unwrap();
    let ResolvedAttachmentDiff::Known(known) = output else {
        panic!("known empty");
    };
    assert!(known.added().is_empty());
    assert_eq!(resolver.calls.get(), 0);
}

#[test]
fn missing_duplicate_and_unrequested_results_are_corrupted_data() {
    let diff =
        compare_attachment_snapshots(&known(Vec::new()), &known(vec![reference('a', "추가")]));
    for records in [
        Vec::new(),
        vec![
            availability('a', AssetAvailability::Available),
            availability('a', AssetAvailability::Missing),
        ],
        vec![
            availability('a', AssetAvailability::Available),
            availability('b', AssetAvailability::Available),
        ],
    ] {
        let resolver = FakeResolver {
            calls: Cell::new(0),
            requested: RefCell::new(Vec::new()),
            records,
            error: None,
        };
        assert_eq!(
            ResolveAttachmentDiffAvailabilityUsecase::new()
                .execute(
                    ResolveAttachmentDiffAvailabilityInput::new("workspace-1", diff.clone()),
                    &resolver,
                )
                .unwrap_err(),
            ResolveAttachmentDiffAvailabilityError::CorruptedData
        );
    }
}

#[test]
fn invalid_workspace_and_storage_failure_are_typed() {
    let diff =
        compare_attachment_snapshots(&known(Vec::new()), &known(vec![reference('a', "추가")]));
    let resolver = FakeResolver {
        calls: Cell::new(0),
        requested: RefCell::new(Vec::new()),
        records: Vec::new(),
        error: Some(AssetAvailabilityResolveError::StorageUnavailable),
    };
    assert_eq!(
        ResolveAttachmentDiffAvailabilityUsecase::new()
            .execute(
                ResolveAttachmentDiffAvailabilityInput::new("", diff.clone()),
                &resolver,
            )
            .unwrap_err(),
        ResolveAttachmentDiffAvailabilityError::InvalidInput
    );
    assert_eq!(resolver.calls.get(), 0);
    assert_eq!(
        ResolveAttachmentDiffAvailabilityUsecase::new()
            .execute(
                ResolveAttachmentDiffAvailabilityInput::new("workspace-1", diff),
                &resolver,
            )
            .unwrap_err(),
        ResolveAttachmentDiffAvailabilityError::StorageUnavailable
    );
}

fn empty_resolver() -> FakeResolver {
    FakeResolver {
        calls: Cell::new(0),
        requested: RefCell::new(Vec::new()),
        records: Vec::new(),
        error: None,
    }
}

fn known(references: Vec<AssetReference>) -> AttachmentSnapshotState {
    AttachmentSnapshotState::known(references).unwrap()
}

fn reference(seed: char, label: &str) -> AssetReference {
    AssetReference::new(asset_id(seed), label).unwrap()
}

fn availability(seed: char, value: AssetAvailability) -> AssetAvailabilityRecord {
    AssetAvailabilityRecord::new(asset_id(seed), value)
}

fn asset_id(seed: char) -> AssetId {
    AssetId::from_sha256_hex(&seed.to_string().repeat(64)).unwrap()
}
