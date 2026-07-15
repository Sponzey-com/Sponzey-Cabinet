use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use cabinet_domain::document::DocumentId;
use cabinet_domain::group::GroupId;
use cabinet_domain::permission::{
    AccessResource, AccessSubject, CollectionId, CollectionPolicy, DocumentPolicy, Permission,
    PermissionDecisionResult, PolicyOverride, Role, RoleAssignment, RoleAssignmentId,
    RoleAssignmentSubject, WorkspacePolicy,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::permission_policy_repository::{
    PermissionPolicyRepository, PermissionRepositoryError, RoleAssignmentMutationResult,
    RoleAssignmentRemovalResult,
};

use crate::local_atomic_file::write_text_atomically;

pub const LOCAL_PERMISSIONS_DIR: &str = "permissions";
pub const LOCAL_ROLE_ASSIGNMENTS_DIR: &str = "role-assignments";
pub const LOCAL_ROLE_ASSIGNMENTS_BY_ID_DIR: &str = "by-id";
pub const LOCAL_ROLE_ASSIGNMENTS_BY_USER_DIR: &str = "by-user";
pub const LOCAL_ROLE_ASSIGNMENTS_BY_GROUP_DIR: &str = "by-group";
pub const LOCAL_COLLECTION_POLICIES_DIR: &str = "collection-policies";
pub const LOCAL_DOCUMENT_POLICIES_DIR: &str = "document-policies";

#[derive(Clone, PartialEq, Eq)]
pub struct LocalPermissionPolicyRepository {
    root: PathBuf,
}

impl fmt::Debug for LocalPermissionPolicyRepository {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalPermissionPolicyRepository")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl LocalPermissionPolicyRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn workspace_root(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.root
            .join(LOCAL_PERMISSIONS_DIR)
            .join(hex_encode(workspace_id.as_str()))
    }

    fn assignment_path(
        &self,
        workspace_id: &WorkspaceId,
        assignment_id: &RoleAssignmentId,
    ) -> PathBuf {
        self.workspace_root(workspace_id)
            .join(LOCAL_ROLE_ASSIGNMENTS_DIR)
            .join(LOCAL_ROLE_ASSIGNMENTS_BY_ID_DIR)
            .join(format!("{}.assignment", hex_encode(assignment_id.as_str())))
    }

    fn user_assignment_index_dir(&self, workspace_id: &WorkspaceId, user_id: &UserId) -> PathBuf {
        self.workspace_root(workspace_id)
            .join(LOCAL_ROLE_ASSIGNMENTS_DIR)
            .join(LOCAL_ROLE_ASSIGNMENTS_BY_USER_DIR)
            .join(hex_encode(user_id.as_str()))
    }

    fn group_assignment_index_dir(
        &self,
        workspace_id: &WorkspaceId,
        group_id: &GroupId,
    ) -> PathBuf {
        self.workspace_root(workspace_id)
            .join(LOCAL_ROLE_ASSIGNMENTS_DIR)
            .join(LOCAL_ROLE_ASSIGNMENTS_BY_GROUP_DIR)
            .join(hex_encode(group_id.as_str()))
    }

    fn assignment_index_path(
        &self,
        workspace_id: &WorkspaceId,
        assignment: &RoleAssignment,
    ) -> PathBuf {
        let file_name = format!("{}.idx", hex_encode(assignment.id().as_str()));
        match assignment.subject() {
            RoleAssignmentSubject::User(user_id) => self
                .user_assignment_index_dir(workspace_id, user_id)
                .join(file_name),
            RoleAssignmentSubject::Group(group_id) => self
                .group_assignment_index_dir(workspace_id, group_id)
                .join(file_name),
        }
    }

    fn collection_policy_path(
        &self,
        workspace_id: &WorkspaceId,
        collection_id: &CollectionId,
    ) -> PathBuf {
        self.workspace_root(workspace_id)
            .join(LOCAL_COLLECTION_POLICIES_DIR)
            .join(format!("{}.policy", hex_encode(collection_id.as_str())))
    }

    fn document_policy_path(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> PathBuf {
        self.workspace_root(workspace_id)
            .join(LOCAL_DOCUMENT_POLICIES_DIR)
            .join(format!("{}.policy", hex_encode(document_id.as_str())))
    }

    fn load_assignment(
        &self,
        workspace_id: &WorkspaceId,
        assignment_id: &RoleAssignmentId,
    ) -> Result<Option<RoleAssignment>, PermissionRepositoryError> {
        let path = self.assignment_path(workspace_id, assignment_id);
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(PermissionRepositoryError::StorageUnavailable),
        };
        let assignment = decode_assignment(&content)?;
        if assignment.workspace_id() != workspace_id {
            return Err(PermissionRepositoryError::StorageUnavailable);
        }
        Ok(Some(assignment))
    }

    fn list_assignments_from_index_dir(
        &self,
        workspace_id: &WorkspaceId,
        dir: PathBuf,
    ) -> Result<Vec<RoleAssignment>, PermissionRepositoryError> {
        let ids = read_assignment_ids_from_index_dir(&dir)?;
        let mut assignments = Vec::new();
        for assignment_id in ids {
            let Some(assignment) = self.load_assignment(workspace_id, &assignment_id)? else {
                return Err(PermissionRepositoryError::StorageUnavailable);
            };
            assignments.push(assignment);
        }
        assignments.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        Ok(assignments)
    }
}

