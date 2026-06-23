use cabinet_domain::asset::{
    AssetError, AssetLifecycleEvent, AssetLifecycleState, transition_asset_lifecycle,
};

#[test]
fn asset_lifecycle_allows_expected_link_archive_missing_and_restore_flow() {
    let linked =
        transition_asset_lifecycle(AssetLifecycleState::Registered, AssetLifecycleEvent::Link)
            .expect("link should transition");
    assert_eq!(linked.next_state, AssetLifecycleState::Linked);

    let unlinked = transition_asset_lifecycle(linked.next_state, AssetLifecycleEvent::Unlink)
        .expect("unlink should transition");
    assert_eq!(unlinked.next_state, AssetLifecycleState::Unlinked);

    let archived = transition_asset_lifecycle(unlinked.next_state, AssetLifecycleEvent::Archive)
        .expect("archive should transition");
    assert_eq!(archived.next_state, AssetLifecycleState::Archived);

    let restored = transition_asset_lifecycle(archived.next_state, AssetLifecycleEvent::Restore)
        .expect("restore should transition");
    assert_eq!(restored.next_state, AssetLifecycleState::Restored);

    let missing = transition_asset_lifecycle(restored.next_state, AssetLifecycleEvent::MarkMissing)
        .expect("mark missing should transition");
    assert_eq!(missing.next_state, AssetLifecycleState::Missing);
}

#[test]
fn asset_lifecycle_rejects_invalid_transition() {
    let error = transition_asset_lifecycle(
        AssetLifecycleState::Registered,
        AssetLifecycleEvent::Restore,
    )
    .expect_err("restore from registered must fail");

    assert_eq!(
        error,
        AssetError::InvalidLifecycleTransition {
            state: AssetLifecycleState::Registered,
            event: AssetLifecycleEvent::Restore,
        }
    );
}
