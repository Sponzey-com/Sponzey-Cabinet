use std::collections::{HashMap, HashSet};

use cabinet_domain::group::{Group, GroupId, GroupMembership, GroupName};
use cabinet_domain::user::{User, UserEmail, UserExternalIdentity, UserId, UserLogin, UserProfile};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::group_repository::{
    GroupIdGenerator, GroupRepository, GroupRepositoryError, MembershipMutationResult,
};
use cabinet_ports::user_repository::{UserRepository, UserRepositoryError};
use cabinet_usecases::group::{
    AddUserToGroupInput, AddUserToGroupUsecase, CreateGroupError, CreateGroupInput,
    CreateGroupProductEvent, CreateGroupProductLogger, CreateGroupUsecase, GroupMembershipResult,
    ListWorkspaceGroupsError, ListWorkspaceGroupsInput, ListWorkspaceGroupsUsecase,
    ListWorkspaceMembersInput, ListWorkspaceMembersUsecase, RemoveUserFromGroupError,
    RemoveUserFromGroupInput, RemoveUserFromGroupUsecase,
};

#[derive(Default)]
struct FakeGroupRepository {
    groups: HashMap<String, Group>,
    memberships: HashSet<(String, String, String)>,
    fail_list_groups: bool,
}

impl GroupRepository for FakeGroupRepository {
    fn find_group_by_name(
        &self,
        workspace_id: &WorkspaceId,
        name: &GroupName,
    ) -> Result<Option<Group>, GroupRepositoryError> {
        Ok(self
            .groups
            .values()
            .find(|group| {
                group.workspace_id() == workspace_id
                    && group.name().duplicate_key() == name.duplicate_key()
            })
            .cloned())
    }

    fn get_group(
        &self,
        workspace_id: &WorkspaceId,
        group_id: &GroupId,
    ) -> Result<Option<Group>, GroupRepositoryError> {
        Ok(self.groups.get(&group_key(workspace_id, group_id)).cloned())
    }

    fn save_group(
        &mut self,
        workspace_id: &WorkspaceId,
        group: Group,
    ) -> Result<(), GroupRepositoryError> {
        if self
            .groups
            .contains_key(&group_key(workspace_id, group.id()))
        {
            return Err(GroupRepositoryError::Conflict);
        }
        self.groups
            .insert(group_key(workspace_id, group.id()), group);
        Ok(())
    }

    fn has_membership(
        &self,
        workspace_id: &WorkspaceId,
        group_id: &GroupId,
        user_id: &UserId,
    ) -> Result<bool, GroupRepositoryError> {
        Ok(self
            .memberships
            .contains(&membership_key(workspace_id, group_id, user_id)))
    }

    fn add_membership(
        &mut self,
        workspace_id: &WorkspaceId,
        membership: GroupMembership,
    ) -> Result<MembershipMutationResult, GroupRepositoryError> {
        let inserted = self.memberships.insert(membership_key(
            workspace_id,
            membership.group_id(),
            membership.user_id(),
        ));
        Ok(if inserted {
            MembershipMutationResult::Changed
        } else {
            MembershipMutationResult::AlreadyApplied
        })
    }

    fn remove_membership(
        &mut self,
        workspace_id: &WorkspaceId,
        group_id: &GroupId,
        user_id: &UserId,
    ) -> Result<MembershipMutationResult, GroupRepositoryError> {
        let removed = self
            .memberships
            .remove(&membership_key(workspace_id, group_id, user_id));
        Ok(if removed {
            MembershipMutationResult::Changed
        } else {
            MembershipMutationResult::Missing
        })
    }