impl PermissionPolicyRepository for LocalPermissionPolicyRepository {
    fn list_user_role_assignments(
        &self,
        workspace_id: &WorkspaceId,
        user_id: &UserId,
    ) -> Result<Vec<RoleAssignment>, PermissionRepositoryError> {
        self.list_assignments_from_index_dir(
            workspace_id,
            self.user_assignment_index_dir(workspace_id, user_id),
        )
    }

    fn list_group_role_assignments(
        &self,
        workspace_id: &WorkspaceId,
        group_ids: &[GroupId],
    ) -> Result<Vec<RoleAssignment>, PermissionRepositoryError> {
        let mut ids = BTreeSet::new();
        for group_id in group_ids {
            for assignment_id in read_assignment_ids_from_index_dir(
                &self.group_assignment_index_dir(workspace_id, group_id),
            )? {
                ids.insert(assignment_id.as_str().to_string());
            }
        }
        let mut assignments = Vec::new();
        for assignment_id in ids {
            let assignment_id = RoleAssignmentId::new(&assignment_id)
                .map_err(|_| PermissionRepositoryError::StorageUnavailable)?;
            let Some(assignment) = self.load_assignment(workspace_id, &assignment_id)? else {
                return Err(PermissionRepositoryError::StorageUnavailable);
            };
            assignments.push(assignment);
        }
        assignments.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        Ok(assignments)
    }

    fn list_workspace_role_assignments(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<RoleAssignment>, PermissionRepositoryError> {
        let root = self
            .workspace_root(workspace_id)
            .join(LOCAL_ROLE_ASSIGNMENTS_DIR)
            .join(LOCAL_ROLE_ASSIGNMENTS_BY_ID_DIR);
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(PermissionRepositoryError::StorageUnavailable),
        };
        let mut assignments = Vec::new();
        for entry in entries {
            let path = entry
                .map_err(|_| PermissionRepositoryError::StorageUnavailable)?
                .path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("assignment") {
                continue;
            }
            let assignment = decode_assignment(
                &fs::read_to_string(path)
                    .map_err(|_| PermissionRepositoryError::StorageUnavailable)?,
            )?;
            if assignment.workspace_id() != workspace_id {
                return Err(PermissionRepositoryError::StorageUnavailable);
            }
            assignments.push(assignment);
        }
        assignments.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        Ok(assignments)
    }

    fn get_role_assignment(
        &self,
        workspace_id: &WorkspaceId,
        assignment_id: &RoleAssignmentId,
    ) -> Result<Option<RoleAssignment>, PermissionRepositoryError> {
        self.load_assignment(workspace_id, assignment_id)
    }

