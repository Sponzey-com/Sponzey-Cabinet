use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::attachment_snapshot_mutation::{
    AttachmentSnapshotDelta, AttachmentSnapshotMutation, AttachmentSnapshotMutationError,
    transition_attachment_snapshot,
};
use cabinet_domain::version::AttachmentSnapshotState;

#[test]
fn link_adds_relabels_and_detects_identical_no_op() {
    let empty = known(Vec::new());
    let linked = transition_attachment_snapshot(
        &empty,
        AttachmentSnapshotMutation::Link(reference('b', "보고서")),
        None,
    )
    .expect("link");
    assert_eq!(linked.delta(), AttachmentSnapshotDelta::Linked);
    assert!(linked.changed());
    assert_eq!(labels(linked.state()), vec!["보고서"]);

    let unchanged = transition_attachment_snapshot(
        linked.state(),
        AttachmentSnapshotMutation::Link(reference('b', "보고서")),
        None,
    )
    .expect("same link");
    assert_eq!(unchanged.delta(), AttachmentSnapshotDelta::Unchanged);
    assert!(!unchanged.changed());

    let relabeled = transition_attachment_snapshot(
        linked.state(),
        AttachmentSnapshotMutation::Link(reference('b', "최종 보고서")),
        None,
    )
    .expect("relabel");
    assert_eq!(relabeled.delta(), AttachmentSnapshotDelta::Relabeled);
    assert_eq!(labels(relabeled.state()), vec!["최종 보고서"]);
    assert_eq!(references(relabeled.state()).len(), 1);
}

#[test]
fn unlink_preserves_other_references_and_known_empty_semantics() {
    let state = known(vec![reference('b', "B"), reference('a', "A")]);
    let unlinked = transition_attachment_snapshot(
        &state,
        AttachmentSnapshotMutation::Unlink(asset_id('a')),
        None,
    )
    .expect("unlink");
    assert_eq!(unlinked.delta(), AttachmentSnapshotDelta::Unlinked);
    assert_eq!(labels(unlinked.state()), vec!["B"]);

    let missing = transition_attachment_snapshot(
        unlinked.state(),
        AttachmentSnapshotMutation::Unlink(asset_id('a')),
        None,
    )
    .expect("missing unlink");
    assert_eq!(missing.delta(), AttachmentSnapshotDelta::Unchanged);
    assert!(!missing.changed());

    let empty = transition_attachment_snapshot(
        missing.state(),
        AttachmentSnapshotMutation::Unlink(asset_id('b')),
        None,
    )
    .expect("last unlink");
    assert_eq!(references(empty.state()), &[]);
    assert!(!empty.state().is_legacy_unknown());
}

#[test]
fn legacy_unknown_requires_explicit_baseline_and_preserves_it() {
    let legacy = AttachmentSnapshotState::legacy_unknown();
    let error = transition_attachment_snapshot(
        &legacy,
        AttachmentSnapshotMutation::Link(reference('c', "C")),
        None,
    )
    .unwrap_err();
    assert_eq!(
        error,
        AttachmentSnapshotMutationError::LegacyBaselineRequired
    );
    assert_eq!(
        error.code(),
        "attachment_snapshot_mutation.legacy_baseline_required"
    );

    let resolved = transition_attachment_snapshot(
        &legacy,
        AttachmentSnapshotMutation::Link(reference('c', "C")),
        Some(vec![reference('b', "B"), reference('a', "A")]),
    )
    .expect("legacy baseline");
    assert_eq!(resolved.delta(), AttachmentSnapshotDelta::Linked);
    assert_eq!(labels(resolved.state()), vec!["A", "B", "C"]);
    assert!(!resolved.state().is_legacy_unknown());
}

#[test]
fn duplicate_legacy_baseline_is_rejected_and_known_results_are_sorted() {
    let legacy = AttachmentSnapshotState::legacy_unknown();
    let duplicate = transition_attachment_snapshot(
        &legacy,
        AttachmentSnapshotMutation::Unlink(asset_id('c')),
        Some(vec![reference('a', "A"), reference('a', "Duplicate")]),
    )
    .unwrap_err();
    assert_eq!(duplicate, AttachmentSnapshotMutationError::InvalidBaseline);

    let sorted = transition_attachment_snapshot(
        &known(vec![reference('c', "C"), reference('a', "A")]),
        AttachmentSnapshotMutation::Link(reference('b', "B")),
        None,
    )
    .expect("sorted result");
    let ids = references(sorted.state())
        .iter()
        .map(|reference| reference.asset_id().as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![hex('a'), hex('b'), hex('c')]);
}

fn known(references: Vec<AssetReference>) -> AttachmentSnapshotState {
    AttachmentSnapshotState::known(references).unwrap()
}

fn references(state: &AttachmentSnapshotState) -> &[AssetReference] {
    state.references().expect("known state")
}

fn labels(state: &AttachmentSnapshotState) -> Vec<&str> {
    references(state)
        .iter()
        .map(AssetReference::label)
        .collect()
}

fn reference(character: char, label: &str) -> AssetReference {
    AssetReference::new(asset_id(character), label).unwrap()
}

fn asset_id(character: char) -> AssetId {
    AssetId::from_sha256_hex(&hex(character)).unwrap()
}

fn hex(character: char) -> String {
    std::iter::repeat_n(character, 64).collect()
}
