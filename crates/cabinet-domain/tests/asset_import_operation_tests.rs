use cabinet_domain::asset_import_operation::{
    AssetImportEvent as E, AssetImportOperationError, AssetImportSideEffect as Fx,
    AssetImportState as S, transition_asset_import,
};

#[test]
fn asset_import_happy_path_is_explicit_and_deterministic() {
    let steps = [
        (S::Selected, E::Begin, S::Validating, Fx::Validate),
        (S::Validating, E::ValidationSucceeded, S::Staging, Fx::Stage),
        (S::Staging, E::StagingSucceeded, S::Hashing, Fx::Hash),
        (
            S::Hashing,
            E::HashingSucceeded,
            S::PublishingObject,
            Fx::PublishObject,
        ),
        (
            S::PublishingObject,
            E::ObjectPublished,
            S::PersistingMetadata,
            Fx::PersistMetadata,
        ),
        (
            S::PersistingMetadata,
            E::MetadataPersisted,
            S::Linking,
            Fx::Link,
        ),
        (S::Linking, E::LinkSucceeded, S::Completed, Fx::None),
    ];
    for (state, event, next, effect) in steps {
        let result = transition_asset_import(state, event).expect("transition");
        assert_eq!(result.next_state, next);
        assert_eq!(result.side_effect, effect);
    }
    assert_eq!(
        transition_asset_import(S::Linking, E::LinkSucceeded)
            .expect("complete")
            .product_log_event,
        Some("asset.import.completed")
    );
}

#[test]
fn asset_import_maps_stage_failures_to_stable_terminal_states() {
    let cases = [
        (S::Validating, E::ValidationFailed, S::ValidationFailed),
        (S::Staging, E::StagingFailed, S::StagingFailed),
        (S::Hashing, E::HashingFailed, S::StagingFailed),
        (
            S::PublishingObject,
            E::ObjectPublishFailed,
            S::ObjectPublishFailed,
        ),
        (
            S::PersistingMetadata,
            E::MetadataPersistFailed,
            S::MetadataPersistFailed,
        ),
        (S::Linking, E::LinkFailed, S::LinkFailed),
    ];
    for (state, event, next) in cases {
        let result = transition_asset_import(state, event).expect("failure transition");
        assert_eq!(result.next_state, next);
        assert_eq!(result.product_log_event, Some("asset.import.failed"));
        assert!(result.error_code.is_some());
    }
}

#[test]
fn cancel_requests_cleanup_and_invalid_transitions_are_rejected() {
    let cancelling = transition_asset_import(S::Staging, E::CancelRequested).expect("cancel");
    assert_eq!(cancelling.next_state, S::Cancelling);
    assert_eq!(cancelling.side_effect, Fx::CleanupStaging);
    assert_eq!(
        transition_asset_import(S::Cancelling, E::CleanupSucceeded)
            .expect("clean")
            .next_state,
        S::Cancelled
    );
    assert_eq!(
        transition_asset_import(S::Cancelling, E::CleanupFailed)
            .expect("failed cleanup")
            .next_state,
        S::CleanupRequired
    );
    assert_eq!(
        transition_asset_import(S::PublishingObject, E::CancelRequested)
            .expect("uncertain publish")
            .next_state,
        S::CleanupRequired
    );
    assert_eq!(
        transition_asset_import(S::Completed, E::Begin).expect_err("terminal"),
        AssetImportOperationError::InvalidTransition {
            state: S::Completed,
            event: E::Begin
        }
    );
}