    fn save_role_assignment(
        &mut self,
        workspace_id: &WorkspaceId,
        assignment: RoleAssignment,
    ) -> Result<RoleAssignmentMutationResult, PermissionRepositoryError> {
        if assignment.workspace_id() != workspace_id {
            return Err(PermissionRepositoryError::StorageUnavailable);
        }

        let existing = self.load_assignment(workspace_id, assignment.id())?;
        let result = if existing.is_some() {
            RoleAssignmentMutationResult::AlreadyApplied
        } else {
            RoleAssignmentMutationResult::Changed
        };
        if let Some(existing_assignment) = existing {
            remove_file_if_exists(self.assignment_index_path(workspace_id, &existing_assignment))?;
        }

        write_text_atomically(
            &self.assignment_path(workspace_id, assignment.id()),
            encode_assignment(&assignment),
        )
        .map_err(|_| PermissionRepositoryError::StorageUnavailable)?;
        write_text_atomically(
            &self.assignment_index_path(workspace_id, &assignment),
            format!("{}\n", hex_encode(assignment.id().as_str())),
        )
        .map_err(|_| PermissionRepositoryError::StorageUnavailable)?;
        Ok(result)
    }

    fn remove_role_assignment(
        &mut self,
        workspace_id: &WorkspaceId,
        assignment_id: &RoleAssignmentId,
    ) -> Result<RoleAssignmentRemovalResult, PermissionRepositoryError> {
        let Some(existing_assignment) = self.load_assignment(workspace_id, assignment_id)? else {
            return Ok(RoleAssignmentRemovalResult::Missing);
        };
        remove_file_if_exists(self.assignment_index_path(workspace_id, &existing_assignment))?;
        fs::remove_file(self.assignment_path(workspace_id, assignment_id))
            .map(|_| RoleAssignmentRemovalResult::Removed)
            .map_err(|_| PermissionRepositoryError::StorageUnavailable)
    }

    fn get_collection_policy(
        &self,
        workspace_id: &WorkspaceId,
        collection_id: &CollectionId,
    ) -> Result<Option<CollectionPolicy>, PermissionRepositoryError> {
        let path = self.collection_policy_path(workspace_id, collection_id);
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(PermissionRepositoryError::StorageUnavailable),
        };
        decode_collection_policy(&content).map(Some)
    }

    fn save_collection_policy(
        &mut self,
        workspace_id: &WorkspaceId,
        policy: CollectionPolicy,
    ) -> Result<(), PermissionRepositoryError> {
        write_text_atomically(
            &self.collection_policy_path(workspace_id, policy.collection_id()),
            encode_collection_policy(workspace_id, &policy)?,
        )
        .map(|_| ())
        .map_err(|_| PermissionRepositoryError::StorageUnavailable)
    }

    fn get_document_policy(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentPolicy>, PermissionRepositoryError> {
        let path = self.document_policy_path(workspace_id, document_id);
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(PermissionRepositoryError::StorageUnavailable),
        };
        decode_document_policy(&content).map(Some)
    }

    fn save_document_policy(
        &mut self,
        workspace_id: &WorkspaceId,
        policy: DocumentPolicy,
    ) -> Result<(), PermissionRepositoryError> {
        write_text_atomically(
            &self.document_policy_path(workspace_id, policy.document_id()),
            encode_document_policy(workspace_id, &policy)?,
        )
        .map(|_| ())
        .map_err(|_| PermissionRepositoryError::StorageUnavailable)
    }
}

fn read_assignment_ids_from_index_dir(
    dir: &Path,
) -> Result<Vec<RoleAssignmentId>, PermissionRepositoryError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(PermissionRepositoryError::StorageUnavailable),
    };
    let mut ids = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|_| PermissionRepositoryError::StorageUnavailable)?
            .path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("idx") {
            continue;
        }
        let id = hex_decode(
            fs::read_to_string(path)
                .map_err(|_| PermissionRepositoryError::StorageUnavailable)?
                .trim(),
        )?;
        ids.push(
            RoleAssignmentId::new(&id)
                .map_err(|_| PermissionRepositoryError::StorageUnavailable)?,
        );
    }
    ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    Ok(ids)
}

fn remove_file_if_exists(path: PathBuf) -> Result<(), PermissionRepositoryError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(PermissionRepositoryError::StorageUnavailable),
    }
}

fn encode_assignment(assignment: &RoleAssignment) -> String {
    let (subject_type, subject_id) = match assignment.subject() {
        RoleAssignmentSubject::User(user_id) => ("user", user_id.as_str()),
        RoleAssignmentSubject::Group(group_id) => ("group", group_id.as_str()),
    };
    format!(
        "id={}\nworkspace_id={}\nsubject_type={}\nsubject_id={}\nrole={}\n",
        hex_encode(assignment.id().as_str()),
        hex_encode(assignment.workspace_id().as_str()),
        subject_type,
        hex_encode(subject_id),
        assignment.role().as_str()
    )
}

