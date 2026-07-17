use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::version::{
    AttachmentSnapshot, AttachmentSnapshotError, AttachmentSnapshotState,
};

fn asset_reference(hash_character: char, label: &str) -> AssetReference {
    let hash = hash_character.to_string().repeat(64);
    AssetReference::new(
        AssetId::from_sha256_hex(&hash).expect("valid asset id"),
        label,
    )
    .expect("valid asset reference")
}

#[test]
fn known_empty_and_legacy_unknown_are_distinct_attachment_states() {
    let known = AttachmentSnapshotState::known(Vec::new()).expect("known empty snapshot");
    let legacy = AttachmentSnapshotState::legacy_unknown();

    assert_ne!(known, legacy);
    assert_eq!(known.references(), Some(&[][..]));
    assert_eq!(legacy.references(), None);
    assert!(!known.is_legacy_unknown());
    assert!(legacy.is_legacy_unknown());
}

#[test]
fn known_snapshot_preserves_labels_and_has_canonical_order() {
    let first = asset_reference('a', "Architecture");
    let second = asset_reference('b', "Requirements");

    let forward =
        AttachmentSnapshot::new(vec![first.clone(), second.clone()]).expect("forward snapshot");
    let reverse = AttachmentSnapshot::new(vec![second, first]).expect("reverse snapshot");

    assert_eq!(forward, reverse);
    assert_eq!(forward.references()[0].label(), "Architecture");
    assert_eq!(forward.references()[1].label(), "Requirements");
    assert_eq!(forward.references()[0].asset_id().as_str(), "a".repeat(64));
    assert_eq!(forward.references()[1].asset_id().as_str(), "b".repeat(64));
}

#[test]
fn snapshot_rejects_duplicate_asset_ids_instead_of_deduplicating_them() {
    let duplicate_with_same_label = AttachmentSnapshot::new(vec![
        asset_reference('c', "Diagram"),
        asset_reference('c', "Diagram"),
    ])
    .expect_err("same reference must be rejected");
    let duplicate_with_different_label = AttachmentSnapshot::new(vec![
        asset_reference('d', "Original"),
        asset_reference('d', "Renamed"),
    ])
    .expect_err("same asset id with another label must be rejected");

    assert_eq!(
        duplicate_with_same_label,
        AttachmentSnapshotError::DuplicateAssetReference
    );
    assert_eq!(duplicate_with_same_label, duplicate_with_different_label);
    assert_eq!(
        duplicate_with_different_label.code(),
        "version.duplicate_attachment_reference"
    );
}
