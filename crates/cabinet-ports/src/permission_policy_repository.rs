use cabinet_domain::document::DocumentId;
use cabinet_domain::group::GroupId;
use cabinet_domain::permission::{
    CollectionId, CollectionPolicy, DocumentPolicy, RoleAssignment, RoleAssignmentId,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;

pub trait PermissionPolicyRepository {
    fn list_user_role_assignments(
        &self,
        workspace_id: &WorkspaceId,
        user_id: &UserId,
    ) -> Result<Vec<RoleAssignment>, PermissionRepositoryError>;

    fn list_group_role_assignments(
        &self,
        workspace_id: &WorkspaceId,
        group_ids: &[GroupId],
    ) -> Result<Vec<RoleAssignment>, PermissionRepositoryError>;

    fn list_workspace_role_assignments(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<RoleAssignment>, PermissionRepositoryError>;

    fn get_role_assignment(
        &self,
        workspace_id: &WorkspaceId,
        assignment_id: &RoleAssignmentId,
    ) -> Result<Option<RoleAssignment>, PermissionRepositoryError>;

    fn save_role_assignment(
        &mut self,
        workspace_id: &WorkspaceId,
        assignment: RoleAssignment,
    ) -> Result<RoleAssignmentMutationResult, PermissionRepositoryError>;

    fn remove_role_assignment(
        &mut self,
        workspace_id: &WorkspaceId,
        assignment_id: &RoleAssignmentId,
    ) -> Result<RoleAssignmentRemovalResult, PermissionRepositoryError>;

    fn get_collection_policy(
        &self,
        workspace_id: &WorkspaceId,
        collection_id: &CollectionId,
    ) -> Result<Option<CollectionPolicy>, PermissionRepositoryError>;

    fn save_collection_policy(
        &mut self,
        workspace_id: &WorkspaceId,
        policy: CollectionPolicy,
    ) -> Result<(), PermissionRepositoryError>;

    fn get_document_policy(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentPolicy>, PermissionRepositoryError>;

    fn save_document_policy(
        &mut self,
        workspace_id: &WorkspaceId,
        policy: DocumentPolicy,
    ) -> Result<(), PermissionRepositoryError>;
}

pub trait PermissionGroupRepository {
    fn list_user_group_ids(
        &self,
        workspace_id: &WorkspaceId,
        user_id: &UserId,
    ) -> Result<Vec<GroupId>, PermissionRepositoryError>;
}

pub trait RoleAssignmentIdGenerator {
    fn generate_role_assignment_id(&mut self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleAssignmentMutationResult {
    Changed,
    AlreadyApplied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleAssignmentRemovalResult {
    Removed,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionRepositoryError {
    Conflict,
    NotFound,
    StorageUnavailable,
}

impl PermissionRepositoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Conflict => "permission_repository.conflict",
            Self::NotFound => "permission_repository.not_found",
            Self::StorageUnavailable => "permission_repository.storage_unavailable",
        }
    }
}
