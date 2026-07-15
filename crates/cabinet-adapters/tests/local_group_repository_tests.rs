use std::fs;
use std::path::PathBuf;

use cabinet_adapters::local_group_repository::LocalGroupRepository;
use cabinet_domain::group::{Group, GroupId, GroupMembership, GroupName};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::group_repository::{
    GroupRepository, GroupRepositoryError, MembershipMutationResult,
};

#[test]
fn local_group_repository_persists_group_and_name_index_across_instances() {
    let root = unique_temp_dir("local-group-repository-persist");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let group = group("group-2", &workspace_id, "Editors");

    {
        let mut repository = LocalGroupRepository::new(root.clone());
        repository
            .save_group(&workspace_id, group.clone())
            .expect("save group");
    }

    let repository = LocalGroupRepository::new(root.clone());
    let loaded = repository
        .get_group(&workspace_id, group.id())
        .expect("get group")
        .expect("stored group");
    let by_name = repository
        .find_group_by_name(
            &workspace_id,
            &GroupName::new("editors").expect("group name"),
        )
        .expect("find by name")
        .expect("name group");
    let listed = repository
        .list_workspace_groups(&workspace_id)
        .expect("list groups");

    assert_eq!(loaded.id(), group.id());
    assert_eq!(by_name.id(), group.id());
    assert_eq!(
        listed
            .iter()
            .map(|group| group.id().as_str())
            .collect::<Vec<_>>(),
        vec!["group-2"]
    );
    assert!(!format!("{repository:?}").contains("Editors"));
    cleanup_temp_dir(root);
}

#[test]
fn local_group_repository_persists_membership_idempotency_across_instances() {
    let root = unique_temp_dir("local-group-repository-membership");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let group_id = GroupId::new("group-1").expect("group id");
    let user_id = UserId::new("user-1").expect("user id");
    let membership = GroupMembership::new(group_id.clone(), user_id.clone());
    let mut repository = LocalGroupRepository::new(root.clone());

    assert_eq!(
        repository
            .add_membership(&workspace_id, membership.clone())
            .expect("add membership"),
        MembershipMutationResult::Changed
    );
    assert_eq!(
        repository
            .add_membership(&workspace_id, membership.clone())
            .expect("add membership again"),
        MembershipMutationResult::AlreadyApplied
    );
    assert!(
        repository
            .has_membership(&workspace_id, &group_id, &user_id)
            .expect("has membership")
    );

    let mut restarted = LocalGroupRepository::new(root.clone());
    assert!(
        restarted
            .has_membership(&workspace_id, &group_id, &user_id)
            .expect("has after restart")
    );
    assert_eq!(
        restarted
            .list_workspace_memberships(&workspace_id)
            .expect("list memberships")
            .len(),
        1
    );
    assert_eq!(
        restarted
            .remove_membership(&workspace_id, &group_id, &user_id)
            .expect("remove membership"),
        MembershipMutationResult::Changed
    );
    assert_eq!(
        restarted
            .remove_membership(&workspace_id, &group_id, &user_id)
            .expect("remove missing membership"),
        MembershipMutationResult::Missing
    );
    assert!(
        !LocalGroupRepository::new(root.clone())
            .has_membership(&workspace_id, &group_id, &user_id)
            .expect("missing after restart")
    );
    cleanup_temp_dir(root);
}

#[test]
fn local_group_repository_reports_conflict_and_corrupted_records() {
    let root = unique_temp_dir("local-group-repository-errors");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let group = group("group-1", &workspace_id, "Editors");
    let mut repository = LocalGroupRepository::new(root.clone());

    repository
        .save_group(&workspace_id, group.clone())
        .expect("save group");
    let duplicate = repository
        .save_group(&workspace_id, group.clone())
        .expect_err("duplicate group must conflict");
    let group_file = first_file_under(&root.join("groups"), "group");
    fs::write(group_file, "not-a-group-record").expect("corrupt group file");
    let corrupted_group = repository
        .get_group(&workspace_id, group.id())
        .expect_err("corrupted group must fail");

    let mut membership_repository = LocalGroupRepository::new(root.clone());
    membership_repository
        .add_membership(
            &workspace_id,
            GroupMembership::new(
                GroupId::new("group-2").expect("group id"),
                UserId::new("user-1").expect("user id"),
            ),
        )
        .expect("add membership");
    let membership_file = first_file_under(&root.join("groups"), "member");
    fs::write(membership_file, "not-a-membership-record").expect("corrupt membership file");
    let corrupted_membership = membership_repository
        .list_workspace_memberships(&workspace_id)
        .expect_err("corrupted membership must fail");

    assert_eq!(duplicate, GroupRepositoryError::Conflict);
    assert_eq!(corrupted_group, GroupRepositoryError::StorageUnavailable);
    assert_eq!(
        corrupted_membership,
        GroupRepositoryError::StorageUnavailable
    );
    cleanup_temp_dir(root);
}

fn group(group_id: &str, workspace_id: &WorkspaceId, name: &str) -> Group {
    Group::new(
        GroupId::new(group_id).expect("group id"),
        workspace_id.clone(),
        GroupName::new(name).expect("group name"),
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
