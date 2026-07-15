use cabinet_platform::local_desktop_runtime::{
    NativeBootstrapErrorCode, NativeBootstrapEvent, NativeBootstrapState,
    transition_native_bootstrap,
};

#[test]
fn native_bootstrap_state_machine_reaches_ready_in_order() {
    let reading_config =
        transition_native_bootstrap(NativeBootstrapState::Pending, NativeBootstrapEvent::Start);
    let resolving_app_data =
        transition_native_bootstrap(reading_config.next_state, NativeBootstrapEvent::ConfigRead);
    let initializing_stores = transition_native_bootstrap(
        resolving_app_data.next_state,
        NativeBootstrapEvent::AppDataResolved,
    );
    let opening_default_workspace = transition_native_bootstrap(
        initializing_stores.next_state,
        NativeBootstrapEvent::StoresInitialized,
    );
    let ready = transition_native_bootstrap(
        opening_default_workspace.next_state,
        NativeBootstrapEvent::DefaultWorkspaceOpened,
    );

    assert_eq!(
        reading_config.next_state,
        NativeBootstrapState::ReadingConfig
    );
    assert_eq!(
        resolving_app_data.next_state,
        NativeBootstrapState::ResolvingAppData
    );
    assert_eq!(
        initializing_stores.next_state,
        NativeBootstrapState::InitializingStores
    );
    assert_eq!(
        opening_default_workspace.next_state,
        NativeBootstrapState::OpeningDefaultWorkspace
    );
    assert_eq!(ready.next_state, NativeBootstrapState::Ready);
    assert_eq!(ready.error_code, None);
}

#[test]
fn native_bootstrap_fail_event_returns_stable_error_state() {
    let failed = transition_native_bootstrap(
        NativeBootstrapState::ReadingConfig,
        NativeBootstrapEvent::Fail(NativeBootstrapErrorCode::ConfigInvalid),
    );

    assert_eq!(
        failed.next_state,
        NativeBootstrapState::Failed {
            error_code: NativeBootstrapErrorCode::ConfigInvalid,
            retryable: true,
        }
    );
    assert_eq!(
        failed.error_code,
        Some(NativeBootstrapErrorCode::ConfigInvalid)
    );
}

#[test]
fn native_bootstrap_invalid_transition_returns_non_retryable_failure() {
    let invalid = transition_native_bootstrap(
        NativeBootstrapState::Pending,
        NativeBootstrapEvent::StoresInitialized,
    );

    assert_eq!(
        invalid.next_state,
        NativeBootstrapState::Failed {
            error_code: NativeBootstrapErrorCode::InvalidTransition,
            retryable: false,
        }
    );
    assert_eq!(
        invalid.error_code,
        Some(NativeBootstrapErrorCode::InvalidTransition)
    );
}
