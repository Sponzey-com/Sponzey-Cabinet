use std::fs;
use std::path::PathBuf;

use cabinet_adapters::local_permission_policy_repository::LocalPermissionPolicyRepository;
use cabinet_domain::document::DocumentId;
use cabinet_domain::group::GroupId;
use cabinet_domain::permission::{
    CollectionId, CollectionPolicy, DocumentPolicy, Permission, PolicyOverride, Role,
    RoleAssignment, RoleAssignmentId, RoleAssignmentSubject,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::permission_policy_repository::{
    PermissionPolicyRepository, PermissionRepositoryError, RoleAssignmentMutationResult,
    RoleAssignmentRemovalResult,
};

#[test]
fn local_permission_policy_repository_persists_role_assignments_and_subject_indexes() {
    let root = unique_temp_dir("local-permission-repository-assignments");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let user_id = UserId::new("user-1").expect("user id");
    let group_id = GroupId::new("group-1").expect("group id");
    let user_assignment =
        role_assignment_for_user("role-assignment-1", &workspace_id, &user_id, Role::Editor);
    let group_assignment = role_assignment_for_group(
        "role-assignment-2",
        &workspace_id,
        &group_id,
        Role::Reviewer,
    );
    let mut repository = LocalPermissionPolicyRepository::new(root.clone());

    assert_eq!(
        repository
            .save_role_assignment(&workspace_id, user_assignment.clone())
            .expect("save user assignment"),
        RoleAssignmentMutationResult::Changed
    );
    assert_eq!(
        repository
            .save_role_assignment(&workspace_id, user_assignment.clone())
            .expect("save user assignment again"),
        RoleAssignmentMutationResult::AlreadyApplied
    );
    repository
        .save_role_assignment(&workspace_id, group_assignment.clone())
        .expect("save group assignment");

    let mut restarted = LocalPermissionPolicyRepository::new(root.clone());
    assert_eq!(
        restarted
            .get_role_assignment(&workspace_id, user_assignment.id())
            .expect("get assignment"),
        Some(user_assignment.clone())
    );
    assert_eq!(
        restarted
            .list_user_role_assignments(&workspace_id, &user_id)
            .expect("list user assignments"),
        vec![user_assignment.clone()]
    );
    assert_eq!(
        restarted
            .list_group_role_assignments(&workspace_id, &[group_id.clone()])
            .expect("list group assignments"),
        vec![group_assignment.clone()]
    );
    assert_eq!(
        restarted
            .list_workspace_role_assignments(&workspace_id)
            .expect("list workspace assignments"),
        vec![user_assignment.clone(), group_assignment.clone()]
    );
    assert_eq!(
        restarted
            .remove_role_assignment(&workspace_id, user_assignment.id())
            .expect("remove assignment"),
        RoleAssignmentRemovalResult::Removed
    );
    assert_eq!(
        restarted
            .remove_role_assignment(&workspace_id, user_assignment.id())
            .expect("remove missing assignment"),
        RoleAssignmentRemovalResult::Missing
    );
    assert_eq!(
        LocalPermissionPolicyRepository::new(root.clone())
            .get_role_assignment(&workspace_id, user_assignment.id())
            .expect("get removed assignment"),
        None
    );
    cleanup_temp_dir(root);
}

#[test]
fn local_permission_policy_repository_round_trips_collection_and_document_policies() {
    let root = unique_temp_dir("local-permission-repository-policies");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let collection_policy =
        CollectionPolicy::new(CollectionId::new("collection-1").expect("collection id"))
            .with_override(PolicyOverride::deny(Permission::Write))
            .with_override(PolicyOverride::hide(Permission::Read));
    let document_policy = DocumentPolicy::new(DocumentId::new("document-1").expect("document id"))
        .with_override(PolicyOverride::allow(Permission::Review))
        .with_override(PolicyOverride::deny(Permission::Write));
    let mut repository = LocalPermissionPolicyRepository::new(root.clone());

    repository
        .save_collection_policy(&workspace_id, collection_policy.clone())
        .expect("save collection policy");
    repository
        .save_document_policy(&workspace_id, document_policy.clone())
        .expect("save document policy");

    let restarted = LocalPermissionPolicyRepository::new(root.clone());
    assert_eq!(
        restarted
            .get_collection_policy(&workspace_id, collection_policy.collection_id())
            .expect("get collection policy"),
        Some(collection_policy)
    );
    assert_eq!(
        restarted
            .get_document_policy(&workspace_id, document_policy.document_id())
            .expect("get document policy"),
        Some(document_policy)
    );
    assert!(!format!("{restarted:?}").contains("document-1"));
    cleanup_temp_dir(root);
}

#[test]
fn local_permission_policy_repository_reports_corrupted_assignment_and_policy_files() {
    let root = unique_temp_dir("local-permission-repository-corrupt");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let user_id = UserId::new("user-1").expect("user id");
    let assignment =
        role_assignment_for_user("role-assignment-1", &workspace_id, &user_id, Role::Viewer);
    let collection_policy =
        CollectionPolicy::new(CollectionId::new("collection-1").expect("collection id"))
            .with_override(PolicyOverride::allow(Permission::Read));
    let mut repository = LocalPermissionPolicyRepository::new(root.clone());

    repository
        .save_role_assignment(&workspace_id, assignment)
        .expect("save assignment");
    fs::write(
        first_file_under(&root.join("permissions"), "assignment"),
        "not-an-assignment",
    )
    .expect("corrupt assignment");
    let assignment_error = repository
        .list_workspace_role_assignments(&workspace_id)
        .expect_err("corrupted assignment must fail");

    repository
        .save_collection_policy(&workspace_id, collection_policy.clone())
        .expect("save collection policy");
    fs::write(
        first_file_under(&root.join("permissions"), "policy"),
        "not-a-policy",
    )
    .expect("corrupt policy");
    let policy_error = repository
        .get_collection_policy(&workspace_id, collection_policy.collection_id())
        .expect_err("corrupted policy must fail");

    assert_eq!(
        assignment_error,
        PermissionRepositoryError::StorageUnavailable
    );
    assert_eq!(policy_error, PermissionRepositoryError::StorageUnavailable);
    cleanup_temp_dir(root);
}

fn role_assignment_for_user(
    assignment_id: &str,
    workspace_id: &WorkspaceId,
    user_id: &UserId,
    role: Role,
) -> RoleAssignment {
    RoleAssignment::new(
        RoleAssignmentId::new(assignment_id).expect("assignment id"),
        workspace_id.clone(),
        RoleAssignmentSubject::User(user_id.clone()),
        role,
    )
}

fn role_assignment_for_group(
    assignment_id: &str,
    workspace_id: &WorkspaceId,
    group_id: &GroupId,
    role: Role,
) -> RoleAssignment {
    RoleAssignment::new(
        RoleAssignmentId::new(assignment_id).expect("assignment id"),
        workspace_id.clone(),
        RoleAssignmentSubject::Group(group_id.clone()),
        role,
    )
}

fn first_file_under(root: &PathBuf, extension: &str) -> PathBuf {
    let mut stack = vec![root.clone()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(path).expect("read dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
                return path;
            }
        }
    }
    panic!("file with extension {extension} not found");
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sponzey-cabinet-{name}-{}", std::process::id()));
    cleanup_temp_dir(dir.clone());
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn cleanup_temp_dir(dir: PathBuf) {
    if dir.exists() {
        fs::remove_dir_all(dir).expect("remove temp dir");
    }
}
