use cabinet_domain::asset::AssetId;
use cabinet_domain::document::DocumentId;
use cabinet_domain::group::GroupId;
use cabinet_domain::permission::{
    AccessResource, AccessSubject, AssetPolicy, CollectionId, CollectionPolicy, DocumentPolicy,
    Permission, PermissionDecision, PermissionDecisionReason, PermissionDecisionResult,
    PolicyOverride, PolicySource, Role, WorkspacePolicy,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;

#[test]
fn workspace_role_matrix_decides_document_permissions() {
    let workspace_policy = WorkspacePolicy::default_role_matrix();
    let document = document_resource();

    let cases = [
        (
            Role::Owner,
            &[
                Permission::Read,
                Permission::Write,
                Permission::Review,
                Permission::Publish,
                Permission::Manage,
            ][..],
            &[][..],
        ),
        (
            Role::Admin,
            &[
                Permission::Read,
                Permission::Write,
                Permission::Review,
                Permission::Publish,
                Permission::Manage,
            ][..],
            &[][..],
        ),
        (
            Role::Editor,
            &[Permission::Read, Permission::Write][..],
            &[Permission::Review, Permission::Publish, Permission::Manage][..],
        ),
        (
            Role::Reviewer,
            &[Permission::Read, Permission::Review][..],
            &[Permission::Write, Permission::Publish, Permission::Manage][..],
        ),
        (
            Role::Viewer,
            &[Permission::Read][..],
            &[
                Permission::Write,
                Permission::Review,
                Permission::Publish,
                Permission::Manage,
            ][..],
        ),
    ];

    for (role, allowed_permissions, denied_permissions) in cases {
        let subject = subject_with_role(role);

        for permission in allowed_permissions.iter().copied() {
            let decision = workspace_policy.decide(&subject, &document, permission);
            assert_eq!(
                decision,
                PermissionDecision::allowed(
                    PolicySource::Workspace,
                    PermissionDecisionReason::RoleAllowsPermission
                ),
                "role {role:?} should allow {permission:?}"
            );
        }

        for permission in denied_permissions.iter().copied() {
            let decision = workspace_policy.decide(&subject, &document, permission);
            assert_eq!(
                decision,
                PermissionDecision::denied(
                    PolicySource::Workspace,
                    PermissionDecisionReason::RoleDoesNotAllowPermission
                ),
                "role {role:?} should deny {permission:?}"
            );
        }
    }
}

#[test]
fn asset_metadata_and_asset_content_permissions_are_separate() {
    let workspace_policy = WorkspacePolicy::default_role_matrix();
    let asset = asset_resource();

    let reviewer = subject_with_role(Role::Reviewer);
    assert_eq!(
        workspace_policy.decide(&reviewer, &asset, Permission::ReadAssetMetadata),
        PermissionDecision::allowed(
            PolicySource::Workspace,
            PermissionDecisionReason::RoleAllowsPermission
        )
    );
    assert_eq!(
        workspace_policy.decide(&reviewer, &asset, Permission::ReadAssetContent),
        PermissionDecision::denied(
            PolicySource::Workspace,
            PermissionDecisionReason::RoleDoesNotAllowPermission
        )
    );

    let viewer = subject_with_role(Role::Viewer);
    assert_eq!(
        workspace_policy.decide(&viewer, &asset, Permission::ReadAssetMetadata),
        PermissionDecision::allowed(
            PolicySource::Workspace,
            PermissionDecisionReason::RoleAllowsPermission
        )
    );
    assert_eq!(
        workspace_policy.decide(&viewer, &asset, Permission::ReadAssetContent),
        PermissionDecision::denied(
            PolicySource::Workspace,
            PermissionDecisionReason::RoleDoesNotAllowPermission
        )
    );
}

#[test]
fn asset_policy_override_applies_after_document_policy_for_specific_asset() {
    let workspace_policy = WorkspacePolicy::default_role_matrix();
    let document_policy = DocumentPolicy::new(document_id())
        .with_override(PolicyOverride::allow(Permission::ReadAssetContent));
    let asset_policy = AssetPolicy::new(asset_id())
        .with_override(PolicyOverride::hide(Permission::ReadAssetContent));
    let asset = asset_resource();
    let reviewer = subject_with_role(Role::Reviewer);

    let document_decision = document_policy.decide_with_parents(
        &workspace_policy,
        None,
        &reviewer,
        &asset,
        Permission::ReadAssetContent,
    );
    assert_eq!(
        document_decision.result(),
        PermissionDecisionResult::Allowed
    );

    let asset_decision = asset_policy.decide_with_parents(
        &workspace_policy,
        None,
        Some(&document_policy),
        &reviewer,
        &asset,
        Permission::ReadAssetContent,
    );

    assert_eq!(
        asset_decision,
        PermissionDecision::not_found(
            PolicySource::Asset,
            PermissionDecisionReason::HiddenByPolicy
        )
    );
}

#[test]
fn collection_policy_override_applies_before_workspace_inheritance() {
    let workspace_policy = WorkspacePolicy::default_role_matrix();
    let collection_id = collection_id();
    let document = document_resource();
    let editor = subject_with_role(Role::Editor);
    let collection_policy = CollectionPolicy::new(collection_id.clone())
        .with_override(PolicyOverride::deny(Permission::Write));

    let inherited = workspace_policy.decide(&editor, &document, Permission::Write);
    assert_eq!(inherited.result(), PermissionDecisionResult::Allowed);

    let overridden = collection_policy.decide_with_parent(
        &workspace_policy,
        &editor,
        &document,
        Permission::Write,
    );
    assert_eq!(
        overridden,
        PermissionDecision::denied(
            PolicySource::Collection,
            PermissionDecisionReason::PolicyOverrideDenied
        )
    );
}

#[test]
fn document_policy_override_applies_before_collection_policy() {
    let workspace_policy = WorkspacePolicy::default_role_matrix();
    let collection_policy = CollectionPolicy::new(collection_id())
        .with_override(PolicyOverride::deny(Permission::Write));
    let document_policy =
        DocumentPolicy::new(document_id()).with_override(PolicyOverride::allow(Permission::Write));
    let document = document_resource();
    let editor = subject_with_role(Role::Editor);

    let decision = document_policy.decide_with_parents(
        &workspace_policy,
        Some(&collection_policy),
        &editor,
        &document,
        Permission::Write,
    );

    assert_eq!(
        decision,
        PermissionDecision::allowed(
            PolicySource::Document,
            PermissionDecisionReason::PolicyOverrideAllowed
        )
    );
}

#[test]
fn hidden_denial_returns_not_found_without_exposing_resource_existence() {
    let workspace_policy = WorkspacePolicy::default_role_matrix();
    let document_policy =
        DocumentPolicy::new(document_id()).with_override(PolicyOverride::hide(Permission::Read));
    let document = document_resource();
    let viewer = subject_with_role(Role::Viewer);

    let decision = document_policy.decide_with_parents(
        &workspace_policy,
        None,
        &viewer,
        &document,
        Permission::Read,
    );

    assert_eq!(
        decision,
        PermissionDecision::not_found(
            PolicySource::Document,
            PermissionDecisionReason::HiddenByPolicy
        )
    );
}

#[test]
fn mismatched_policy_resource_returns_indeterminate_reason() {
    let workspace_policy = WorkspacePolicy::default_role_matrix();
    let document_policy = DocumentPolicy::new(
        DocumentId::new("other-document").expect("valid mismatched document id"),
    )
    .with_override(PolicyOverride::allow(Permission::Write));
    let document = document_resource();
    let editor = subject_with_role(Role::Editor);

    let decision = document_policy.decide_with_parents(
        &workspace_policy,
        None,
        &editor,
        &document,
        Permission::Write,
    );

    assert_eq!(
        decision,
        PermissionDecision::indeterminate(
            PolicySource::Document,
            PermissionDecisionReason::PolicyResourceMismatch
        )
    );
    assert_eq!(decision.reason_code(), "POLICY_RESOURCE_MISMATCH");
}

#[test]
fn access_subject_keeps_user_group_and_role_identity_without_external_state() {
    let user_id = user_id();
    let group_id = GroupId::new("group-editors").expect("valid group id");
    let subject = AccessSubject::new(user_id.clone(), vec![Role::Editor], vec![group_id.clone()]);

    assert_eq!(subject.user_id(), &user_id);
    assert_eq!(subject.roles(), &[Role::Editor]);
    assert_eq!(subject.group_ids(), &[group_id]);
}

#[test]
fn permission_decision_is_deterministic_for_same_input() {
    let workspace_policy = WorkspacePolicy::default_role_matrix();
    let subject = subject_with_role(Role::Admin);
    let document = document_resource();

    let first = workspace_policy.decide(&subject, &document, Permission::Publish);
    let second = workspace_policy.decide(&subject, &document, Permission::Publish);

    assert_eq!(first, second);
    assert_eq!(first.result(), PermissionDecisionResult::Allowed);
    assert_eq!(first.reason_code(), "ROLE_ALLOWS_PERMISSION");
}

fn subject_with_role(role: Role) -> AccessSubject {
    AccessSubject::new(user_id(), vec![role], Vec::new())
}

fn user_id() -> UserId {
    UserId::new("user-1").expect("valid user id")
}

fn workspace_id() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("valid workspace id")
}

fn collection_id() -> CollectionId {
    CollectionId::new("collection-1").expect("valid collection id")
}

fn document_id() -> DocumentId {
    DocumentId::new("document-1").expect("valid document id")
}

fn asset_id() -> AssetId {
    AssetId::from_sha256_hex("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        .expect("valid asset id")
}

fn document_resource() -> AccessResource {
    AccessResource::document(workspace_id(), Some(collection_id()), document_id())
}

fn asset_resource() -> AccessResource {
    AccessResource::asset(
        workspace_id(),
        Some(collection_id()),
        Some(document_id()),
        asset_id(),
    )
}
