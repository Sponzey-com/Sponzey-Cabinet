use cabinet_domain::group::{Group, GroupId, GroupMembership, GroupName};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;

pub trait GroupRepository {
    fn find_group_by_name(
        &self,
        workspace_id: &WorkspaceId,
        name: &GroupName,
    ) -> Result<Option<Group>, GroupRepositoryError>;

    fn get_group(
        &self,
        workspace_id: &WorkspaceId,
        group_id: &GroupId,
    ) -> Result<Option<Group>, GroupRepositoryError>;

    fn save_group(
        &mut self,
        workspace_id: &WorkspaceId,
        group: Group,
    ) -> Result<(), GroupRepositoryError>;

    fn has_membership(
        &self,
        workspace_id: &WorkspaceId,
        group_id: &GroupId,
        user_id: &UserId,
    ) -> Result<bool, GroupRepositoryError>;

    fn add_membership(
        &mut self,
        workspace_id: &WorkspaceId,
        membership: GroupMembership,
    ) -> Result<MembershipMutationResult, GroupRepositoryError>;

    fn remove_membership(
        &mut self,
        workspace_id: &WorkspaceId,
        group_id: &GroupId,
        user_id: &UserId,
    ) -> Result<MembershipMutationResult, GroupRepositoryError>;

    fn list_workspace_memberships(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<GroupMembership>, GroupRepositoryError>;

    fn list_workspace_groups(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<Group>, GroupRepositoryError>;
}

pub trait GroupIdGenerator {
    fn generate_group_id(&mut self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipMutationResult {
    Changed,
    AlreadyApplied,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupRepositoryError {
    Conflict,
    NotFound,
    StorageUnavailable,
}

impl GroupRepositoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Conflict => "group_repository.conflict",
            Self::NotFound => "group_repository.not_found",
            Self::StorageUnavailable => "group_repository.storage_unavailable",
        }
    }
}
