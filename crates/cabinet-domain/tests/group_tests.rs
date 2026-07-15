use cabinet_domain::group::{Group, GroupId, GroupMembership, GroupName};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;

#[test]
fn group_validates_identity_workspace_and_name() {
    let group = Group::new(
        GroupId::new("group-1").expect("valid group id"),
        WorkspaceId::new("workspace-1").expect("valid workspace id"),
        GroupName::new("Editors").expect("valid group name"),
    );

    assert_eq!(group.id().as_str(), "group-1");
    assert_eq!(group.workspace_id().as_str(), "workspace-1");
    assert_eq!(group.name().as_str(), "Editors");
    assert_eq!(group.name().duplicate_key(), "editors");
}

#[test]
fn group_name_rejects_empty_too_long_or_control_character_values() {
    assert!(GroupName::new(" ").is_err());
    assert!(GroupName::new("a".repeat(81).as_str()).is_err());
    assert!(GroupName::new("ops\nteam").is_err());
}

#[test]
fn group_membership_keeps_group_and_user_identity() {
    let membership = GroupMembership::new(
        GroupId::new("group-1").expect("valid group id"),
        UserId::new("user-1").expect("valid user id"),
    );

    assert_eq!(membership.group_id().as_str(), "group-1");
    assert_eq!(membership.user_id().as_str(), "user-1");
}
