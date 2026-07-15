use cabinet_domain::asset::AssetId;
use cabinet_domain::document::DocumentId;
use cabinet_domain::group::GroupId;
use cabinet_domain::permission::{
    AccessResource, AccessSubject, CollectionId, CollectionPolicy, DocumentPolicy, Permission,
    PermissionDecision, PermissionDecisionResult, PolicyOverride, Role, RoleAssignment,
    RoleAssignmentId, RoleAssignmentSubject, WorkspacePolicy,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::permission_policy_repository::{
    PermissionGroupRepository, PermissionPolicyRepository, PermissionRepositoryError,
    RoleAssignmentIdGenerator, RoleAssignmentRemovalResult,
};

const ALL_PERMISSIONS: [Permission; 7] = [
    Permission::Read,
    Permission::Write,
    Permission::Review,
    Permission::Publish,
    Permission::Manage,
    Permission::ReadAssetMetadata,
    Permission::ReadAssetContent,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckPermissionInput {
    actor_user_id: String,
    resource: PermissionResourceInput,
    permission: Permission,
}

impl CheckPermissionInput {
    pub fn new(
        actor_user_id: &str,
        resource: PermissionResourceInput,
        permission: Permission,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            resource,
            permission,
        }
    }

    pub fn document(
        actor_user_id: &str,
        workspace_id: &str,
        collection_id: Option<&str>,
        document_id: &str,
        permission: Permission,
    ) -> Self {
        Self::new(
            actor_user_id,
            PermissionResourceInput::document(workspace_id, collection_id, document_id),
            permission,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckPermissionOutput {
    decision: PermissionDecision,
    role_count: usize,
}

impl CheckPermissionOutput {
    pub const fn decision(&self) -> PermissionDecision {
        self.decision
    }

    pub const fn role_count(&self) -> usize {
        self.role_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignRoleInput {
    actor_user_id: String,
    workspace_id: String,
    subject: RoleAssignmentSubjectInput,
    role: Role,
}

impl AssignRoleInput {
    pub fn for_user(
        actor_user_id: &str,
        workspace_id: &str,
        target_user_id: &str,
        role: Role,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            subject: RoleAssignmentSubjectInput::User(target_user_id.to_string()),
            role,
        }
    }

    pub fn for_group(
        actor_user_id: &str,
        workspace_id: &str,
        target_group_id: &str,
        role: Role,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            subject: RoleAssignmentSubjectInput::Group(target_group_id.to_string()),
            role,
        }
    }

    pub fn for_user_role_name(
        actor_user_id: &str,
        workspace_id: &str,
        target_user_id: &str,
        role: &str,
    ) -> Result<Self, AssignRoleError> {
        Ok(Self::for_user(
            actor_user_id,
            workspace_id,
            target_user_id,
            parse_role_name(role)?,
        ))
    }

    pub fn for_group_role_name(
        actor_user_id: &str,
        workspace_id: &str,
        target_group_id: &str,
        role: &str,
    ) -> Result<Self, AssignRoleError> {
        Ok(Self::for_group(
            actor_user_id,
            workspace_id,
            target_group_id,
            parse_role_name(role)?,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignRoleOutput {
    assignment: RoleAssignment,
}

impl AssignRoleOutput {
    pub fn assignment(&self) -> &RoleAssignment {
        &self.assignment
    }

    pub fn assignment_id(&self) -> &str {
        self.assignment.id().as_str()
    }

    pub fn subject_type(&self) -> &'static str {
        role_assignment_subject_type(self.assignment.subject())
    }

    pub fn subject_id(&self) -> &str {
        role_assignment_subject_id(self.assignment.subject())
    }

    pub const fn role(&self) -> &'static str {
        self.assignment.role().as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevokeRoleInput {
    actor_user_id: String,
    workspace_id: String,
    assignment_id: String,
}

impl RevokeRoleInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, assignment_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            assignment_id: assignment_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevokeRoleOutput {
    assignment_id: String,
}

impl RevokeRoleOutput {
    pub fn assignment_id(&self) -> &str {
        &self.assignment_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListWorkspaceRoleAssignmentsInput {
    workspace_id: String,
}

impl ListWorkspaceRoleAssignmentsInput {
    pub fn new(workspace_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListWorkspaceRoleAssignmentsOutput {
    assignments: Vec<RoleAssignmentDto>,
}

impl ListWorkspaceRoleAssignmentsOutput {
    pub fn assignments(&self) -> &[RoleAssignmentDto] {
        &self.assignments
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleAssignmentDto {
    assignment_id: String,
    subject_type: &'static str,
    subject_id: String,
    role: &'static str,
}

impl RoleAssignmentDto {
    pub fn assignment_id(&self) -> &str {
        &self.assignment_id
    }

    pub const fn subject_type(&self) -> &'static str {
        self.subject_type
    }

    pub fn subject_id(&self) -> &str {
        &self.subject_id
    }

    pub const fn role(&self) -> &'static str {
        self.role
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareDocumentInput {
    actor_user_id: String,
    workspace_id: String,
    collection_id: Option<String>,
    document_id: String,
    override_input: PolicyOverrideInput,
}

impl ShareDocumentInput {
    pub fn allow(
        actor_user_id: &str,
        workspace_id: &str,
        collection_id: Option<&str>,
        document_id: &str,
        permission: Permission,
    ) -> Self {
        Self::new(
            actor_user_id,
            workspace_id,
            collection_id,
            document_id,
            PolicyOverrideInput::allow(permission),
        )
    }

    pub fn deny(
        actor_user_id: &str,
        workspace_id: &str,
        collection_id: Option<&str>,
        document_id: &str,
        permission: Permission,
    ) -> Self {
        Self::new(
            actor_user_id,
            workspace_id,
            collection_id,
            document_id,
            PolicyOverrideInput::deny(permission),
        )
    }

    pub fn hide(
        actor_user_id: &str,
        workspace_id: &str,
        collection_id: Option<&str>,
        document_id: &str,
        permission: Permission,
    ) -> Self {
        Self::new(
            actor_user_id,
            workspace_id,
            collection_id,
            document_id,
            PolicyOverrideInput::hide(permission),
        )
    }

    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        collection_id: Option<&str>,
        document_id: &str,
        override_input: PolicyOverrideInput,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            collection_id: collection_id.map(str::to_string),
            document_id: document_id.to_string(),
            override_input,
        }
    }

    pub fn from_effect_name(
        actor_user_id: &str,
        workspace_id: &str,
        collection_id: Option<&str>,
        document_id: &str,
        permission: &str,
        effect: &str,
    ) -> Result<Self, ShareDocumentError> {
        let permission = parse_permission_name(permission)?;
        match effect {
            "allow" => Ok(Self::allow(
                actor_user_id,
                workspace_id,
                collection_id,
                document_id,
                permission,
            )),
            "deny" => Ok(Self::deny(
                actor_user_id,
                workspace_id,
                collection_id,
                document_id,
                permission,
            )),
            "hide" => Ok(Self::hide(
                actor_user_id,
                workspace_id,
                collection_id,
                document_id,
                permission,
            )),
            _ => Err(ShareDocumentError::InvalidInput),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareDocumentOutput {
    document_id: String,
}

impl ShareDocumentOutput {
    pub fn document_id(&self) -> &str {
        &self.document_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetCollectionPermissionInput {
    actor_user_id: String,
    workspace_id: String,
    collection_id: String,
    override_input: PolicyOverrideInput,
}

impl SetCollectionPermissionInput {
    pub fn allow(
        actor_user_id: &str,
        workspace_id: &str,
        collection_id: &str,
        permission: Permission,
    ) -> Self {
        Self::new(
            actor_user_id,
            workspace_id,
            collection_id,
            PolicyOverrideInput::allow(permission),
        )
    }

    pub fn deny(
        actor_user_id: &str,
        workspace_id: &str,
        collection_id: &str,
        permission: Permission,
    ) -> Self {
        Self::new(
            actor_user_id,
            workspace_id,
            collection_id,
            PolicyOverrideInput::deny(permission),
        )
    }

    pub fn hide(
        actor_user_id: &str,
        workspace_id: &str,
        collection_id: &str,
        permission: Permission,
    ) -> Self {
        Self::new(
            actor_user_id,
            workspace_id,
            collection_id,
            PolicyOverrideInput::hide(permission),
        )
    }

    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        collection_id: &str,
        override_input: PolicyOverrideInput,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            collection_id: collection_id.to_string(),
            override_input,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetCollectionPermissionOutput {
    collection_id: String,
}

impl SetCollectionPermissionOutput {
    pub fn collection_id(&self) -> &str {
        &self.collection_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListEffectivePermissionsInput {
    actor_user_id: String,
    resource: PermissionResourceInput,
}

impl ListEffectivePermissionsInput {
    pub fn new(actor_user_id: &str, resource: PermissionResourceInput) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            resource,
        }
    }

    pub fn asset(
        actor_user_id: &str,
        workspace_id: &str,
        collection_id: Option<&str>,
        document_id: Option<&str>,
        asset_id: &str,
    ) -> Self {
        Self::new(
            actor_user_id,
            PermissionResourceInput::asset(workspace_id, collection_id, document_id, asset_id),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListEffectivePermissionsOutput {
    allowed_permissions: Vec<Permission>,
}

impl ListEffectivePermissionsOutput {
    pub fn allowed_permissions(&self) -> &[Permission] {
        &self.allowed_permissions
    }

    pub fn allowed_permission_names(&self) -> Vec<&'static str> {
        self.allowed_permissions
            .iter()
            .map(|permission| permission.as_str())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionResourceInput {
    Workspace {
        workspace_id: String,
    },
    Collection {
        workspace_id: String,
        collection_id: String,
    },
    Document {
        workspace_id: String,
        collection_id: Option<String>,
        document_id: String,
    },
    Asset {
        workspace_id: String,
        collection_id: Option<String>,
        document_id: Option<String>,
        asset_id: String,
    },
}

impl PermissionResourceInput {
    pub fn workspace(workspace_id: &str) -> Self {
        Self::Workspace {
            workspace_id: workspace_id.to_string(),
        }
    }

    pub fn collection(workspace_id: &str, collection_id: &str) -> Self {
        Self::Collection {
            workspace_id: workspace_id.to_string(),
            collection_id: collection_id.to_string(),
        }
    }

    pub fn document(workspace_id: &str, collection_id: Option<&str>, document_id: &str) -> Self {
        Self::Document {
            workspace_id: workspace_id.to_string(),
            collection_id: collection_id.map(str::to_string),
            document_id: document_id.to_string(),
        }
    }

    pub fn asset(
        workspace_id: &str,
        collection_id: Option<&str>,
        document_id: Option<&str>,
        asset_id: &str,
    ) -> Self {
        Self::Asset {
            workspace_id: workspace_id.to_string(),
            collection_id: collection_id.map(str::to_string),
            document_id: document_id.map(str::to_string),
            asset_id: asset_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleAssignmentSubjectInput {
    User(String),
    Group(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyOverrideInput {
    Allow(Permission),
    Deny(Permission),
    Hide(Permission),
}

impl PolicyOverrideInput {
    pub const fn allow(permission: Permission) -> Self {
        Self::Allow(permission)
    }

    pub const fn deny(permission: Permission) -> Self {
        Self::Deny(permission)
    }

    pub const fn hide(permission: Permission) -> Self {
        Self::Hide(permission)
    }

    pub const fn permission(self) -> Permission {
        match self {
            Self::Allow(permission) | Self::Deny(permission) | Self::Hide(permission) => permission,
        }
    }

    pub const fn effect_name(self) -> &'static str {
        match self {
            Self::Allow(_) => "allow",
            Self::Deny(_) => "deny",
            Self::Hide(_) => "hide",
        }
    }

    fn to_policy_override(self) -> PolicyOverride {
        match self {
            Self::Allow(permission) => PolicyOverride::allow(permission),
            Self::Deny(permission) => PolicyOverride::deny(permission),
            Self::Hide(permission) => PolicyOverride::hide(permission),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionProductEvent {
    PermissionDenied {
        masked_actor_id: String,
        target_id: String,
        permission: &'static str,
        decision: &'static str,
        error_code: &'static str,
    },
    PolicyChanged {
        masked_actor_id: String,
        target_id: String,
        permission: &'static str,
        effect: &'static str,
    },
    RoleAssigned {
        masked_actor_id: String,
        target_id: String,
        role: &'static str,
    },
    RoleRevoked {
        masked_actor_id: String,
        target_id: String,
    },
    UsecaseFailed {
        error_code: &'static str,
    },
}

impl PermissionProductEvent {
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::PermissionDenied { .. } => "permission.denied",
            Self::PolicyChanged { .. } => "permission.policy_changed",
            Self::RoleAssigned { .. } => "role.assigned",
            Self::RoleRevoked { .. } => "role.revoked",
            Self::UsecaseFailed { .. } => "permission.usecase.failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionFieldDebugEvent {
    role_count: usize,
    policy_source: &'static str,
    decision: &'static str,
    reason_code: &'static str,
}

impl PermissionFieldDebugEvent {
    pub const fn role_count(&self) -> usize {
        self.role_count
    }

    pub const fn policy_source(&self) -> &'static str {
        self.policy_source
    }

    pub const fn decision(&self) -> &'static str {
        self.decision
    }

    pub const fn reason_code(&self) -> &'static str {
        self.reason_code
    }
}

pub trait PermissionUsecaseLogger {
    fn write_product(&mut self, event: PermissionProductEvent);
    fn write_field_debug(&mut self, event: PermissionFieldDebugEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckPermissionUsecase;

impl CheckPermissionUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: CheckPermissionInput,
        policy_repository: &impl PermissionPolicyRepository,
        group_repository: &impl PermissionGroupRepository,
        logger: &mut impl PermissionUsecaseLogger,
    ) -> Result<CheckPermissionOutput, CheckPermissionError> {
        let actor_user_id =
            parse_user_id(&input.actor_user_id).map_err(|error| log_check_error(logger, error))?;
        let resource =
            parse_resource(input.resource).map_err(|error| log_check_error(logger, error))?;
        let evaluation = evaluate_permission(
            policy_repository,
            group_repository,
            actor_user_id,
            &resource,
            input.permission,
        )
        .map_err(|error| log_check_error(logger, error))?;

        write_field_debug(logger, &evaluation);
        if evaluation.decision.result() != PermissionDecisionResult::Allowed {
            log_permission_denied(
                logger,
                evaluation.subject.user_id(),
                &resource,
                input.permission,
                evaluation.decision,
            );
        }

        Ok(CheckPermissionOutput {
            decision: evaluation.decision,
            role_count: evaluation.subject.roles().len(),
        })
    }
}

impl Default for CheckPermissionUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignRoleUsecase;

impl AssignRoleUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: AssignRoleInput,
        policy_repository: &mut impl PermissionPolicyRepository,
        group_repository: &impl PermissionGroupRepository,
        id_generator: &mut impl RoleAssignmentIdGenerator,
        logger: &mut impl PermissionUsecaseLogger,
    ) -> Result<AssignRoleOutput, AssignRoleError> {
        let actor_user_id = parse_user_id(&input.actor_user_id)
            .map_err(|error| log_assign_error(logger, AssignRoleError::from_check_error(error)))?;
        let workspace_id = parse_workspace_id(&input.workspace_id)
            .map_err(|error| log_assign_error(logger, AssignRoleError::from_check_error(error)))?;
        let workspace_resource = AccessResource::workspace(workspace_id.clone());
        let manage_decision = evaluate_permission(
            policy_repository,
            group_repository,
            actor_user_id.clone(),
            &workspace_resource,
            Permission::Manage,
        )
        .map_err(|error| log_assign_error(logger, AssignRoleError::from_check_error(error)))?;

        write_field_debug(logger, &manage_decision);
        if manage_decision.decision.result() != PermissionDecisionResult::Allowed {
            log_permission_denied(
                logger,
                &actor_user_id,
                &workspace_resource,
                Permission::Manage,
                manage_decision.decision,
            );
            return Err(AssignRoleError::Unauthorized);
        }

        let assignment_id = RoleAssignmentId::new(&id_generator.generate_role_assignment_id())
            .map_err(|_| {
                log_assign_error(logger, AssignRoleError::InvalidInput);
                AssignRoleError::InvalidInput
            })?;
        let subject = parse_role_assignment_subject(input.subject)
            .map_err(|error| log_assign_error(logger, error))?;
        let target_id = role_assignment_subject_target_id(&subject);
        let assignment =
            RoleAssignment::new(assignment_id, workspace_id.clone(), subject, input.role);

        if let Err(error) =
            policy_repository.save_role_assignment(&workspace_id, assignment.clone())
        {
            let mapped = AssignRoleError::from_repository_error(error);
            log_assign_error(logger, mapped);
            return Err(mapped);
        }

        logger.write_product(PermissionProductEvent::RoleAssigned {
            masked_actor_id: mask_user_id(&actor_user_id),
            target_id,
            role: input.role.as_str(),
        });
        Ok(AssignRoleOutput { assignment })
    }
}

impl Default for AssignRoleUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevokeRoleUsecase;

impl RevokeRoleUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: RevokeRoleInput,
        policy_repository: &mut impl PermissionPolicyRepository,
        group_repository: &impl PermissionGroupRepository,
        logger: &mut impl PermissionUsecaseLogger,
    ) -> Result<RevokeRoleOutput, RevokeRoleError> {
        let actor_user_id = parse_user_id(&input.actor_user_id)
            .map_err(|error| log_revoke_error(logger, RevokeRoleError::from_check_error(error)))?;
        let workspace_id = parse_workspace_id(&input.workspace_id)
            .map_err(|error| log_revoke_error(logger, RevokeRoleError::from_check_error(error)))?;
        let assignment_id = RoleAssignmentId::new(&input.assignment_id).map_err(|_| {
            log_revoke_error(logger, RevokeRoleError::InvalidInput);
            RevokeRoleError::InvalidInput
        })?;
        let workspace_resource = AccessResource::workspace(workspace_id.clone());
        let manage_decision = evaluate_permission(
            policy_repository,
            group_repository,
            actor_user_id.clone(),
            &workspace_resource,
            Permission::Manage,
        )
        .map_err(|error| log_revoke_error(logger, RevokeRoleError::from_check_error(error)))?;

        write_field_debug(logger, &manage_decision);
        if manage_decision.decision.result() != PermissionDecisionResult::Allowed {
            log_permission_denied(
                logger,
                &actor_user_id,
                &workspace_resource,
                Permission::Manage,
                manage_decision.decision,
            );
            return Err(RevokeRoleError::Unauthorized);
        }

        match policy_repository.remove_role_assignment(&workspace_id, &assignment_id) {
            Ok(RoleAssignmentRemovalResult::Removed) => {}
            Ok(RoleAssignmentRemovalResult::Missing) => {
                log_revoke_error(logger, RevokeRoleError::RoleAssignmentNotFound);
                return Err(RevokeRoleError::RoleAssignmentNotFound);
            }
            Err(error) => {
                let mapped = RevokeRoleError::from_repository_error(error);
                log_revoke_error(logger, mapped);
                return Err(mapped);
            }
        }

        logger.write_product(PermissionProductEvent::RoleRevoked {
            masked_actor_id: mask_user_id(&actor_user_id),
            target_id: format!("role_assignment:{}", mask_raw_id(assignment_id.as_str())),
        });
        Ok(RevokeRoleOutput {
            assignment_id: assignment_id.as_str().to_string(),
        })
    }
}

impl Default for RevokeRoleUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListWorkspaceRoleAssignmentsUsecase;

impl ListWorkspaceRoleAssignmentsUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ListWorkspaceRoleAssignmentsInput,
        policy_repository: &impl PermissionPolicyRepository,
    ) -> Result<ListWorkspaceRoleAssignmentsOutput, ListWorkspaceRoleAssignmentsError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ListWorkspaceRoleAssignmentsError::InvalidInput)?;
        let mut assignments = policy_repository
            .list_workspace_role_assignments(&workspace_id)
            .map_err(ListWorkspaceRoleAssignmentsError::from_repository_error)?
            .into_iter()
            .map(role_assignment_to_dto)
            .collect::<Vec<_>>();
        assignments.sort_by(|left, right| left.assignment_id.cmp(&right.assignment_id));
        Ok(ListWorkspaceRoleAssignmentsOutput { assignments })
    }
}

impl Default for ListWorkspaceRoleAssignmentsUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShareDocumentUsecase;

impl ShareDocumentUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ShareDocumentInput,
        policy_repository: &mut impl PermissionPolicyRepository,
        group_repository: &impl PermissionGroupRepository,
        logger: &mut impl PermissionUsecaseLogger,
    ) -> Result<ShareDocumentOutput, ShareDocumentError> {
        let actor_user_id = parse_user_id(&input.actor_user_id).map_err(|error| {
            log_share_error(logger, ShareDocumentError::from_check_error(error))
        })?;
        let workspace_id = parse_workspace_id(&input.workspace_id).map_err(|error| {
            log_share_error(logger, ShareDocumentError::from_check_error(error))
        })?;
        let collection_id =
            parse_optional_collection_id(input.collection_id.as_deref()).map_err(|error| {
                log_share_error(logger, ShareDocumentError::from_check_error(error))
            })?;
        let document_id = DocumentId::new(&input.document_id).map_err(|_| {
            log_share_error(logger, ShareDocumentError::InvalidInput);
            ShareDocumentError::InvalidInput
        })?;
        let resource = AccessResource::document(
            workspace_id.clone(),
            collection_id.clone(),
            document_id.clone(),
        );
        let manage_decision = evaluate_permission(
            policy_repository,
            group_repository,
            actor_user_id.clone(),
            &resource,
            Permission::Manage,
        )
        .map_err(|error| log_share_error(logger, ShareDocumentError::from_check_error(error)))?;

        write_field_debug(logger, &manage_decision);
        if manage_decision.decision.result() != PermissionDecisionResult::Allowed {
            log_permission_denied(
                logger,
                &actor_user_id,
                &resource,
                Permission::Manage,
                manage_decision.decision,
            );
            return Err(ShareDocumentError::Unauthorized);
        }

        let existing = policy_repository
            .get_document_policy(&workspace_id, &document_id)
            .map_err(|error| {
                let mapped = ShareDocumentError::from_repository_error(error);
                log_share_error(logger, mapped);
                mapped
            })?;
        let policy = existing
            .unwrap_or_else(|| DocumentPolicy::new(document_id.clone()))
            .with_override(input.override_input.to_policy_override());
        policy_repository
            .save_document_policy(&workspace_id, policy)
            .map_err(|error| {
                let mapped = ShareDocumentError::from_repository_error(error);
                log_share_error(logger, mapped);
                mapped
            })?;

        logger.write_product(PermissionProductEvent::PolicyChanged {
            masked_actor_id: mask_user_id(&actor_user_id),
            target_id: format!("document:{}", mask_raw_id(document_id.as_str())),
            permission: input.override_input.permission().as_str(),
            effect: input.override_input.effect_name(),
        });
        Ok(ShareDocumentOutput {
            document_id: document_id.as_str().to_string(),
        })
    }
}

impl Default for ShareDocumentUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetCollectionPermissionUsecase;

impl SetCollectionPermissionUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: SetCollectionPermissionInput,
        policy_repository: &mut impl PermissionPolicyRepository,
        group_repository: &impl PermissionGroupRepository,
        logger: &mut impl PermissionUsecaseLogger,
    ) -> Result<SetCollectionPermissionOutput, SetCollectionPermissionError> {
        let actor_user_id = parse_user_id(&input.actor_user_id).map_err(|error| {
            log_collection_error(
                logger,
                SetCollectionPermissionError::from_check_error(error),
            )
        })?;
        let workspace_id = parse_workspace_id(&input.workspace_id).map_err(|error| {
            log_collection_error(
                logger,
                SetCollectionPermissionError::from_check_error(error),
            )
        })?;
        let collection_id = CollectionId::new(&input.collection_id).map_err(|_| {
            log_collection_error(logger, SetCollectionPermissionError::InvalidInput);
            SetCollectionPermissionError::InvalidInput
        })?;
        let resource = AccessResource::collection(workspace_id.clone(), collection_id.clone());
        let manage_decision = evaluate_permission(
            policy_repository,
            group_repository,
            actor_user_id.clone(),
            &resource,
            Permission::Manage,
        )
        .map_err(|error| {
            log_collection_error(
                logger,
                SetCollectionPermissionError::from_check_error(error),
            )
        })?;

        write_field_debug(logger, &manage_decision);
        if manage_decision.decision.result() != PermissionDecisionResult::Allowed {
            log_permission_denied(
                logger,
                &actor_user_id,
                &resource,
                Permission::Manage,
                manage_decision.decision,
            );
            return Err(SetCollectionPermissionError::Unauthorized);
        }

        let existing = policy_repository
            .get_collection_policy(&workspace_id, &collection_id)
            .map_err(|error| {
                let mapped = SetCollectionPermissionError::from_repository_error(error);
                log_collection_error(logger, mapped);
                mapped
            })?;
        let policy = existing
            .unwrap_or_else(|| CollectionPolicy::new(collection_id.clone()))
            .with_override(input.override_input.to_policy_override());
        policy_repository
            .save_collection_policy(&workspace_id, policy)
            .map_err(|error| {
                let mapped = SetCollectionPermissionError::from_repository_error(error);
                log_collection_error(logger, mapped);
                mapped
            })?;

        logger.write_product(PermissionProductEvent::PolicyChanged {
            masked_actor_id: mask_user_id(&actor_user_id),
            target_id: format!("collection:{}", mask_raw_id(collection_id.as_str())),
            permission: input.override_input.permission().as_str(),
            effect: input.override_input.effect_name(),
        });
        Ok(SetCollectionPermissionOutput {
            collection_id: collection_id.as_str().to_string(),
        })
    }
}

impl Default for SetCollectionPermissionUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListEffectivePermissionsUsecase;

impl ListEffectivePermissionsUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ListEffectivePermissionsInput,
        policy_repository: &impl PermissionPolicyRepository,
        group_repository: &impl PermissionGroupRepository,
        logger: &mut impl PermissionUsecaseLogger,
    ) -> Result<ListEffectivePermissionsOutput, ListEffectivePermissionsError> {
        let actor_user_id = parse_user_id(&input.actor_user_id).map_err(|error| {
            log_effective_error(
                logger,
                ListEffectivePermissionsError::from_check_error(error),
            )
        })?;
        let resource = parse_resource(input.resource).map_err(|error| {
            log_effective_error(
                logger,
                ListEffectivePermissionsError::from_check_error(error),
            )
        })?;
        let mut allowed_permissions = Vec::new();

        for permission in ALL_PERMISSIONS {
            let evaluation = evaluate_permission(
                policy_repository,
                group_repository,
                actor_user_id.clone(),
                &resource,
                permission,
            )
            .map_err(|error| {
                log_effective_error(
                    logger,
                    ListEffectivePermissionsError::from_check_error(error),
                )
            })?;
            write_field_debug(logger, &evaluation);
            if evaluation.decision.result() == PermissionDecisionResult::Allowed {
                allowed_permissions.push(permission);
            }
        }

        Ok(ListEffectivePermissionsOutput {
            allowed_permissions,
        })
    }
}

impl Default for ListEffectivePermissionsUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckPermissionError {
    InvalidInput,
    StorageUnavailable,
}

impl CheckPermissionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_PERMISSION_INPUT",
            Self::StorageUnavailable => "PERMISSION_STORAGE_UNAVAILABLE",
        }
    }

    const fn from_repository_error(error: PermissionRepositoryError) -> Self {
        match error {
            PermissionRepositoryError::Conflict
            | PermissionRepositoryError::NotFound
            | PermissionRepositoryError::StorageUnavailable => Self::StorageUnavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignRoleError {
    InvalidInput,
    Unauthorized,
    StorageUnavailable,
}

impl AssignRoleError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_PERMISSION_INPUT",
            Self::Unauthorized => "PERMISSION_DENIED",
            Self::StorageUnavailable => "PERMISSION_STORAGE_UNAVAILABLE",
        }
    }

    const fn from_check_error(error: CheckPermissionError) -> Self {
        match error {
            CheckPermissionError::InvalidInput => Self::InvalidInput,
            CheckPermissionError::StorageUnavailable => Self::StorageUnavailable,
        }
    }

    const fn from_repository_error(error: PermissionRepositoryError) -> Self {
        match error {
            PermissionRepositoryError::Conflict
            | PermissionRepositoryError::NotFound
            | PermissionRepositoryError::StorageUnavailable => Self::StorageUnavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevokeRoleError {
    InvalidInput,
    Unauthorized,
    RoleAssignmentNotFound,
    StorageUnavailable,
}

impl RevokeRoleError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_PERMISSION_INPUT",
            Self::Unauthorized => "PERMISSION_DENIED",
            Self::RoleAssignmentNotFound => "ROLE_ASSIGNMENT_NOT_FOUND",
            Self::StorageUnavailable => "PERMISSION_STORAGE_UNAVAILABLE",
        }
    }

    const fn from_check_error(error: CheckPermissionError) -> Self {
        match error {
            CheckPermissionError::InvalidInput => Self::InvalidInput,
            CheckPermissionError::StorageUnavailable => Self::StorageUnavailable,
        }
    }

    const fn from_repository_error(error: PermissionRepositoryError) -> Self {
        match error {
            PermissionRepositoryError::NotFound => Self::RoleAssignmentNotFound,
            PermissionRepositoryError::Conflict | PermissionRepositoryError::StorageUnavailable => {
                Self::StorageUnavailable
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListWorkspaceRoleAssignmentsError {
    InvalidInput,
    StorageUnavailable,
}

impl ListWorkspaceRoleAssignmentsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_PERMISSION_INPUT",
            Self::StorageUnavailable => "PERMISSION_STORAGE_UNAVAILABLE",
        }
    }

    const fn from_repository_error(error: PermissionRepositoryError) -> Self {
        match error {
            PermissionRepositoryError::Conflict
            | PermissionRepositoryError::NotFound
            | PermissionRepositoryError::StorageUnavailable => Self::StorageUnavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareDocumentError {
    InvalidInput,
    Unauthorized,
    StorageUnavailable,
}

impl ShareDocumentError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_PERMISSION_INPUT",
            Self::Unauthorized => "PERMISSION_DENIED",
            Self::StorageUnavailable => "PERMISSION_STORAGE_UNAVAILABLE",
        }
    }

    const fn from_check_error(error: CheckPermissionError) -> Self {
        match error {
            CheckPermissionError::InvalidInput => Self::InvalidInput,
            CheckPermissionError::StorageUnavailable => Self::StorageUnavailable,
        }
    }

    const fn from_repository_error(error: PermissionRepositoryError) -> Self {
        match error {
            PermissionRepositoryError::Conflict
            | PermissionRepositoryError::NotFound
            | PermissionRepositoryError::StorageUnavailable => Self::StorageUnavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetCollectionPermissionError {
    InvalidInput,
    Unauthorized,
    StorageUnavailable,
}

impl SetCollectionPermissionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_PERMISSION_INPUT",
            Self::Unauthorized => "PERMISSION_DENIED",
            Self::StorageUnavailable => "PERMISSION_STORAGE_UNAVAILABLE",
        }
    }

    const fn from_check_error(error: CheckPermissionError) -> Self {
        match error {
            CheckPermissionError::InvalidInput => Self::InvalidInput,
            CheckPermissionError::StorageUnavailable => Self::StorageUnavailable,
        }
    }

    const fn from_repository_error(error: PermissionRepositoryError) -> Self {
        match error {
            PermissionRepositoryError::Conflict
            | PermissionRepositoryError::NotFound
            | PermissionRepositoryError::StorageUnavailable => Self::StorageUnavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListEffectivePermissionsError {
    InvalidInput,
    StorageUnavailable,
}

impl ListEffectivePermissionsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_PERMISSION_INPUT",
            Self::StorageUnavailable => "PERMISSION_STORAGE_UNAVAILABLE",
        }
    }

    const fn from_check_error(error: CheckPermissionError) -> Self {
        match error {
            CheckPermissionError::InvalidInput => Self::InvalidInput,
            CheckPermissionError::StorageUnavailable => Self::StorageUnavailable,
        }
    }
}

struct PermissionEvaluation {
    subject: AccessSubject,
    decision: PermissionDecision,
}

fn evaluate_permission(
    policy_repository: &impl PermissionPolicyRepository,
    group_repository: &impl PermissionGroupRepository,
    actor_user_id: UserId,
    resource: &AccessResource,
    permission: Permission,
) -> Result<PermissionEvaluation, CheckPermissionError> {
    let workspace_id = resource.workspace_id();
    let subject = load_access_subject(
        policy_repository,
        group_repository,
        workspace_id,
        actor_user_id,
    )?;
    let workspace_policy = WorkspacePolicy::default_role_matrix();
    let collection_policy = match resource.collection_id() {
        Some(collection_id) => policy_repository
            .get_collection_policy(workspace_id, collection_id)
            .map_err(CheckPermissionError::from_repository_error)?,
        None => None,
    };
    let document_policy = match resource.document_id() {
        Some(document_id) => policy_repository
            .get_document_policy(workspace_id, document_id)
            .map_err(CheckPermissionError::from_repository_error)?,
        None => None,
    };

    let decision = match resource {
        AccessResource::Workspace { .. } => workspace_policy.decide(&subject, resource, permission),
        AccessResource::Collection { .. } => match collection_policy.as_ref() {
            Some(collection_policy) => collection_policy.decide_with_parent(
                &workspace_policy,
                &subject,
                resource,
                permission,
            ),
            None => workspace_policy.decide(&subject, resource, permission),
        },
        AccessResource::Document { .. } | AccessResource::Asset { .. } => {
            match document_policy.as_ref() {
                Some(document_policy) => document_policy.decide_with_parents(
                    &workspace_policy,
                    collection_policy.as_ref(),
                    &subject,
                    resource,
                    permission,
                ),
                None => match collection_policy.as_ref() {
                    Some(collection_policy) => collection_policy.decide_with_parent(
                        &workspace_policy,
                        &subject,
                        resource,
                        permission,
                    ),
                    None => workspace_policy.decide(&subject, resource, permission),
                },
            }
        }
    };

    Ok(PermissionEvaluation { subject, decision })
}

fn load_access_subject(
    policy_repository: &impl PermissionPolicyRepository,
    group_repository: &impl PermissionGroupRepository,
    workspace_id: &WorkspaceId,
    actor_user_id: UserId,
) -> Result<AccessSubject, CheckPermissionError> {
    let group_ids = group_repository
        .list_user_group_ids(workspace_id, &actor_user_id)
        .map_err(CheckPermissionError::from_repository_error)?;
    let user_assignments = policy_repository
        .list_user_role_assignments(workspace_id, &actor_user_id)
        .map_err(CheckPermissionError::from_repository_error)?;
    let group_assignments = policy_repository
        .list_group_role_assignments(workspace_id, &group_ids)
        .map_err(CheckPermissionError::from_repository_error)?;
    let roles = user_assignments
        .iter()
        .chain(group_assignments.iter())
        .map(RoleAssignment::role)
        .collect();
    Ok(AccessSubject::new(actor_user_id, roles, group_ids))
}

fn parse_resource(input: PermissionResourceInput) -> Result<AccessResource, CheckPermissionError> {
    match input {
        PermissionResourceInput::Workspace { workspace_id } => Ok(AccessResource::workspace(
            parse_workspace_id(&workspace_id)?,
        )),
        PermissionResourceInput::Collection {
            workspace_id,
            collection_id,
        } => Ok(AccessResource::collection(
            parse_workspace_id(&workspace_id)?,
            CollectionId::new(&collection_id).map_err(|_| CheckPermissionError::InvalidInput)?,
        )),
        PermissionResourceInput::Document {
            workspace_id,
            collection_id,
            document_id,
        } => Ok(AccessResource::document(
            parse_workspace_id(&workspace_id)?,
            parse_optional_collection_id(collection_id.as_deref())?,
            DocumentId::new(&document_id).map_err(|_| CheckPermissionError::InvalidInput)?,
        )),
        PermissionResourceInput::Asset {
            workspace_id,
            collection_id,
            document_id,
            asset_id,
        } => Ok(AccessResource::asset(
            parse_workspace_id(&workspace_id)?,
            parse_optional_collection_id(collection_id.as_deref())?,
            parse_optional_document_id(document_id.as_deref())?,
            AssetId::from_sha256_hex(&asset_id).map_err(|_| CheckPermissionError::InvalidInput)?,
        )),
    }
}

fn parse_role_assignment_subject(
    input: RoleAssignmentSubjectInput,
) -> Result<RoleAssignmentSubject, AssignRoleError> {
    match input {
        RoleAssignmentSubjectInput::User(user_id) => UserId::new(&user_id)
            .map(RoleAssignmentSubject::User)
            .map_err(|_| AssignRoleError::InvalidInput),
        RoleAssignmentSubjectInput::Group(group_id) => GroupId::new(&group_id)
            .map(RoleAssignmentSubject::Group)
            .map_err(|_| AssignRoleError::InvalidInput),
    }
}

fn parse_role_name(value: &str) -> Result<Role, AssignRoleError> {
    match value {
        "owner" => Ok(Role::Owner),
        "admin" => Ok(Role::Admin),
        "editor" => Ok(Role::Editor),
        "reviewer" => Ok(Role::Reviewer),
        "viewer" => Ok(Role::Viewer),
        _ => Err(AssignRoleError::InvalidInput),
    }
}

fn parse_permission_name(value: &str) -> Result<Permission, ShareDocumentError> {
    match value {
        "read" => Ok(Permission::Read),
        "write" => Ok(Permission::Write),
        "review" => Ok(Permission::Review),
        "publish" => Ok(Permission::Publish),
        "manage" => Ok(Permission::Manage),
        "asset_metadata_read" | "read_asset_metadata" => Ok(Permission::ReadAssetMetadata),
        "asset_content_read" | "read_asset_content" => Ok(Permission::ReadAssetContent),
        _ => Err(ShareDocumentError::InvalidInput),
    }
}

fn role_assignment_to_dto(assignment: RoleAssignment) -> RoleAssignmentDto {
    RoleAssignmentDto {
        assignment_id: assignment.id().as_str().to_string(),
        subject_type: role_assignment_subject_type(assignment.subject()),
        subject_id: role_assignment_subject_id(assignment.subject()).to_string(),
        role: assignment.role().as_str(),
    }
}

const fn role_assignment_subject_type(subject: &RoleAssignmentSubject) -> &'static str {
    match subject {
        RoleAssignmentSubject::User(_) => "user",
        RoleAssignmentSubject::Group(_) => "group",
    }
}

fn role_assignment_subject_id(subject: &RoleAssignmentSubject) -> &str {
    match subject {
        RoleAssignmentSubject::User(user_id) => user_id.as_str(),
        RoleAssignmentSubject::Group(group_id) => group_id.as_str(),
    }
}

fn parse_workspace_id(value: &str) -> Result<WorkspaceId, CheckPermissionError> {
    WorkspaceId::new(value).map_err(|_| CheckPermissionError::InvalidInput)
}

fn parse_user_id(value: &str) -> Result<UserId, CheckPermissionError> {
    UserId::new(value).map_err(|_| CheckPermissionError::InvalidInput)
}

fn parse_optional_collection_id(
    value: Option<&str>,
) -> Result<Option<CollectionId>, CheckPermissionError> {
    value
        .map(CollectionId::new)
        .transpose()
        .map_err(|_| CheckPermissionError::InvalidInput)
}

fn parse_optional_document_id(
    value: Option<&str>,
) -> Result<Option<DocumentId>, CheckPermissionError> {
    value
        .map(DocumentId::new)
        .transpose()
        .map_err(|_| CheckPermissionError::InvalidInput)
}

fn role_assignment_subject_target_id(subject: &RoleAssignmentSubject) -> String {
    match subject {
        RoleAssignmentSubject::User(user_id) => format!("user:{}", mask_raw_id(user_id.as_str())),
        RoleAssignmentSubject::Group(group_id) => {
            format!("group:{}", mask_raw_id(group_id.as_str()))
        }
    }
}

fn resource_target_id(resource: &AccessResource) -> String {
    match resource {
        AccessResource::Workspace { workspace_id } => {
            format!("workspace:{}", mask_raw_id(workspace_id.as_str()))
        }
        AccessResource::Collection { collection_id, .. } => {
            format!("collection:{}", mask_raw_id(collection_id.as_str()))
        }
        AccessResource::Document { document_id, .. } => {
            format!("document:{}", mask_raw_id(document_id.as_str()))
        }
        AccessResource::Asset { asset_id, .. } => {
            format!("asset:{}", mask_raw_id(asset_id.as_str()))
        }
    }
}

fn write_field_debug(logger: &mut impl PermissionUsecaseLogger, evaluation: &PermissionEvaluation) {
    logger.write_field_debug(PermissionFieldDebugEvent {
        role_count: evaluation.subject.roles().len(),
        policy_source: evaluation.decision.source().as_str(),
        decision: evaluation.decision.result().as_str(),
        reason_code: evaluation.decision.reason_code(),
    });
}

fn log_permission_denied(
    logger: &mut impl PermissionUsecaseLogger,
    actor_user_id: &UserId,
    resource: &AccessResource,
    permission: Permission,
    decision: PermissionDecision,
) {
    logger.write_product(PermissionProductEvent::PermissionDenied {
        masked_actor_id: mask_user_id(actor_user_id),
        target_id: resource_target_id(resource),
        permission: permission.as_str(),
        decision: decision.result().as_str(),
        error_code: decision.reason_code(),
    });
}

fn log_check_error(
    logger: &mut impl PermissionUsecaseLogger,
    error: CheckPermissionError,
) -> CheckPermissionError {
    logger.write_product(PermissionProductEvent::UsecaseFailed {
        error_code: error.code(),
    });
    error
}

fn log_assign_error(
    logger: &mut impl PermissionUsecaseLogger,
    error: AssignRoleError,
) -> AssignRoleError {
    logger.write_product(PermissionProductEvent::UsecaseFailed {
        error_code: error.code(),
    });
    error
}

fn log_revoke_error(
    logger: &mut impl PermissionUsecaseLogger,
    error: RevokeRoleError,
) -> RevokeRoleError {
    logger.write_product(PermissionProductEvent::UsecaseFailed {
        error_code: error.code(),
    });
    error
}

fn log_share_error(
    logger: &mut impl PermissionUsecaseLogger,
    error: ShareDocumentError,
) -> ShareDocumentError {
    logger.write_product(PermissionProductEvent::UsecaseFailed {
        error_code: error.code(),
    });
    error
}

fn log_collection_error(
    logger: &mut impl PermissionUsecaseLogger,
    error: SetCollectionPermissionError,
) -> SetCollectionPermissionError {
    logger.write_product(PermissionProductEvent::UsecaseFailed {
        error_code: error.code(),
    });
    error
}

fn log_effective_error(
    logger: &mut impl PermissionUsecaseLogger,
    error: ListEffectivePermissionsError,
) -> ListEffectivePermissionsError {
    logger.write_product(PermissionProductEvent::UsecaseFailed {
        error_code: error.code(),
    });
    error
}

fn mask_user_id(user_id: &UserId) -> String {
    mask_raw_id(user_id.as_str())
}

fn mask_raw_id(value: &str) -> String {
    let suffix_start = value.len().saturating_sub(4);
    format!("masked:{}", &value[suffix_start..])
}