fn decode_assignment(content: &str) -> Result<RoleAssignment, PermissionRepositoryError> {
    let mut id = None;
    let mut workspace_id = None;
    let mut subject_type = None;
    let mut subject_id = None;
    let mut role = None;
    for line in content.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or(PermissionRepositoryError::StorageUnavailable)?;
        match key {
            "id" => id = Some(hex_decode(value)?),
            "workspace_id" => workspace_id = Some(hex_decode(value)?),
            "subject_type" => subject_type = Some(value),
            "subject_id" => subject_id = Some(hex_decode(value)?),
            "role" => role = Some(parse_role(value)?),
            _ => return Err(PermissionRepositoryError::StorageUnavailable),
        }
    }
    let subject_id = subject_id.ok_or(PermissionRepositoryError::StorageUnavailable)?;
    let subject = match subject_type.ok_or(PermissionRepositoryError::StorageUnavailable)? {
        "user" => RoleAssignmentSubject::User(
            UserId::new(&subject_id).map_err(|_| PermissionRepositoryError::StorageUnavailable)?,
        ),
        "group" => RoleAssignmentSubject::Group(
            GroupId::new(&subject_id).map_err(|_| PermissionRepositoryError::StorageUnavailable)?,
        ),
        _ => return Err(PermissionRepositoryError::StorageUnavailable),
    };
    Ok(RoleAssignment::new(
        RoleAssignmentId::new(&id.ok_or(PermissionRepositoryError::StorageUnavailable)?)
            .map_err(|_| PermissionRepositoryError::StorageUnavailable)?,
        WorkspaceId::new(&workspace_id.ok_or(PermissionRepositoryError::StorageUnavailable)?)
            .map_err(|_| PermissionRepositoryError::StorageUnavailable)?,
        subject,
        role.ok_or(PermissionRepositoryError::StorageUnavailable)?,
    ))
}

