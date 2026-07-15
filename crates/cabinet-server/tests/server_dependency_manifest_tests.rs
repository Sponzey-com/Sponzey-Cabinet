use cabinet_core::server_config::ServerConfigInput;
use cabinet_server::composition::build_server_composition;
use cabinet_server::runtime::RuntimeDependencyDurability;

#[test]
fn phase003_self_host_manifest_names_durable_local_persistence_adapters() {
    let composition = build_server_composition(default_config());
    let dependencies = composition.runtime_dependencies();

    for (dependency, implementation) in phase003_durable_dependencies() {
        let entry = dependencies
            .dependency(dependency)
            .unwrap_or_else(|| panic!("missing dependency manifest entry: {dependency}"));
        assert_eq!(entry.implementation(), *implementation);
        assert_eq!(
            entry.durability(),
            RuntimeDependencyDurability::DurableLocal
        );
    }
}

#[test]
fn phase003_self_host_manifest_reports_missing_durable_dependencies_without_logging() {
    let composition = build_server_composition(default_config());
    let dependencies = composition.runtime_dependencies();
    let required = phase003_durable_dependencies()
        .iter()
        .map(|(dependency, _)| *dependency)
        .collect::<Vec<_>>();

    assert!(
        dependencies
            .missing_durable_local_dependencies(&required)
            .is_empty()
    );
    assert_eq!(
        dependencies
            .missing_durable_local_dependencies(&["document_repository", "not_wired_store"]),
        vec!["not_wired_store"]
    );
}

#[test]
fn phase003_self_host_manifest_keeps_policy_and_runtime_utilities_out_of_durable_store_set() {
    let composition = build_server_composition(default_config());
    let dependencies = composition.runtime_dependencies();

    assert_eq!(
        dependencies
            .dependency("auth_policy")
            .expect("auth policy dependency")
            .durability(),
        RuntimeDependencyDurability::Policy
    );
    assert_eq!(
        dependencies
            .dependency("clock")
            .expect("clock dependency")
            .durability(),
        RuntimeDependencyDurability::RuntimeUtility
    );
}

#[test]
fn phase004_self_host_manifest_wires_local_realtime_dependencies_without_external_queue() {
    let composition = build_server_composition(default_config());
    let dependencies = composition.runtime_dependencies();

    let owner_policy = dependencies
        .dependency("realtime_room_owner_policy")
        .expect("realtime owner policy dependency");
    let transport = dependencies
        .dependency("realtime_transport")
        .expect("realtime transport dependency");

    assert_eq!(
        owner_policy.implementation(),
        "LocalDocumentRoomOwnerPolicy"
    );
    assert_eq!(
        owner_policy.durability(),
        RuntimeDependencyDurability::Policy
    );
    assert_eq!(transport.implementation(), "LocalRealtimeTransport");
    assert_eq!(
        transport.durability(),
        RuntimeDependencyDurability::VolatileLocal
    );
    assert!(dependencies.dependency("external_pubsub").is_none());
    assert!(dependencies.dependency("external_queue").is_none());
}

fn phase003_durable_dependencies() -> &'static [(&'static str, &'static str)] {
    &[
        ("document_repository", "LocalDocumentRepository"),
        ("version_store", "LocalVersionStore"),
        (
            "document_asset_metadata_store",
            "LocalDocumentAssetRepository",
        ),
        ("object_storage", "LocalObjectStorage"),
        ("search_index", "LocalSearchIndex"),
        ("link_index", "LocalLinkIndex"),
        ("session_store", "LocalSessionStore"),
        ("user_repository", "LocalUserRepository"),
        ("group_repository", "LocalGroupRepository"),
        (
            "permission_policy_repository",
            "LocalPermissionPolicyRepository",
        ),
        ("comment_repository", "LocalCommentRepository"),
        (
            "review_workflow_repository",
            "LocalReviewWorkflowRepository",
        ),
        ("document_lock_repository", "LocalDocumentLockRepository"),
        ("audit_store", "LocalAuditLogStore"),
        ("backup_store", "LocalBackupStore"),
    ]
}

fn default_config() -> cabinet_core::server_config::ServerConfig {
    ServerConfigInput::local_dev_defaults()
        .validate()
        .expect("valid server config")
}
