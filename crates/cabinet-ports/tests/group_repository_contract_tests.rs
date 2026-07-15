use std::collections::{HashMap, HashSet};

use cabinet_domain::group::{Group, GroupId, GroupMembership, GroupName};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::group_repository::{
    GroupIdGenerator, GroupRepository, GroupRepositoryError, MembershipMutationResult,
};

#[derive(Default)]
struct FakeGroupRepository {
    groups: HashMap<String, Group>,
    memberships: HashSet<(String, String, String)>,
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
struct FakeGroupIdGenerator {
    next: u32,
}

impl GroupIdGenerator for FakeGroupIdGenerator {
    fn generate_group_id(&mut self) -> String {
        self.next += 1;
        format!("group-{}", self.next)
    }
}

#[test]
fn group_repository_contract_handles_group_lookup_and_membership_idempotency() {
    let mut repository = FakeGroupRepository::default();
    let mut id_generator = FakeGroupIdGenerator::default();
    let workspace_id = WorkspaceId::new("workspace-1").expect("valid workspace id");
    let group = Group::new(
        GroupId::new(&id_generator.generate_group_id()).expect("valid group id"),
        workspace_id.clone(),
        GroupName::new("Editors").expect("valid group name"),
    );
    let user_id = UserId::new("user-1").expect("valid user id");

    repository
        .save_group(&workspace_id, group.clone())
        .expect("save group");

    assert!(
        repository
            .find_group_by_name(
                &workspace_id,
                &GroupName::new("editors").expect("valid name"),
            )
            .expect("lookup")
            .is_some()
    );
    assert_eq!(
        repository
            .add_membership(
                &workspace_id,
                GroupMembership::new(group.id().clone(), user_id.clone()),
            )
            .expect("add member"),
        MembershipMutationResult::Changed
    );
    assert_eq!(
        repository
            .add_membership(
                &workspace_id,
                GroupMembership::new(group.id().clone(), user_id.clone()),
            )
            .expect("add same member"),
        MembershipMutationResult::AlreadyApplied
    );
    assert_eq!(
        repository
            .list_workspace_memberships(&workspace_id)
            .expect("list members")
            .len(),
        1
    );
    assert_eq!(
        repository
            .list_workspace_groups(&workspace_id)
            .expect("list groups")
            .len(),
        1
    );
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
