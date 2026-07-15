use crate::asset::AssetId;
use crate::document::DocumentId;
use crate::group::GroupId;
use crate::user::UserId;
use crate::workspace::WorkspaceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Owner,
    Admin,
    Editor,
    Reviewer,
    Viewer,
}

impl Role {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Editor => "editor",
            Self::Reviewer => "reviewer",
            Self::Viewer => "viewer",
        }
    }

    const fn allows(self, permission: Permission) -> bool {
        match self {
            Self::Owner | Self::Admin => true,
            Self::Editor => matches!(
                permission,
                Permission::Read
                    | Permission::Write
                    | Permission::ReadAssetMetadata
                    | Permission::ReadAssetContent
            ),
            Self::Reviewer => matches!(
                permission,
                Permission::Read | Permission::Review | Permission::ReadAssetMetadata
            ),
            Self::Viewer => matches!(permission, Permission::Read | Permission::ReadAssetMetadata),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Read,
    Write,
    Review,
    Publish,
    Manage,
    ReadAssetMetadata,
    ReadAssetContent,
}

impl Permission {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Review => "review",
            Self::Publish => "publish",
            Self::Manage => "manage",
            Self::ReadAssetMetadata => "asset_metadata_read",
            Self::ReadAssetContent => "asset_content_read",
        }
    }

    pub const fn scope(self) -> PermissionScope {
        match self {
            Self::Read | Self::Write | Self::Review | Self::Publish | Self::Manage => {
                PermissionScope::Document
            }
            Self::ReadAssetMetadata => PermissionScope::AssetMetadata,
            Self::ReadAssetContent => PermissionScope::AssetContent,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionScope {
    Workspace,
    Collection,
    Document,
    AssetMetadata,
    AssetContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessSubject {
    user_id: UserId,
    roles: Vec<Role>,
    group_ids: Vec<GroupId>,
}

impl AccessSubject {
    pub fn new(user_id: UserId, roles: Vec<Role>, group_ids: Vec<GroupId>) -> Self {
        Self {
            user_id,
            roles,
            group_ids,
        }
    }

    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    pub fn roles(&self) -> &[Role] {
        &self.roles
    }

    pub fn group_ids(&self) -> &[GroupId] {
        &self.group_ids
    }

    fn has_permission(&self, permission: Permission) -> bool {
        self.roles.iter().any(|role| role.allows(permission))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessResource {
    Workspace {
        workspace_id: WorkspaceId,
    },
    Collection {
        workspace_id: WorkspaceId,
        collection_id: CollectionId,
    },
    Document {
        workspace_id: WorkspaceId,
        collection_id: Option<CollectionId>,
        document_id: DocumentId,
    },
    Asset {
        workspace_id: WorkspaceId,
        collection_id: Option<CollectionId>,
        document_id: Option<DocumentId>,
        asset_id: AssetId,
    },
}

impl AccessResource {
    pub fn workspace(workspace_id: WorkspaceId) -> Self {
        Self::Workspace { workspace_id }
    }

    pub fn collection(workspace_id: WorkspaceId, collection_id: CollectionId) -> Self {
        Self::Collection {
            workspace_id,
            collection_id,
        }
    }

    pub fn document(
        workspace_id: WorkspaceId,
        collection_id: Option<CollectionId>,
        document_id: DocumentId,
    ) -> Self {
        Self::Document {
            workspace_id,
            collection_id,
            document_id,
        }
    }

    pub fn asset(
        workspace_id: WorkspaceId,
        collection_id: Option<CollectionId>,
        document_id: Option<DocumentId>,
        asset_id: AssetId,
    ) -> Self {
        Self::Asset {
            workspace_id,
            collection_id,
            document_id,
            asset_id,
        }
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        match self {
            Self::Workspace { workspace_id }
            | Self::Collection { workspace_id, .. }
            | Self::Document { workspace_id, .. }
            | Self::Asset { workspace_id, .. } => workspace_id,
        }
    }

    pub fn collection_id(&self) -> Option<&CollectionId> {
        match self {
            Self::Collection { collection_id, .. } => Some(collection_id),
            Self::Document { collection_id, .. } | Self::Asset { collection_id, .. } => {
                collection_id.as_ref()
            }
            Self::Workspace { .. } => None,
        }
    }

    pub fn document_id(&self) -> Option<&DocumentId> {
        match self {
            Self::Document { document_id, .. } => Some(document_id),
            Self::Asset { document_id, .. } => document_id.as_ref(),
            Self::Workspace { .. } | Self::Collection { .. } => None,
        }
    }

    pub fn asset_id(&self) -> Option<&AssetId> {
        match self {
            Self::Asset { asset_id, .. } => Some(asset_id),
            Self::Workspace { .. } | Self::Collection { .. } | Self::Document { .. } => None,
        }
    }

    fn supports(&self, permission: Permission) -> bool {
        match permission.scope() {
            PermissionScope::Workspace => matches!(self, Self::Workspace { .. }),
            PermissionScope::Collection => {
                matches!(self, Self::Workspace { .. } | Self::Collection { .. })
            }
            PermissionScope::Document => !matches!(self, Self::Asset { .. }),
            PermissionScope::AssetMetadata | PermissionScope::AssetContent => {
                matches!(self, Self::Asset { .. })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionId {
    value: String,
}

impl CollectionId {
    pub fn new(value: &str) -> Result<Self, PermissionError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(PermissionError::EmptyCollectionId);
        }
        if trimmed.chars().any(char::is_control) {
            return Err(PermissionError::InvalidCollectionId);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleAssignmentId {
    value: String,
}

impl RoleAssignmentId {
    pub fn new(value: &str) -> Result<Self, PermissionError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(PermissionError::EmptyRoleAssignmentId);
        }
        if trimmed.chars().any(char::is_control) {
            return Err(PermissionError::InvalidRoleAssignmentId);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleAssignmentSubject {
    User(UserId),
    Group(GroupId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleAssignment {
    id: RoleAssignmentId,
    workspace_id: WorkspaceId,
    subject: RoleAssignmentSubject,
    role: Role,
}

impl RoleAssignment {
    pub fn new(
        id: RoleAssignmentId,
        workspace_id: WorkspaceId,
        subject: RoleAssignmentSubject,
        role: Role,
    ) -> Self {
        Self {
            id,
            workspace_id,
            subject,
            role,
        }
    }

    pub fn id(&self) -> &RoleAssignmentId {
        &self.id
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn subject(&self) -> &RoleAssignmentSubject {
        &self.subject
    }

    pub const fn role(&self) -> Role {
        self.role
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionDecision {
    result: PermissionDecisionResult,
    source: PolicySource,
    reason: PermissionDecisionReason,
}

impl PermissionDecision {
    pub const fn allowed(source: PolicySource, reason: PermissionDecisionReason) -> Self {
        Self {
            result: PermissionDecisionResult::Allowed,
            source,
            reason,
        }
    }

    pub const fn denied(source: PolicySource, reason: PermissionDecisionReason) -> Self {
        Self {
            result: PermissionDecisionResult::Denied,
            source,
            reason,
        }
    }

    pub const fn not_found(source: PolicySource, reason: PermissionDecisionReason) -> Self {
        Self {
            result: PermissionDecisionResult::NotFound,
            source,
            reason,
        }
    }

    pub const fn indeterminate(source: PolicySource, reason: PermissionDecisionReason) -> Self {
        Self {
            result: PermissionDecisionResult::Indeterminate,
            source,
            reason,
        }
    }

    pub const fn result(self) -> PermissionDecisionResult {
        self.result
    }

    pub const fn source(self) -> PolicySource {
        self.source
    }

    pub const fn reason(self) -> PermissionDecisionReason {
        self.reason
    }

    pub const fn reason_code(self) -> &'static str {
        self.reason.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecisionResult {
    Allowed,
    Denied,
    NotFound,
    Indeterminate,
}

impl PermissionDecisionResult {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "Allowed",
            Self::Denied => "Denied",
            Self::NotFound => "NotFound",
            Self::Indeterminate => "Indeterminate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecisionReason {
    RoleAllowsPermission,
    RoleDoesNotAllowPermission,
    RoleNotAssigned,
    PolicyOverrideAllowed,
    PolicyOverrideDenied,
    HiddenByPolicy,
    PolicyResourceMismatch,
    PermissionScopeMismatch,
}

impl PermissionDecisionReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RoleAllowsPermission => "ROLE_ALLOWS_PERMISSION",
            Self::RoleDoesNotAllowPermission => "ROLE_DOES_NOT_ALLOW_PERMISSION",
            Self::RoleNotAssigned => "ROLE_NOT_ASSIGNED",
            Self::PolicyOverrideAllowed => "POLICY_OVERRIDE_ALLOWED",
            Self::PolicyOverrideDenied => "POLICY_OVERRIDE_DENIED",
            Self::HiddenByPolicy => "HIDDEN_BY_POLICY",
            Self::PolicyResourceMismatch => "POLICY_RESOURCE_MISMATCH",
            Self::PermissionScopeMismatch => "PERMISSION_SCOPE_MISMATCH",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicySource {
    Workspace,
    Collection,
    Document,
    Asset,
}

impl PolicySource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Collection => "collection",
            Self::Document => "document",
            Self::Asset => "asset",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePolicy;

impl WorkspacePolicy {
    pub const fn default_role_matrix() -> Self {
        Self
    }

    pub fn decide(
        &self,
        subject: &AccessSubject,
        resource: &AccessResource,
        permission: Permission,
    ) -> PermissionDecision {
        if !resource.supports(permission) {
            return PermissionDecision::indeterminate(
                PolicySource::Workspace,
                PermissionDecisionReason::PermissionScopeMismatch,
            );
        }
        if subject.roles().is_empty() {
            return PermissionDecision::denied(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleNotAssigned,
            );
        }
        if subject.has_permission(permission) {
            return PermissionDecision::allowed(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleAllowsPermission,
            );
        }
        PermissionDecision::denied(
            PolicySource::Workspace,
            PermissionDecisionReason::RoleDoesNotAllowPermission,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionPolicy {
    collection_id: CollectionId,
    overrides: Vec<PolicyOverride>,
}

impl CollectionPolicy {
    pub fn new(collection_id: CollectionId) -> Self {
        Self {
            collection_id,
            overrides: Vec::new(),
        }
    }

    pub fn with_override(mut self, policy_override: PolicyOverride) -> Self {
        self.overrides.push(policy_override);
        self
    }

    pub fn collection_id(&self) -> &CollectionId {
        &self.collection_id
    }

    pub fn overrides(&self) -> &[PolicyOverride] {
        &self.overrides
    }

    pub fn decide_with_parent(
        &self,
        workspace_policy: &WorkspacePolicy,
        subject: &AccessSubject,
        resource: &AccessResource,
        permission: Permission,
    ) -> PermissionDecision {
        if !matches_policy_collection(resource, &self.collection_id) {
            return PermissionDecision::indeterminate(
                PolicySource::Collection,
                PermissionDecisionReason::PolicyResourceMismatch,
            );
        }
        decide_override_or_else(
            self.overrides(),
            PolicySource::Collection,
            permission,
            || workspace_policy.decide(subject, resource, permission),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentPolicy {
    document_id: DocumentId,
    overrides: Vec<PolicyOverride>,
}

impl DocumentPolicy {
    pub fn new(document_id: DocumentId) -> Self {
        Self {
            document_id,
            overrides: Vec::new(),
        }
    }

    pub fn with_override(mut self, policy_override: PolicyOverride) -> Self {
        self.overrides.push(policy_override);
        self
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn overrides(&self) -> &[PolicyOverride] {
        &self.overrides
    }

    pub fn decide_with_parents(
        &self,
        workspace_policy: &WorkspacePolicy,
        collection_policy: Option<&CollectionPolicy>,
        subject: &AccessSubject,
        resource: &AccessResource,
        permission: Permission,
    ) -> PermissionDecision {
        if !matches_policy_document(resource, &self.document_id) {
            return PermissionDecision::indeterminate(
                PolicySource::Document,
                PermissionDecisionReason::PolicyResourceMismatch,
            );
        }
        decide_override_or_else(self.overrides(), PolicySource::Document, permission, || {
            match collection_policy {
                Some(collection_policy) => collection_policy.decide_with_parent(
                    workspace_policy,
                    subject,
                    resource,
                    permission,
                ),
                None => workspace_policy.decide(subject, resource, permission),
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetPolicy {
    asset_id: AssetId,
    overrides: Vec<PolicyOverride>,
}

impl AssetPolicy {
    pub fn new(asset_id: AssetId) -> Self {
        Self {
            asset_id,
            overrides: Vec::new(),
        }
    }

    pub fn with_override(mut self, policy_override: PolicyOverride) -> Self {
        self.overrides.push(policy_override);
        self
    }

    pub fn asset_id(&self) -> &AssetId {
        &self.asset_id
    }

    pub fn overrides(&self) -> &[PolicyOverride] {
        &self.overrides
    }

    pub fn decide_with_parents(
        &self,
        workspace_policy: &WorkspacePolicy,
        collection_policy: Option<&CollectionPolicy>,
        document_policy: Option<&DocumentPolicy>,
        subject: &AccessSubject,
        resource: &AccessResource,
        permission: Permission,
    ) -> PermissionDecision {
        if !matches_policy_asset(resource, &self.asset_id) {
            return PermissionDecision::indeterminate(
                PolicySource::Asset,
                PermissionDecisionReason::PolicyResourceMismatch,
            );
        }
        decide_override_or_else(self.overrides(), PolicySource::Asset, permission, || {
            match document_policy {
                Some(document_policy) => document_policy.decide_with_parents(
                    workspace_policy,
                    collection_policy,
                    subject,
                    resource,
                    permission,
                ),
                None => match collection_policy {
                    Some(collection_policy) => collection_policy.decide_with_parent(
                        workspace_policy,
                        subject,
                        resource,
                        permission,
                    ),
                    None => workspace_policy.decide(subject, resource, permission),
                },
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyOverride {
    permission: Permission,
    effect: PolicyOverrideEffect,
}

impl PolicyOverride {
    pub const fn allow(permission: Permission) -> Self {
        Self {
            permission,
            effect: PolicyOverrideEffect::Allow,
        }
    }

    pub const fn deny(permission: Permission) -> Self {
        Self {
            permission,
            effect: PolicyOverrideEffect::Deny,
        }
    }

    pub const fn hide(permission: Permission) -> Self {
        Self {
            permission,
            effect: PolicyOverrideEffect::Hide,
        }
    }

    pub const fn permission(self) -> Permission {
        self.permission
    }

    const fn effect(self) -> PolicyOverrideEffect {
        self.effect
    }

    fn decide(self, source: PolicySource) -> PermissionDecision {
        match self.effect() {
            PolicyOverrideEffect::Allow => {
                PermissionDecision::allowed(source, PermissionDecisionReason::PolicyOverrideAllowed)
            }
            PolicyOverrideEffect::Deny => {
                PermissionDecision::denied(source, PermissionDecisionReason::PolicyOverrideDenied)
            }
            PolicyOverrideEffect::Hide => {
                PermissionDecision::not_found(source, PermissionDecisionReason::HiddenByPolicy)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PolicyOverrideEffect {
    Allow,
    Deny,
    Hide,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionError {
    EmptyCollectionId,
    InvalidCollectionId,
    EmptyRoleAssignmentId,
    InvalidRoleAssignmentId,
}

fn decide_override_or_else<F>(
    overrides: &[PolicyOverride],
    source: PolicySource,
    permission: Permission,
    fallback: F,
) -> PermissionDecision
where
    F: FnOnce() -> PermissionDecision,
{
    overrides
        .iter()
        .rev()
        .copied()
        .find(|policy_override| policy_override.permission() == permission)
        .map_or_else(fallback, |policy_override| policy_override.decide(source))
}

fn matches_policy_collection(resource: &AccessResource, collection_id: &CollectionId) -> bool {
    resource.collection_id() == Some(collection_id)
}

fn matches_policy_document(resource: &AccessResource, document_id: &DocumentId) -> bool {
    resource.document_id() == Some(document_id)
}

fn matches_policy_asset(resource: &AccessResource, asset_id: &AssetId) -> bool {
    resource.asset_id() == Some(asset_id)
}
