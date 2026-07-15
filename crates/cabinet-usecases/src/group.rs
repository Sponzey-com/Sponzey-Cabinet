use cabinet_domain::group::{Group, GroupId, GroupMembership, GroupName};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::group_repository::{
    GroupIdGenerator, GroupRepository, GroupRepositoryError, MembershipMutationResult,
};
use cabinet_ports::user_repository::{UserRepository, UserRepositoryError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateGroupInput {
    workspace_id: String,
    name: String,
}

impl CreateGroupInput {
    pub fn new(workspace_id: &str, name: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            name: name.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateGroupOutput {
    group: Group,
}

impl CreateGroupOutput {
    pub fn group(&self) -> &Group {
        &self.group
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddUserToGroupInput {
    workspace_id: String,
    group_id: String,
    user_id: String,
}

impl AddUserToGroupInput {
    pub fn new(workspace_id: &str, group_id: &str, user_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            group_id: group_id.to_string(),
            user_id: user_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddUserToGroupOutput {
    group_id: String,
    user_id: String,
    result: GroupMembershipResult,
}

impl AddUserToGroupOutput {
    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub const fn result(&self) -> GroupMembershipResult {
        self.result
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveUserFromGroupInput {
    workspace_id: String,
    group_id: String,
    user_id: String,
}

impl RemoveUserFromGroupInput {
    pub fn new(workspace_id: &str, group_id: &str, user_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            group_id: group_id.to_string(),
            user_id: user_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveUserFromGroupOutput {
    group_id: String,
    user_id: String,
    result: GroupMembershipResult,
}

impl RemoveUserFromGroupOutput {
    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub const fn result(&self) -> GroupMembershipResult {
        self.result
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListWorkspaceMembersInput {
    workspace_id: String,
}

impl ListWorkspaceMembersInput {
    pub fn new(workspace_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListWorkspaceMembersOutput {
    members: Vec<WorkspaceMemberDto>,
}

impl ListWorkspaceMembersOutput {
    pub fn members(&self) -> &[WorkspaceMemberDto] {
        &self.members
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMemberDto {
    group_id: String,
    user_id: String,
}

impl WorkspaceMemberDto {
    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMembershipResult {
    Added,
    AlreadyMember,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateGroupProductEvent {
    GroupCreated {
        workspace_id: String,
        masked_group_id: String,
        member_count: usize,
    },
    MembershipAdded {
        workspace_id: String,
        masked_group_id: String,
        masked_user_id: String,
        member_count: usize,
    },
    MembershipRemoved {
        workspace_id: String,
        masked_group_id: String,
        masked_user_id: String,
        member_count: usize,
    },
    UsecaseFailed {
        error_code: &'static str,
    },
}

impl CreateGroupProductEvent {
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::GroupCreated { .. } => "group.created",
            Self::MembershipAdded { .. } => "group.membership.added",
            Self::MembershipRemoved { .. } => "group.membership.removed",
            Self::UsecaseFailed { .. } => "group.usecase.failed",
        }
    }
}

pub trait CreateGroupProductLogger {
    fn write_product(&mut self, event: CreateGroupProductEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateGroupUsecase;

impl CreateGroupUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: CreateGroupInput,
        repository: &mut impl GroupRepository,
        id_generator: &mut impl GroupIdGenerator,
        product_logger: &mut impl CreateGroupProductLogger,
    ) -> Result<CreateGroupOutput, CreateGroupError> {
        let workspace_id = parse_workspace_id(&input.workspace_id).map_err(|error| {
            log_group_error_code(product_logger, error.code());
            error
        })?;
        let group_name = GroupName::new(&input.name).map_err(|_| {
            log_group_error_code(product_logger, CreateGroupError::InvalidInput.code());
            CreateGroupError::InvalidInput
        })?;

        match repository.find_group_by_name(&workspace_id, &group_name) {
            Ok(Some(_)) => {
                log_group_error_code(product_logger, CreateGroupError::GroupAlreadyExists.code());
                return Err(CreateGroupError::GroupAlreadyExists);
            }
            Ok(None) => {}
            Err(error) => {
                let mapped = CreateGroupError::from_group_repository_error(error);
                log_group_error_code(product_logger, mapped.code());
                return Err(mapped);
            }
        }

        let group_id = GroupId::new(&id_generator.generate_group_id()).map_err(|_| {
            log_group_error_code(product_logger, CreateGroupError::InvalidInput.code());
            CreateGroupError::InvalidInput
        })?;
        let group = Group::new(group_id, workspace_id.clone(), group_name);
        if let Err(error) = repository.save_group(&workspace_id, group.clone()) {
            let mapped = CreateGroupError::from_group_repository_error(error);
            log_group_error_code(product_logger, mapped.code());
            return Err(mapped);
        }

        product_logger.write_product(CreateGroupProductEvent::GroupCreated {
            workspace_id: workspace_id.as_str().to_string(),
            masked_group_id: mask_group_id(group.id()),
            member_count: 0,
        });
        Ok(CreateGroupOutput { group })
    }
}

impl Default for CreateGroupUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddUserToGroupUsecase;

impl AddUserToGroupUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: AddUserToGroupInput,
        group_repository: &mut impl GroupRepository,
        user_repository: &impl UserRepository,
        product_logger: &mut impl CreateGroupProductLogger,
    ) -> Result<AddUserToGroupOutput, AddUserToGroupError> {
        let (workspace_id, group_id, user_id) =
            parse_membership_ids(&input.workspace_id, &input.group_id, &input.user_id).map_err(
                |error| {
                    log_group_error_code(product_logger, error.code());
                    error
                },
            )?;
        ensure_group_exists(group_repository, &workspace_id, &group_id).map_err(|error| {
            log_group_error_code(product_logger, error.code());
            error
        })?;
        ensure_user_exists(user_repository, &user_id).map_err(|error| {
            log_group_error_code(product_logger, error.code());
            error
        })?;

        let mutation = group_repository
            .add_membership(
                &workspace_id,
                GroupMembership::new(group_id.clone(), user_id.clone()),
            )
            .map_err(|error| {
                let mapped = AddUserToGroupError::from_group_repository_error(error);
                log_group_error_code(product_logger, mapped.code());
                mapped
            })?;
        let result = match mutation {
            MembershipMutationResult::Changed => {
                product_logger.write_product(CreateGroupProductEvent::MembershipAdded {
                    workspace_id: workspace_id.as_str().to_string(),
                    masked_group_id: mask_group_id(&group_id),
                    masked_user_id: mask_user_id(&user_id),
                    member_count: count_group_members(group_repository, &workspace_id, &group_id),
                });
                GroupMembershipResult::Added
            }
            MembershipMutationResult::AlreadyApplied | MembershipMutationResult::Missing => {
                GroupMembershipResult::AlreadyMember
            }
        };

        Ok(AddUserToGroupOutput {
            group_id: group_id.as_str().to_string(),
            user_id: user_id.as_str().to_string(),
            result,
        })
    }
}

impl Default for AddUserToGroupUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveUserFromGroupUsecase;

impl RemoveUserFromGroupUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: RemoveUserFromGroupInput,
        group_repository: &mut impl GroupRepository,
        user_repository: &impl UserRepository,
        product_logger: &mut impl CreateGroupProductLogger,
    ) -> Result<RemoveUserFromGroupOutput, RemoveUserFromGroupError> {
        let (workspace_id, group_id, user_id) =
            parse_membership_ids(&input.workspace_id, &input.group_id, &input.user_id).map_err(
                |error| {
                    let mapped = RemoveUserFromGroupError::from_add_error(error);
                    log_group_error_code(product_logger, mapped.code());
                    mapped
                },
            )?;
        ensure_group_exists(group_repository, &workspace_id, &group_id).map_err(|error| {
            let mapped = RemoveUserFromGroupError::from_add_error(error);
            log_group_error_code(product_logger, mapped.code());
            mapped
        })?;
        ensure_user_exists(user_repository, &user_id).map_err(|error| {
            let mapped = RemoveUserFromGroupError::from_add_error(error);
            log_group_error_code(product_logger, mapped.code());
            mapped
        })?;

        let mutation = group_repository
            .remove_membership(&workspace_id, &group_id, &user_id)
            .map_err(|error| {
                let mapped = RemoveUserFromGroupError::from_group_repository_error(error);
                log_group_error_code(product_logger, mapped.code());
                mapped
            })?;
        if mutation != MembershipMutationResult::Changed {
            log_group_error_code(
                product_logger,
                RemoveUserFromGroupError::MembershipNotFound.code(),
            );
            return Err(RemoveUserFromGroupError::MembershipNotFound);
        }

        product_logger.write_product(CreateGroupProductEvent::MembershipRemoved {
            workspace_id: workspace_id.as_str().to_string(),
            masked_group_id: mask_group_id(&group_id),
            masked_user_id: mask_user_id(&user_id),
            member_count: count_group_members(group_repository, &workspace_id, &group_id),
        });
        Ok(RemoveUserFromGroupOutput {
            group_id: group_id.as_str().to_string(),
            user_id: user_id.as_str().to_string(),
            result: GroupMembershipResult::Removed,
        })
    }
}

impl Default for RemoveUserFromGroupUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListWorkspaceMembersUsecase;

impl ListWorkspaceMembersUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ListWorkspaceMembersInput,
        repository: &impl GroupRepository,
    ) -> Result<ListWorkspaceMembersOutput, ListWorkspaceMembersError> {
        let workspace_id = parse_workspace_id(&input.workspace_id)
            .map_err(|_| ListWorkspaceMembersError::InvalidInput)?;
        let members = repository
            .list_workspace_memberships(&workspace_id)
            .map_err(|_| ListWorkspaceMembersError::StorageUnavailable)?
            .into_iter()
            .map(|membership| WorkspaceMemberDto {
                group_id: membership.group_id().as_str().to_string(),
                user_id: membership.user_id().as_str().to_string(),
            })
            .collect();
        Ok(ListWorkspaceMembersOutput { members })
    }
}

impl Default for ListWorkspaceMembersUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListWorkspaceGroupsInput {
    workspace_id: String,
}

impl ListWorkspaceGroupsInput {
    pub fn new(workspace_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListWorkspaceGroupsOutput {
    groups: Vec<WorkspaceGroupDto>,
}

impl ListWorkspaceGroupsOutput {
    pub fn groups(&self) -> &[WorkspaceGroupDto] {
        &self.groups
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceGroupDto {
    group_id: String,
    name: String,
    member_user_ids: Vec<String>,
}

impl WorkspaceGroupDto {
    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn member_user_ids(&self) -> &[String] {
        &self.member_user_ids
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListWorkspaceGroupsUsecase;

impl ListWorkspaceGroupsUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ListWorkspaceGroupsInput,
        repository: &impl GroupRepository,
    ) -> Result<ListWorkspaceGroupsOutput, ListWorkspaceGroupsError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ListWorkspaceGroupsError::InvalidInput)?;
        let groups = repository
            .list_workspace_groups(&workspace_id)
            .map_err(|_| ListWorkspaceGroupsError::StorageUnavailable)?;
        let memberships = repository
            .list_workspace_memberships(&workspace_id)
            .map_err(|_| ListWorkspaceGroupsError::StorageUnavailable)?;
        let summaries = groups
            .into_iter()
            .map(|group| {
                let mut member_user_ids = memberships
                    .iter()
                    .filter(|membership| membership.group_id() == group.id())
                    .map(|membership| membership.user_id().as_str().to_string())
                    .collect::<Vec<_>>();
                member_user_ids.sort();
                WorkspaceGroupDto {
                    group_id: group.id().as_str().to_string(),
                    name: group.name().as_str().to_string(),
                    member_user_ids,
                }
            })
            .collect();
        Ok(ListWorkspaceGroupsOutput { groups: summaries })
    }
}

impl Default for ListWorkspaceGroupsUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateGroupError {
    InvalidInput,
    GroupAlreadyExists,
    StorageUnavailable,
}

impl CreateGroupError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_GROUP_INPUT",
            Self::GroupAlreadyExists => "GROUP_ALREADY_EXISTS",
            Self::StorageUnavailable => "GROUP_STORAGE_UNAVAILABLE",
        }
    }

    const fn from_group_repository_error(error: GroupRepositoryError) -> Self {
        match error {
            GroupRepositoryError::Conflict => Self::GroupAlreadyExists,
            GroupRepositoryError::NotFound | GroupRepositoryError::StorageUnavailable => {
                Self::StorageUnavailable
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddUserToGroupError {
    InvalidInput,
    GroupNotFound,
    UserNotFound,
    StorageUnavailable,
}

impl AddUserToGroupError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_GROUP_MEMBERSHIP_INPUT",
            Self::GroupNotFound => "GROUP_NOT_FOUND",
            Self::UserNotFound => "USER_NOT_FOUND",
            Self::StorageUnavailable => "GROUP_STORAGE_UNAVAILABLE",
        }
    }

    const fn from_group_repository_error(error: GroupRepositoryError) -> Self {
        match error {
            GroupRepositoryError::NotFound => Self::GroupNotFound,
            GroupRepositoryError::Conflict | GroupRepositoryError::StorageUnavailable => {
                Self::StorageUnavailable
            }
        }
    }

    const fn from_user_repository_error(error: UserRepositoryError) -> Self {
        match error {
            UserRepositoryError::NotFound => Self::UserNotFound,
            UserRepositoryError::Conflict | UserRepositoryError::StorageUnavailable => {
                Self::StorageUnavailable
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveUserFromGroupError {
    InvalidInput,
    GroupNotFound,
    UserNotFound,
    MembershipNotFound,
    StorageUnavailable,
}

impl RemoveUserFromGroupError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_GROUP_MEMBERSHIP_INPUT",
            Self::GroupNotFound => "GROUP_NOT_FOUND",
            Self::UserNotFound => "USER_NOT_FOUND",
            Self::MembershipNotFound => "MEMBERSHIP_NOT_FOUND",
            Self::StorageUnavailable => "GROUP_STORAGE_UNAVAILABLE",
        }
    }

    const fn from_add_error(error: AddUserToGroupError) -> Self {
        match error {
            AddUserToGroupError::InvalidInput => Self::InvalidInput,
            AddUserToGroupError::GroupNotFound => Self::GroupNotFound,
            AddUserToGroupError::UserNotFound => Self::UserNotFound,
            AddUserToGroupError::StorageUnavailable => Self::StorageUnavailable,
        }
    }

    const fn from_group_repository_error(error: GroupRepositoryError) -> Self {
        match error {
            GroupRepositoryError::NotFound => Self::MembershipNotFound,
            GroupRepositoryError::Conflict | GroupRepositoryError::StorageUnavailable => {
                Self::StorageUnavailable
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListWorkspaceMembersError {
    InvalidInput,
    StorageUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListWorkspaceGroupsError {
    InvalidInput,
    StorageUnavailable,
}

impl ListWorkspaceGroupsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_GROUP_INPUT",
            Self::StorageUnavailable => "GROUP_STORAGE_UNAVAILABLE",
        }
    }
}

fn parse_workspace_id(value: &str) -> Result<WorkspaceId, CreateGroupError> {
    WorkspaceId::new(value).map_err(|_| CreateGroupError::InvalidInput)
}

fn parse_membership_ids(
    workspace_id: &str,
    group_id: &str,
    user_id: &str,
) -> Result<(WorkspaceId, GroupId, UserId), AddUserToGroupError> {
    Ok((
        WorkspaceId::new(workspace_id).map_err(|_| AddUserToGroupError::InvalidInput)?,
        GroupId::new(group_id).map_err(|_| AddUserToGroupError::InvalidInput)?,
        UserId::new(user_id).map_err(|_| AddUserToGroupError::InvalidInput)?,
    ))
}

fn ensure_group_exists(
    repository: &impl GroupRepository,
    workspace_id: &WorkspaceId,
    group_id: &GroupId,
) -> Result<Group, AddUserToGroupError> {
    repository
        .get_group(workspace_id, group_id)
        .map_err(AddUserToGroupError::from_group_repository_error)?
        .ok_or(AddUserToGroupError::GroupNotFound)
}

fn ensure_user_exists(
    repository: &impl UserRepository,
    user_id: &UserId,
) -> Result<(), AddUserToGroupError> {
    repository
        .get_user(user_id)
        .map_err(AddUserToGroupError::from_user_repository_error)?
        .map(|_| ())
        .ok_or(AddUserToGroupError::UserNotFound)
}

fn count_group_members(
    repository: &impl GroupRepository,
    workspace_id: &WorkspaceId,
    group_id: &GroupId,
) -> usize {
    repository
        .list_workspace_memberships(workspace_id)
        .map(|members| {
            members
                .iter()
                .filter(|membership| membership.group_id() == group_id)
                .count()
        })
        .unwrap_or(0)
}

fn log_group_error_code(
    product_logger: &mut impl CreateGroupProductLogger,
    error_code: &'static str,
) {
    product_logger.write_product(CreateGroupProductEvent::UsecaseFailed { error_code });
}

fn mask_group_id(group_id: &GroupId) -> String {
    format!("masked:{}", group_id.as_str())
}

fn mask_user_id(user_id: &UserId) -> String {
    format!("masked:{}", user_id.as_str())
}
