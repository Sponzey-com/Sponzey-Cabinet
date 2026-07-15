use std::collections::HashMap;

use cabinet_domain::document::DocumentId;
use cabinet_domain::group::GroupId;
use cabinet_domain::permission::{
    CollectionId, CollectionPolicy, DocumentPolicy, Permission, PolicyOverride, Role,
    RoleAssignment, RoleAssignmentId, RoleAssignmentSubject,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::permission_policy_repository::{
    PermissionGroupRepository, PermissionPolicyRepository, PermissionRepositoryError,
    RoleAssignmentIdGenerator, RoleAssignmentMutationResult, RoleAssignmentRemovalResult,
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
        Ok(self
            .assignments
            .values()
            .filter(|assignment| {
                assignment.workspace_id() == workspace_id
                    && matches!(
                        assignment.subject(),
                        RoleAssignmentSubject::Group(group_id)
                            if group_ids.iter().any(|candidate| candidate == group_id)
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
            .get(&assignment_key(workspace_id, assignment_id))
            .cloned())
    }

    fn save_role_assignment(
        &mut self,
        workspace_id: &WorkspaceId,
        assignment: RoleAssignment,
    ) -> Result<RoleAssignmentMutationResult, PermissionRepositoryError> {
        let changed = self
            .assignments
            .insert(assignment_key(workspace_id, assignment.id()), assignment)
            .is_none();
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
            .remove(&assignment_key(workspace_id, assignment_id))
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
            .get(&collection_key(workspace_id, collection_id))
            .cloned())
    }

    fn save_collection_policy(
        &mut self,
        workspace_id: &WorkspaceId,
        policy: CollectionPolicy,
    ) -> Result<(), PermissionRepositoryError> {
        self.collection_policies
            .insert(collection_key(workspace_id, policy.collection_id()), policy);
        Ok(())
    }

    fn get_document_policy(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentPolicy>, PermissionRepositoryError> {
        Ok(self
            .document_policies
            .get(&document_key(workspace_id, document_id))
            .cloned())
    }

    fn save_document_policy(
        &mut self,
        workspace_id: &WorkspaceId,
        policy: DocumentPolicy,
    ) -> Result<(), PermissionRepositoryError> {
        self.document_policies
            .insert(document_key(workspace_id, policy.document_id()), policy);
        Ok(())
    }
}

#[derive(Default)]
struct FakeGroupRepository {
    groups_by_user: HashMap<String, Vec<GroupId>>,
}

impl PermissionGroupRepository for FakeGroupRepository {
    fn list_user_group_ids(
        &self,
        workspace_id: &WorkspaceId,
        user_id: &UserId,
    ) -> Result<Vec<GroupId>, PermissionRepositoryError> {
        Ok(self
            .groups_by_user
            .get(&user_key(workspace_id, user_id))
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

#[test]
fn permission_policy_repository_contract_handles_roles_groups_and_policy_overrides() {
    let mut repository = FakePermissionPolicyRepository::default();
    let mut groups = FakeGroupRepository::default();
    let mut ids = FakeRoleAssignmentIdGenerator::default();
    let workspace_id = WorkspaceId::new("workspace-1").expect("valid workspace id");
    let user_id = UserId::new("user-1").expect("valid user id");
    let group_id = GroupId::new("group-1").expect("valid group id");
    let assignment_id =
        RoleAssignmentId::new(&ids.generate_role_assignment_id()).expect("valid assignment id");
    let group_assignment_id =
        RoleAssignmentId::new(&ids.generate_role_assignment_id()).expect("valid assignment id");

    groups
        .groups_by_user
        .insert(user_key(&workspace_id, &user_id), vec![group_id.clone()]);
    assert_eq!(
        groups
            .list_user_group_ids(&workspace_id, &user_id)
            .expect("list groups"),
        vec![group_id.clone()]
    );

    let user_assignment = RoleAssignment::new(
        assignment_id.clone(),
        workspace_id.clone(),
        RoleAssignmentSubject::User(user_id.clone()),
        Role::Viewer,
    );
    let group_assignment = RoleAssignment::new(
        group_assignment_id.clone(),
        workspace_id.clone(),
        RoleAssignmentSubject::Group(group_id.clone()),
        Role::Reviewer,
    );

    assert_eq!(
        repository
            .save_role_assignment(&workspace_id, user_assignment.clone())
            .expect("save user role"),
        RoleAssignmentMutationResult::Changed
    );
    repository
        .save_role_assignment(&workspace_id, group_assignment.clone())
        .expect("save group role");

    assert_eq!(
        repository
            .list_user_role_assignments(&workspace_id, &user_id)
            .expect("list user roles"),
        vec![user_assignment.clone()]
    );
    assert_eq!(
        repository
            .list_group_role_assignments(&workspace_id, &[group_id.clone()])
            .expect("list group roles"),
        vec![group_assignment.clone()]
    );
    assert_eq!(
        repository
            .list_workspace_role_assignments(&workspace_id)
            .expect("list workspace roles"),
        vec![user_assignment.clone(), group_assignment.clone()]
    );

    let collection_policy =
        CollectionPolicy::new(CollectionId::new("collection-1").expect("valid collection id"))
            .with_override(PolicyOverride::deny(Permission::Write));
    repository
        .save_collection_policy(&workspace_id, collection_policy.clone())
        .expect("save collection policy");
    assert_eq!(
        repository
            .get_collection_policy(&workspace_id, collection_policy.collection_id())
            .expect("load collection policy"),
        Some(collection_policy)
    );

    let document_policy =
        DocumentPolicy::new(DocumentId::new("document-1").expect("valid document id"))
            .with_override(PolicyOverride::allow(Permission::Write));
    repository
        .save_document_policy(&workspace_id, document_policy.clone())
        .expect("save document policy");
    assert_eq!(
        repository
            .get_document_policy(&workspace_id, document_policy.document_id())
            .expect("load document policy"),
        Some(document_policy)
    );

    assert_eq!(
        repository
            .remove_role_assignment(&workspace_id, &assignment_id)
            .expect("remove role"),
        RoleAssignmentRemovalResult::Removed
    );
    assert_eq!(
        repository
            .get_role_assignment(&workspace_id, &assignment_id)
            .expect("load removed role"),
        None
    );
}

fn assignment_key(workspace_id: &WorkspaceId, assignment_id: &RoleAssignmentId) -> String {
    format!("{}:{}", workspace_id.as_str(), assignment_id.as_str())
}

fn collection_key(workspace_id: &WorkspaceId, collection_id: &CollectionId) -> String {
    format!("{}:{}", workspace_id.as_str(), collection_id.as_str())
}

fn document_key(workspace_id: &WorkspaceId, document_id: &DocumentId) -> String {
    format!("{}:{}", workspace_id.as_str(), document_id.as_str())
}

fn user_key(workspace_id: &WorkspaceId, user_id: &UserId) -> String {
    format!("{}:{}", workspace_id.as_str(), user_id.as_str())
}
