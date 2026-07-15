use std::collections::{HashMap, HashSet};

use cabinet_domain::document::DocumentId;
use cabinet_domain::group::GroupId;
use cabinet_domain::permission::{
    CollectionId, CollectionPolicy, DocumentPolicy, Permission, PermissionDecisionResult,
    PolicySource, Role, RoleAssignment, RoleAssignmentId, RoleAssignmentSubject,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::permission_policy_repository::{
    PermissionGroupRepository, PermissionPolicyRepository, PermissionRepositoryError,
    RoleAssignmentIdGenerator, RoleAssignmentMutationResult, RoleAssignmentRemovalResult,
};
use cabinet_usecases::permission::{
    AssignRoleError, AssignRoleInput, AssignRoleUsecase, CheckPermissionInput,
    CheckPermissionUsecase, ListEffectivePermissionsInput, ListEffectivePermissionsUsecase,
    ListWorkspaceRoleAssignmentsInput, ListWorkspaceRoleAssignmentsUsecase, PermissionProductEvent,
    PermissionUsecaseLogger, RevokeRoleError, RevokeRoleInput, RevokeRoleUsecase,
    SetCollectionPermissionInput, SetCollectionPermissionUsecase, ShareDocumentError,
    ShareDocumentInput, ShareDocumentUsecase,
};

#[derive(Default)]
struct FakePermissionPolicyRepository {
    assignments: HashMap<String, RoleAssignment>,
    collection_policies: HashMap<String, CollectionPolicy>,
    document_policies: HashMap<String, DocumentPolicy>,
}

impl PermissionPolicyRepository for FakePermissionPolicyRepository {
    fn list_user_role_assignments(
        &self,
        workspace_id: &WorkspaceId,
        user_id: &UserId,
    ) -> Result<Vec<RoleAssignment>, PermissionRepositoryError> {
        Ok(self
            .assignments
            .values()
            .filter(|assignment| {
                assignment.workspace_id() == workspace_id
                    && assignment.subject() == &RoleAssignmentSubject::User(user_id.clone())
            })
            .cloned()
            .collect())
    }

    fn list_group_role_assignments(
        &self,
        workspace_id: &WorkspaceId,
        group_ids: &[GroupId],
    ) -> Result<Vec<RoleAssignment>, PermissionRepositoryError> {
        let group_set = group_ids
            .iter()
            .map(|group_id| group_id.as_str().to_string())
            .collect::<HashSet<_>>();
        Ok(self
            .assignments
            .values()
            .filter(|assignment| {
                assignment.workspace_id() == workspace_id
                    && matches!(
                        assignment.subject(),
                        RoleAssignmentSubject::Group(group_id)
                            if group_set.contains(group_id.as_str())
                    )
            })
            .cloned()
            .collect())
    }

    fn list_workspace_role_assignments(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<RoleAssignment>, PermissionRepositoryError> {
        let mut assignments = self
            .assignments
            .values()
            .filter(|assignment| assignment.workspace_id() == workspace_id)
            .cloned()
            .collect::<Vec<_>>();
        assignments.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        Ok(assignments)
    }

    fn get_role_assignment(
        &self,
        workspace_id: &WorkspaceId,
        assignment_id: &RoleAssignmentId,
    ) -> Result<Option<RoleAssignment>, PermissionRepositoryError> {
        Ok(self
            .assignments
            .get(&role_assignment_key(workspace_id, assignment_id))
            .cloned())
    }

    fn save_role_assignment(
        &mut self,
        workspace_id: &WorkspaceId,
        assignment: RoleAssignment,
    ) -> Result<RoleAssignmentMutationResult, PermissionRepositoryError> {
        let key = role_assignment_key(workspace_id, assignment.id());
        let changed = self.assignments.insert(key, assignment).is_none();
        Ok(if changed {
            RoleAssignmentMutationResult::Changed
        } else {
            RoleAssignmentMutationResult::AlreadyApplied
        })
    }

    fn remove_role_assignment(
        &mut self,
        workspace_id: &WorkspaceId,
        assignment_id: &RoleAssignmentId,
    ) -> Result<RoleAssignmentRemovalResult, PermissionRepositoryError> {
        let removed = self
            .assignments
            .remove(&role_assignment_key(workspace_id, assignment_id))
            .is_some();
        Ok(if removed {
            RoleAssignmentRemovalResult::Removed
        } else {
            RoleAssignmentRemovalResult::Missing
        })
    }

    fn get_collection_policy(
        &self,
        workspace_id: &WorkspaceId,
        collection_id: &CollectionId,
    ) -> Result<Option<CollectionPolicy>, PermissionRepositoryError> {
        Ok(self
            .collection_policies
            .get(&collection_policy_key(workspace_id, collection_id))
            .cloned())
    }

    fn save_collection_policy(
        &mut self,
        workspace_id: &WorkspaceId,
        policy: CollectionPolicy,
    ) -> Result<(), PermissionRepositoryError> {
        self.collection_policies.insert(
            collection_policy_key(workspace_id, policy.collection_id()),
            policy,
        );
        Ok(())
    }

    fn get_document_policy(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentPolicy>, PermissionRepositoryError> {
        Ok(self
            .document_policies
            .get(&document_policy_key(workspace_id, document_id))
            .cloned())
    }

    fn save_document_policy(
        &mut self,
        workspace_id: &WorkspaceId,
        policy: DocumentPolicy,
    ) -> Result<(), PermissionRepositoryError> {
        self.document_policies.insert(
            document_policy_key(workspace_id, policy.document_id()),
            policy,
        );
        Ok(())
    }
}

#[derive(Default)]
struct FakeGroupRepository {
    memberships: HashMap<String, Vec<GroupId>>,
}

impl PermissionGroupRepository for FakeGroupRepository {
    fn list_user_group_ids(
        &self,
        workspace_id: &WorkspaceId,
        user_id: &UserId,
    ) -> Result<Vec<GroupId>, PermissionRepositoryError> {
        Ok(self
            .memberships
            .get(&user_group_key(workspace_id, user_id))
            .cloned()
            .unwrap_or_default())
    }
}

#[derive(Default)]
struct FakeRoleAssignmentIdGenerator {
    next: u32,
}

impl RoleAssignmentIdGenerator for FakeRoleAssignmentIdGenerator {
    fn generate_role_assignment_id(&mut self) -> String {
        self.next += 1;
        format!("role-assignment-{}", self.next)
    }
}

#[derive(Default)]
struct FakePermissionLogger {
    product_events: Vec<PermissionProductEvent>,
    field_debug_count: usize,
}

impl PermissionUsecaseLogger for FakePermissionLogger {
    fn write_product(&mut self, event: PermissionProductEvent) {
        self.product_events.push(event);
    }

    fn write_field_debug(
        &mut self,
        _event: cabinet_usecases::permission::PermissionFieldDebugEvent,
    ) {
        self.field_debug_count += 1;
    }
}

#[test]
fn check_permission_returns_allowed_and_denied_decisions_from_domain_matrix() {
    let policy_repository = repository_with_user_role("workspace-1", "actor-1", Role::Editor);
    let group_repository = FakeGroupRepository::default();
    let mut logger = FakePermissionLogger::default();
    let usecase = CheckPermissionUsecase::new();

    let allowed = usecase
        .execute(
            CheckPermissionInput::document(
                "actor-1",
                "workspace-1",
                Some("collection-1"),
                "document-1",
                Permission::Write,
            ),
            &policy_repository,
            &group_repository,
            &mut logger,
        )
        .expect("check write");
    let denied = usecase
        .execute(
            CheckPermissionInput::document(
                "actor-1",
                "workspace-1",
                Some("collection-1"),
                "document-1",
                Permission::Publish,
            ),
            &policy_repository,
            &group_repository,
            &mut logger,
        )
        .expect("check publish");

    assert_eq!(
        allowed.decision().result(),
        PermissionDecisionResult::Allowed
    );
    assert_eq!(denied.decision().result(), PermissionDecisionResult::Denied);
    assert_eq!(denied.decision().source(), PolicySource::Workspace);
    assert_eq!(
        denied.decision().reason_code(),
        "ROLE_DOES_NOT_ALLOW_PERMISSION"
    );
    assert_eq!(logger.field_debug_count, 2);
    assert_eq!(
        logger.product_events,
        vec![PermissionProductEvent::PermissionDenied {
            masked_actor_id: "masked:or-1".to_string(),
            target_id: "document:masked:nt-1".to_string(),
            permission: "publish",
            decision: "Denied",
            error_code: "ROLE_DOES_NOT_ALLOW_PERMISSION",
        }]
    );
}

#[test]
fn assign_role_requires_manage_permission_and_writes_safe_product_log() {
    let mut repository = repository_with_user_role("workspace-1", "actor-1", Role::Viewer);
    let group_repository = FakeGroupRepository::default();
    let mut ids = FakeRoleAssignmentIdGenerator::default();
    let mut logger = FakePermissionLogger::default();

    let error = AssignRoleUsecase::new()
        .execute(
            AssignRoleInput::for_user("actor-1", "workspace-1", "target-1", Role::Editor),
            &mut repository,
            &group_repository,
            &mut ids,
            &mut logger,
        )
        .expect_err("viewer cannot assign role");

    assert_eq!(error, AssignRoleError::Unauthorized);
    assert_eq!(
        logger.product_events,
        vec![PermissionProductEvent::PermissionDenied {
            masked_actor_id: "masked:or-1".to_string(),
            target_id: "workspace:masked:ce-1".to_string(),
            permission: "manage",
            decision: "Denied",
            error_code: "ROLE_DOES_NOT_ALLOW_PERMISSION",
        }]
    );

    repository = repository_with_user_role("workspace-1", "actor-1", Role::Owner);
    logger.product_events.clear();

    let output = AssignRoleUsecase::new()
        .execute(
            AssignRoleInput::for_user("actor-1", "workspace-1", "target-1", Role::Editor),
            &mut repository,
            &group_repository,
            &mut ids,
            &mut logger,
        )
        .expect("owner can assign role");

    assert_eq!(output.assignment().id().as_str(), "role-assignment-1");
    assert_eq!(output.assignment().role(), Role::Editor);
    assert_eq!(repository.assignments.len(), 2);
    assert_eq!(
        logger.product_events.last(),
        Some(&PermissionProductEvent::RoleAssigned {
            masked_actor_id: "masked:or-1".to_string(),
            target_id: "user:masked:et-1".to_string(),
            role: "editor",
        })
    );
}

#[test]
fn revoke_role_requires_manage_permission_and_removes_assignment() {
    let mut repository = repository_with_user_role("workspace-1", "actor-1", Role::Owner);
    let assignment = role_assignment(
        "role-assignment-9",
        "workspace-1",
        RoleAssignmentSubject::User(user_id("target-1")),
        Role::Viewer,
    );
    repository
        .save_role_assignment(&workspace_id("workspace-1"), assignment.clone())
        .expect("seed assignment");
    let group_repository = FakeGroupRepository::default();
    let mut logger = FakePermissionLogger::default();

    let output = RevokeRoleUsecase::new()
        .execute(
            RevokeRoleInput::new("actor-1", "workspace-1", "role-assignment-9"),
            &mut repository,
            &group_repository,
            &mut logger,
        )
        .expect("revoke role");

    assert_eq!(output.assignment_id(), "role-assignment-9");
    assert_eq!(
        repository.get_role_assignment(&workspace_id("workspace-1"), assignment.id()),
        Ok(None)
    );
    assert_eq!(
        logger.product_events.last(),
        Some(&PermissionProductEvent::RoleRevoked {
            masked_actor_id: "masked:or-1".to_string(),
            target_id: "role_assignment:masked:nt-9".to_string(),
        })
    );

    let error = RevokeRoleUsecase::new()
        .execute(
            RevokeRoleInput::new("actor-1", "workspace-1", "missing-assignment"),
            &mut repository,
            &group_repository,
            &mut logger,
        )
        .expect_err("missing assignment");
    assert_eq!(error, RevokeRoleError::RoleAssignmentNotFound);
}

#[test]
fn share_document_and_collection_permission_keep_override_sources_distinct() {
    let mut repository = repository_with_user_role("workspace-1", "owner-1", Role::Owner);
    seed_assignment(&mut repository, "workspace-1", "editor-1", Role::Editor);
    let group_repository = FakeGroupRepository::default();
    let mut logger = FakePermissionLogger::default();

    SetCollectionPermissionUsecase::new()
        .execute(
            SetCollectionPermissionInput::deny(
                "owner-1",
                "workspace-1",
                "collection-1",
                Permission::Write,
            ),
            &mut repository,
            &group_repository,
            &mut logger,
        )
        .expect("set collection deny");

    let denied = CheckPermissionUsecase::new()
        .execute(
            CheckPermissionInput::document(
                "editor-1",
                "workspace-1",
                Some("collection-1"),
                "document-1",
                Permission::Write,
            ),
            &repository,
            &group_repository,
            &mut logger,
        )
        .expect("check denied by collection");
    assert_eq!(denied.decision().result(), PermissionDecisionResult::Denied);
    assert_eq!(denied.decision().source(), PolicySource::Collection);

    ShareDocumentUsecase::new()
        .execute(
            ShareDocumentInput::allow(
                "owner-1",
                "workspace-1",
                Some("collection-1"),
                "document-1",
                Permission::Write,
            ),
            &mut repository,
            &group_repository,
            &mut logger,
        )
        .expect("share document allow");

    let allowed = CheckPermissionUsecase::new()
        .execute(
            CheckPermissionInput::document(
                "editor-1",
                "workspace-1",
                Some("collection-1"),
                "document-1",
                Permission::Write,
            ),
            &repository,
            &group_repository,
            &mut logger,
        )
        .expect("check allowed by document");
    assert_eq!(
        allowed.decision().result(),
        PermissionDecisionResult::Allowed
    );
    assert_eq!(allowed.decision().source(), PolicySource::Document);
    assert!(logger.product_events.iter().any(|event| {
        matches!(
            event,
            PermissionProductEvent::PolicyChanged {
                target_id,
                permission: "write",
                ..
            } if target_id == "document:masked:nt-1"
        )
    }));
}

#[test]
fn share_document_requires_manage_permission() {
    let mut repository = repository_with_user_role("workspace-1", "actor-1", Role::Editor);
    let group_repository = FakeGroupRepository::default();
    let mut logger = FakePermissionLogger::default();

    let error = ShareDocumentUsecase::new()
        .execute(
            ShareDocumentInput::allow(
                "actor-1",
                "workspace-1",
                None,
                "document-1",
                Permission::Read,
            ),
            &mut repository,
            &group_repository,
            &mut logger,
        )
        .expect_err("editor cannot manage sharing");

    assert_eq!(error, ShareDocumentError::Unauthorized);
    assert!(repository.document_policies.is_empty());
}

#[test]
fn list_effective_permissions_reuses_check_permission_path() {
    let repository = repository_with_user_role("workspace-1", "actor-1", Role::Reviewer);
    let group_repository = FakeGroupRepository::default();
    let mut logger = FakePermissionLogger::default();

    let output = ListEffectivePermissionsUsecase::new()
        .execute(
            ListEffectivePermissionsInput::asset(
                "actor-1",
                "workspace-1",
                Some("collection-1"),
                Some("document-1"),
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
            &repository,
            &group_repository,
            &mut logger,
        )
        .expect("list effective permissions");

    assert!(
        output
            .allowed_permissions()
            .contains(&Permission::ReadAssetMetadata)
    );
    assert!(
        !output
            .allowed_permissions()
            .contains(&Permission::ReadAssetContent)
    );
    assert!(!output.allowed_permissions().contains(&Permission::Publish));
}

#[test]
fn list_workspace_role_assignments_returns_safe_explicit_dto_without_logs() {
    let mut repository = repository_with_user_role("workspace-1", "actor-1", Role::Owner);
    let group_assignment = role_assignment(
        "role-assignment-9",
        "workspace-1",
        RoleAssignmentSubject::Group(GroupId::new("group-1").expect("group id")),
        Role::Reviewer,
    );
    repository
        .save_role_assignment(&workspace_id("workspace-1"), group_assignment)
        .expect("seed group role");
    repository
        .save_role_assignment(
            &workspace_id("workspace-2"),
            role_assignment(
                "role-assignment-other",
                "workspace-2",
                RoleAssignmentSubject::User(user_id("other-user")),
                Role::Viewer,
            ),
        )
        .expect("seed foreign role");
    let logger = FakePermissionLogger::default();

    let output = ListWorkspaceRoleAssignmentsUsecase::new()
        .execute(
            ListWorkspaceRoleAssignmentsInput::new("workspace-1"),
            &repository,
        )
        .expect("list workspace role assignments");

    assert_eq!(output.assignments().len(), 2);
    assert_eq!(output.assignments()[0].assignment_id(), "role-assignment-9");
    assert_eq!(output.assignments()[0].subject_type(), "group");
    assert_eq!(output.assignments()[0].subject_id(), "group-1");
    assert_eq!(output.assignments()[0].role(), "reviewer");
    assert_eq!(output.assignments()[1].subject_type(), "user");
    assert_eq!(output.assignments()[1].subject_id(), "actor-1");
    assert_eq!(output.assignments()[1].role(), "owner");
    assert!(logger.product_events.is_empty());
}

fn repository_with_user_role(
    workspace: &str,
    user: &str,
    role: Role,
) -> FakePermissionPolicyRepository {
    let mut repository = FakePermissionPolicyRepository::default();
    seed_assignment(&mut repository, workspace, user, role);
    repository
}

fn seed_assignment(
    repository: &mut FakePermissionPolicyRepository,
    workspace: &str,
    user: &str,
    role: Role,
) {
    let assignment_id = format!("seed-{}-{}", user, role.as_str());
    let assignment = role_assignment(
        &assignment_id,
        workspace,
        RoleAssignmentSubject::User(user_id(user)),
        role,
    );
    repository
        .save_role_assignment(&workspace_id(workspace), assignment)
        .expect("seed role assignment");
}

fn role_assignment(
    assignment_id: &str,
    workspace: &str,
    subject: RoleAssignmentSubject,
    role: Role,
) -> RoleAssignment {
    RoleAssignment::new(
        RoleAssignmentId::new(assignment_id).expect("valid assignment id"),
        workspace_id(workspace),
        subject,
        role,
    )
}

fn workspace_id(value: &str) -> WorkspaceId {
    WorkspaceId::new(value).expect("valid workspace id")
}

fn user_id(value: &str) -> UserId {
    UserId::new(value).expect("valid user id")
}

fn role_assignment_key(workspace_id: &WorkspaceId, assignment_id: &RoleAssignmentId) -> String {
    format!("{}:{}", workspace_id.as_str(), assignment_id.as_str())
}

fn collection_policy_key(workspace_id: &WorkspaceId, collection_id: &CollectionId) -> String {
    format!("{}:{}", workspace_id.as_str(), collection_id.as_str())
}

fn document_policy_key(workspace_id: &WorkspaceId, document_id: &DocumentId) -> String {
    format!("{}:{}", workspace_id.as_str(), document_id.as_str())
}

fn user_group_key(workspace_id: &WorkspaceId, user_id: &UserId) -> String {
    format!("{}:{}", workspace_id.as_str(), user_id.as_str())
}