    fn list_workspace_memberships(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<GroupMembership>, GroupRepositoryError> {
        Ok(self
            .memberships
            .iter()
            .filter(|(stored_workspace_id, _, _)| stored_workspace_id == workspace_id.as_str())
            .map(|(_, group_id, user_id)| {
                GroupMembership::new(
                    GroupId::new(group_id).expect("stored group id"),
                    UserId::new(user_id).expect("stored user id"),
                )
            })
            .collect())
    }

    fn list_workspace_groups(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<Group>, GroupRepositoryError> {
        if self.fail_list_groups {
            return Err(GroupRepositoryError::StorageUnavailable);
        }
        let mut groups = self
            .groups
            .values()
            .filter(|group| group.workspace_id() == workspace_id)
            .cloned()
            .collect::<Vec<_>>();
        groups.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        Ok(groups)
    }
}

#[derive(Default)]
struct FakeUserRepository {
    users: HashMap<String, User>,
}

impl UserRepository for FakeUserRepository {
    fn find_by_identity(
        &self,
        _login: &UserLogin,
        _email: &UserEmail,
        _external_identity: Option<&UserExternalIdentity>,
    ) -> Result<Option<User>, UserRepositoryError> {
        Ok(None)
    }

    fn get_user(&self, user_id: &UserId) -> Result<Option<User>, UserRepositoryError> {
        Ok(self.users.get(user_id.as_str()).cloned())
    }

    fn save(&mut self, user: User) -> Result<(), UserRepositoryError> {
        self.users.insert(user.id().as_str().to_string(), user);
        Ok(())
    }

    fn update_status(&mut self, user: User) -> Result<(), UserRepositoryError> {
        self.users.insert(user.id().as_str().to_string(), user);
        Ok(())
    }

    fn list_users(&self) -> Result<Vec<User>, UserRepositoryError> {
        Ok(self.users.values().cloned().collect())
    }
}

#[derive(Default)]
struct FakeGroupIdGenerator {
    next: u32,
}

impl GroupIdGenerator for FakeGroupIdGenerator {
    fn generate_group_id(&mut self) -> String {
        self.next += 1;
        format!("group-{}", self.next)
    }
}

#[derive(Default)]
struct FakeProductLogger {
    events: Vec<CreateGroupProductEvent>,
}

impl CreateGroupProductLogger for FakeProductLogger {
    fn write_product(&mut self, event: CreateGroupProductEvent) {
        self.events.push(event);
    }
}

#[test]
fn create_group_rejects_duplicate_name_within_same_workspace() {
    let mut groups = FakeGroupRepository::default();
    let mut ids = FakeGroupIdGenerator::default();
    let mut logger = FakeProductLogger::default();
    let usecase = CreateGroupUsecase::new();

    let first = usecase
        .execute(
            CreateGroupInput::new("workspace-1", "Editors"),
            &mut groups,
            &mut ids,
            &mut logger,
        )
        .expect("create group");
    let error = usecase
        .execute(
            CreateGroupInput::new("workspace-1", "editors"),
            &mut groups,
            &mut ids,
            &mut logger,
        )
        .expect_err("duplicate group name must fail");

    assert_eq!(first.group().id().as_str(), "group-1");
    assert_eq!(error, CreateGroupError::GroupAlreadyExists);
    assert_eq!(groups.groups.len(), 1);
    assert_eq!(
        logger.events.last(),
        Some(&CreateGroupProductEvent::UsecaseFailed {
            error_code: "GROUP_ALREADY_EXISTS",
        })
    );
}

#[test]
fn add_user_to_group_is_idempotent_for_existing_membership() {
    let mut groups = repository_with_group("workspace-1", "group-1", "Editors");
    let users = repository_with_user("user-1");
    let mut logger = FakeProductLogger::default();
    let usecase = AddUserToGroupUsecase::new();

    let first = usecase
        .execute(
            AddUserToGroupInput::new("workspace-1", "group-1", "user-1"),
            &mut groups,
            &users,
            &mut logger,
        )
        .expect("add member");
    let second = usecase
        .execute(
            AddUserToGroupInput::new("workspace-1", "group-1", "user-1"),
            &mut groups,
            &users,
            &mut logger,
        )
        .expect("add same member");

    assert_eq!(first.result(), GroupMembershipResult::Added);
    assert_eq!(second.result(), GroupMembershipResult::AlreadyMember);
    assert_eq!(groups.memberships.len(), 1);
}

#[test]
fn remove_user_from_group_reports_missing_member_with_stable_error() {
    let mut groups = repository_with_group("workspace-1", "group-1", "Editors");
    let users = repository_with_user("user-1");
    let mut logger = FakeProductLogger::default();

    let error = RemoveUserFromGroupUsecase::new()
        .execute(
            RemoveUserFromGroupInput::new("workspace-1", "group-1", "user-1"),
            &mut groups,
            &users,
            &mut logger,
        )
        .expect_err("missing membership must fail");

    assert_eq!(error, RemoveUserFromGroupError::MembershipNotFound);
    assert_eq!(
        logger.events,
        vec![CreateGroupProductEvent::UsecaseFailed {
            error_code: "MEMBERSHIP_NOT_FOUND",
        }]
    );
}

#[test]
fn remove_user_from_group_reports_missing_group_and_user_with_stable_errors() {
    let users = repository_with_user("user-1");
    let mut logger = FakeProductLogger::default();
    let missing_group_error = RemoveUserFromGroupUsecase::new()
        .execute(
            RemoveUserFromGroupInput::new("workspace-1", "missing-group", "user-1"),
            &mut FakeGroupRepository::default(),
            &users,
            &mut logger,
        )
        .expect_err("missing group must fail");

    assert_eq!(missing_group_error, RemoveUserFromGroupError::GroupNotFound);
    assert_eq!(
        logger.events.last(),
        Some(&CreateGroupProductEvent::UsecaseFailed {
            error_code: "GROUP_NOT_FOUND",
        })
    );

    let mut groups = repository_with_group("workspace-1", "group-1", "Editors");
    let missing_user_error = RemoveUserFromGroupUsecase::new()
        .execute(
            RemoveUserFromGroupInput::new("workspace-1", "group-1", "missing-user"),
            &mut groups,
            &FakeUserRepository::default(),
            &mut logger,
        )
        .expect_err("missing user must fail");

    assert_eq!(missing_user_error, RemoveUserFromGroupError::UserNotFound);
    assert_eq!(
        logger.events.last(),
        Some(&CreateGroupProductEvent::UsecaseFailed {
            error_code: "USER_NOT_FOUND",
        })
    );
}

#[test]
fn list_workspace_members_returns_stable_membership_dtos() {
    let mut groups = repository_with_group("workspace-1", "group-1", "Editors");
    let users = repository_with_user("user-1");
    let mut logger = FakeProductLogger::default();

    AddUserToGroupUsecase::new()
        .execute(
            AddUserToGroupInput::new("workspace-1", "group-1", "user-1"),
            &mut groups,
            &users,
            &mut logger,
        )
        .expect("add member");

    let output = ListWorkspaceMembersUsecase::new()
        .execute(ListWorkspaceMembersInput::new("workspace-1"), &groups)
        .expect("list members");

    assert_eq!(output.members().len(), 1);
    assert_eq!(output.members()[0].group_id(), "group-1");
    assert_eq!(output.members()[0].user_id(), "user-1");
}

#[test]
fn list_workspace_groups_returns_group_summaries_with_member_ids() {
    let mut groups = repository_with_group("workspace-1", "group-1", "Editors");
    let users = repository_with_user("user-1");
    let mut logger = FakeProductLogger::default();

    AddUserToGroupUsecase::new()
        .execute(
            AddUserToGroupInput::new("workspace-1", "group-1", "user-1"),
            &mut groups,
            &users,
            &mut logger,
        )
        .expect("add member");

    let output = ListWorkspaceGroupsUsecase::new()
        .execute(ListWorkspaceGroupsInput::new("workspace-1"), &groups)
        .expect("list groups");

    assert_eq!(output.groups().len(), 1);
    assert_eq!(output.groups()[0].group_id(), "group-1");
    assert_eq!(output.groups()[0].name(), "Editors");
    assert_eq!(
        output.groups()[0].member_user_ids(),
        &["user-1".to_string()]
    );
    let rendered = format!("{:?}", output.groups()[0]);
    assert!(!rendered.contains("alice@example.com"));
}

#[test]
fn list_workspace_groups_maps_repository_failure_to_stable_error_code() {
    let groups = FakeGroupRepository {
        fail_list_groups: true,
        ..FakeGroupRepository::default()
    };

    let error = ListWorkspaceGroupsUsecase::new()
        .execute(ListWorkspaceGroupsInput::new("workspace-1"), &groups)
        .expect_err("list groups should fail");

    assert_eq!(error, ListWorkspaceGroupsError::StorageUnavailable);
    assert_eq!(error.code(), "GROUP_STORAGE_UNAVAILABLE");
}

#[test]
fn group_product_log_payload_excludes_group_name_and_user_email() {
    let event = CreateGroupProductEvent::MembershipAdded {
        workspace_id: "workspace-1".to_string(),
        masked_group_id: "masked:group-1".to_string(),
        masked_user_id: "masked:user-1".to_string(),
        member_count: 1,
    };
    let rendered = format!("{event:?}");

    assert_eq!(event.event_name(), "group.membership.added");
    assert!(!rendered.contains("Editors"));
    assert!(!rendered.contains("alice@example.com"));
}

fn repository_with_group(workspace_id: &str, group_id: &str, name: &str) -> FakeGroupRepository {
    let mut repository = FakeGroupRepository::default();
    let workspace_id = WorkspaceId::new(workspace_id).expect("valid workspace id");
    let group = Group::new(
        GroupId::new(group_id).expect("valid group id"),
        workspace_id.clone(),
        GroupName::new(name).expect("valid group name"),
    );
    repository
        .save_group(&workspace_id, group)
        .expect("save group");
    repository
}

fn repository_with_user(user_id: &str) -> FakeUserRepository {
    let mut repository = FakeUserRepository::default();
    repository
        .save(User::new(
            UserId::new(user_id).expect("valid user id"),
            UserProfile::new(
                UserLogin::new("alice").expect("valid login"),
                UserEmail::new("alice@example.com").expect("valid email"),
                "Alice Lee",
                None,
            )
            .expect("valid profile"),
            cabinet_domain::user::UserTimestamp::new("2026-06-25T00:00:00Z")
                .expect("valid timestamp"),
        ))
        .expect("save user");
    repository
}

fn group_key(workspace_id: &WorkspaceId, group_id: &GroupId) -> String {
    format!("{}:{}", workspace_id.as_str(), group_id.as_str())
}

fn membership_key(
    workspace_id: &WorkspaceId,
    group_id: &GroupId,
    user_id: &UserId,
) -> (String, String, String) {
    (
        workspace_id.as_str().to_string(),
        group_id.as_str().to_string(),
        user_id.as_str().to_string(),
    )
}
