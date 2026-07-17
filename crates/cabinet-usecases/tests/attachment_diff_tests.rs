use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::version::AttachmentSnapshotState;
use cabinet_usecases::attachment_diff::{AttachmentDiff, compare_attachment_snapshots};

#[test]
fn known_snapshots_report_added_removed_unchanged_in_asset_order() {
    let left = known(vec![reference('b', "B"), reference('a', "A")]);
    let right = known(vec![
        reference('d', "D"),
        reference('b', "B"),
        reference('c', "C"),
    ]);

    let AttachmentDiff::Known(diff) = compare_attachment_snapshots(&left, &right) else {
        panic!("known diff");
    };

    assert_eq!(ids(diff.added()), vec!["c", "d"]);
    assert_eq!(ids(diff.removed()), vec!["a"]);
    assert_eq!(diff.unchanged_count(), 1);
    assert!(diff.relabeled().is_empty());
}

#[test]
fn label_change_is_explicit_and_not_reported_as_remove_add() {
    let left = known(vec![reference('a', "초안")]);
    let right = known(vec![reference('a', "최종")]);

    let AttachmentDiff::Known(diff) = compare_attachment_snapshots(&left, &right) else {
        panic!("known diff");
    };

    assert!(diff.added().is_empty());
    assert!(diff.removed().is_empty());
    assert_eq!(diff.unchanged_count(), 0);
    assert_eq!(diff.relabeled().len(), 1);
    assert_eq!(diff.relabeled()[0].before_label(), "초안");
    assert_eq!(diff.relabeled()[0].after_label(), "최종");
}

#[test]
fn empty_known_is_distinct_from_legacy_unknown() {
    let empty = known(Vec::new());
    let legacy = AttachmentSnapshotState::legacy_unknown();

    assert!(matches!(
        compare_attachment_snapshots(&empty, &empty),
        AttachmentDiff::Known(_)
    ));
    assert_eq!(
        compare_attachment_snapshots(&legacy, &empty),
        AttachmentDiff::LegacyUnknown
    );
    assert_eq!(
        compare_attachment_snapshots(&empty, &legacy),
        AttachmentDiff::LegacyUnknown
    );
}

fn known(references: Vec<AssetReference>) -> AttachmentSnapshotState {
    AttachmentSnapshotState::known(references).unwrap()
}

fn reference(seed: char, label: &str) -> AssetReference {
    AssetReference::new(
        AssetId::from_sha256_hex(&seed.to_string().repeat(64)).unwrap(),
        label,
    )
    .unwrap()
}

fn ids(references: &[AssetReference]) -> Vec<&str> {
    references
        .iter()
        .map(|reference| &reference.asset_id().as_str()[..1])
        .collect()
}