fn encode_collection_policy(
    workspace_id: &WorkspaceId,
    policy: &CollectionPolicy,
) -> Result<String, PermissionRepositoryError> {
    let mut lines = vec![format!(
        "collection_id={}",
        hex_encode(policy.collection_id().as_str())
    )];
    for policy_override in policy.overrides() {
        lines.push(format!(
            "override={},{}",
            policy_override.permission().as_str(),
            collection_override_effect(workspace_id, policy.collection_id(), *policy_override)?
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn decode_collection_policy(content: &str) -> Result<CollectionPolicy, PermissionRepositoryError> {
    let mut collection_id = None;
    let mut overrides = Vec::new();
    for line in content.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or(PermissionRepositoryError::StorageUnavailable)?;
        match key {
            "collection_id" => collection_id = Some(hex_decode(value)?),
            "override" => overrides.push(parse_override(value)?),
            _ => return Err(PermissionRepositoryError::StorageUnavailable),
        }
    }
    let mut policy = CollectionPolicy::new(
        CollectionId::new(&collection_id.ok_or(PermissionRepositoryError::StorageUnavailable)?)
            .map_err(|_| PermissionRepositoryError::StorageUnavailable)?,
    );
    for policy_override in overrides {
        policy = policy.with_override(policy_override);
    }
    Ok(policy)
}

fn encode_document_policy(
    workspace_id: &WorkspaceId,
    policy: &DocumentPolicy,
) -> Result<String, PermissionRepositoryError> {
    let mut lines = vec![format!(
        "document_id={}",
        hex_encode(policy.document_id().as_str())
    )];
    for policy_override in policy.overrides() {
        lines.push(format!(
            "override={},{}",
            policy_override.permission().as_str(),
            document_override_effect(workspace_id, policy.document_id(), *policy_override)?
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn decode_document_policy(content: &str) -> Result<DocumentPolicy, PermissionRepositoryError> {
    let mut document_id = None;
    let mut overrides = Vec::new();
    for line in content.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or(PermissionRepositoryError::StorageUnavailable)?;
        match key {
            "document_id" => document_id = Some(hex_decode(value)?),
            "override" => overrides.push(parse_override(value)?),
            _ => return Err(PermissionRepositoryError::StorageUnavailable),
        }
    }
    let mut policy = DocumentPolicy::new(
        DocumentId::new(&document_id.ok_or(PermissionRepositoryError::StorageUnavailable)?)
            .map_err(|_| PermissionRepositoryError::StorageUnavailable)?,
    );
    for policy_override in overrides {
        policy = policy.with_override(policy_override);
    }
    Ok(policy)
}

fn collection_override_effect(
    workspace_id: &WorkspaceId,
    collection_id: &CollectionId,
    policy_override: PolicyOverride,
) -> Result<&'static str, PermissionRepositoryError> {
    let policy = CollectionPolicy::new(collection_id.clone()).with_override(policy_override);
    let subject = AccessSubject::new(
        UserId::new("__policy_encoder")
            .map_err(|_| PermissionRepositoryError::StorageUnavailable)?,
        Vec::new(),
        Vec::new(),
    );
    let resource = AccessResource::collection(workspace_id.clone(), collection_id.clone());
    let decision = policy.decide_with_parent(
        &WorkspacePolicy::default_role_matrix(),
        &subject,
        &resource,
        policy_override.permission(),
    );
    effect_from_decision(decision.result())
}

fn document_override_effect(
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
    policy_override: PolicyOverride,
) -> Result<&'static str, PermissionRepositoryError> {
    let policy = DocumentPolicy::new(document_id.clone()).with_override(policy_override);
    let subject = AccessSubject::new(
        UserId::new("__policy_encoder")
            .map_err(|_| PermissionRepositoryError::StorageUnavailable)?,
        Vec::new(),
        Vec::new(),
    );
    let resource = AccessResource::document(workspace_id.clone(), None, document_id.clone());
    let decision = policy.decide_with_parents(
        &WorkspacePolicy::default_role_matrix(),
        None,
        &subject,
        &resource,
        policy_override.permission(),
    );
    effect_from_decision(decision.result())
}

fn effect_from_decision(
    result: PermissionDecisionResult,
) -> Result<&'static str, PermissionRepositoryError> {
    match result {
        PermissionDecisionResult::Allowed => Ok("allow"),
        PermissionDecisionResult::Denied => Ok("deny"),
        PermissionDecisionResult::NotFound => Ok("hide"),
        PermissionDecisionResult::Indeterminate => {
            Err(PermissionRepositoryError::StorageUnavailable)
        }
    }
}

fn parse_override(value: &str) -> Result<PolicyOverride, PermissionRepositoryError> {
    let (permission, effect) = value
        .split_once(',')
        .ok_or(PermissionRepositoryError::StorageUnavailable)?;
    let permission = parse_permission(permission)?;
    match effect {
        "allow" => Ok(PolicyOverride::allow(permission)),
        "deny" => Ok(PolicyOverride::deny(permission)),
        "hide" => Ok(PolicyOverride::hide(permission)),
        _ => Err(PermissionRepositoryError::StorageUnavailable),
    }
}

fn parse_role(value: &str) -> Result<Role, PermissionRepositoryError> {
    match value {
        "owner" => Ok(Role::Owner),
        "admin" => Ok(Role::Admin),
        "editor" => Ok(Role::Editor),
        "reviewer" => Ok(Role::Reviewer),
        "viewer" => Ok(Role::Viewer),
        _ => Err(PermissionRepositoryError::StorageUnavailable),
    }
}

fn parse_permission(value: &str) -> Result<Permission, PermissionRepositoryError> {
    match value {
        "read" => Ok(Permission::Read),
        "write" => Ok(Permission::Write),
        "review" => Ok(Permission::Review),
        "publish" => Ok(Permission::Publish),
        "manage" => Ok(Permission::Manage),
        "asset_metadata_read" => Ok(Permission::ReadAssetMetadata),
        "asset_content_read" => Ok(Permission::ReadAssetContent),
        _ => Err(PermissionRepositoryError::StorageUnavailable),
    }
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> Result<String, PermissionRepositoryError> {
    if !value.len().is_multiple_of(2) {
        return Err(PermissionRepositoryError::StorageUnavailable);
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| PermissionRepositoryError::StorageUnavailable)?;
    String::from_utf8(bytes).map_err(|_| PermissionRepositoryError::StorageUnavailable)
}
