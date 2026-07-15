use std::fs;
use std::path::PathBuf;

use cabinet_adapters::local_review_workflow_repository::LocalReviewWorkflowRepository;
use cabinet_domain::document::DocumentId;
use cabinet_domain::user::UserId;
use cabinet_domain::workflow::{PublishWorkflowState, ReviewRequest};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::review_workflow::{
    ReviewRequestStatus, ReviewWorkflowRepository, ReviewWorkflowRepositoryError,
};

#[test]
fn local_review_workflow_repository_persists_workflow_state_across_instances() {
    let root = unique_temp_dir("local-review-workflow-state");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");

    {
        let mut repository = LocalReviewWorkflowRepository::new(root.clone());
        repository
            .save_workflow_state(&workspace_id, &document_id, PublishWorkflowState::Approved)
            .expect("save state");
    }

    let repository = LocalReviewWorkflowRepository::new(root.clone());
    assert_eq!(
        repository
            .get_workflow_state(&workspace_id, &document_id)
            .expect("get state"),
        Some(PublishWorkflowState::Approved)
    );
    assert!(!format!("{repository:?}").contains("doc-1"));
    cleanup_temp_dir(root);
}

#[test]
fn local_review_workflow_repository_persists_review_requests_and_document_index() {
    let root = unique_temp_dir("local-review-workflow-requests");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let request = ReviewRequest::new(
        document_id.clone(),
        UserId::new("requester-1").expect("user id"),
    );
    let mut repository = LocalReviewWorkflowRepository::new(root.clone());

    let record = repository
        .save_review_request(&workspace_id, "review-1", request)
        .expect("save request");
    let approved = repository
        .update_review_request_status(&workspace_id, "review-1", ReviewRequestStatus::Approved)
        .expect("update request")
        .expect("updated request");

    let mut restarted = LocalReviewWorkflowRepository::new(root.clone());
    assert_eq!(record.status(), ReviewRequestStatus::ReviewRequested);
    assert_eq!(approved.status(), ReviewRequestStatus::Approved);
    assert_eq!(
        restarted
            .get_review_request(&workspace_id, "review-1")
            .expect("get request")
            .expect("request")
            .status(),
        ReviewRequestStatus::Approved
    );
    assert_eq!(
        restarted
            .list_review_requests(&workspace_id, Some(&document_id))
            .expect("list document requests")
            .len(),
        1
    );
    assert_eq!(
        restarted
            .list_review_requests(&workspace_id, None)
            .expect("list workspace requests")
            .len(),
        1
    );
    assert!(
        restarted
            .update_review_request_status(
                &workspace_id,
                "missing-review",
                ReviewRequestStatus::Rejected,
            )
            .expect("missing update")
            .is_none()
    );
    cleanup_temp_dir(root);
}

#[test]
fn local_review_workflow_repository_reports_invalid_id_and_corrupted_files() {
    let root = unique_temp_dir("local-review-workflow-errors");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let request = ReviewRequest::new(
        document_id.clone(),
        UserId::new("requester-1").expect("user id"),
    );
    let mut repository = LocalReviewWorkflowRepository::new(root.clone());

    let invalid = repository
        .save_review_request(&workspace_id, " \n ", request.clone())
        .expect_err("invalid id must fail");
    repository
        .save_workflow_state(&workspace_id, &document_id, PublishWorkflowState::Approved)
        .expect("save state");
    fs::write(
        first_file_under(&root.join("review-workflows"), "state"),
        "bad-state",
    )
    .expect("corrupt state");
    let corrupted_state = repository
        .get_workflow_state(&workspace_id, &document_id)
        .expect_err("corrupted state must fail");

    repository
        .save_review_request(&workspace_id, "review-1", request)
        .expect("save request");
    fs::write(
        first_file_under(&root.join("review-workflows"), "request"),
        "bad-request",
    )
    .expect("corrupt request");
    let corrupted_request = repository
        .get_review_request(&workspace_id, "review-1")
        .expect_err("corrupted request must fail");

    assert_eq!(
        invalid,
        ReviewWorkflowRepositoryError::InvalidReviewRequestId
    );
    assert_eq!(
        corrupted_state,
        ReviewWorkflowRepositoryError::CorruptedState
    );
    assert_eq!(
        corrupted_request,
        ReviewWorkflowRepositoryError::CorruptedState
    );
    cleanup_temp_dir(root);
}

fn first_file_under(root: &PathBuf, extension: &str) -> PathBuf {
    let mut stack = vec![root.clone()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(path).expect("read dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
                return path;
            }
        }
    }
    panic!("file with extension {extension} not found");
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sponzey-cabinet-{name}-{}", std::process::id()));
    cleanup_temp_dir(dir.clone());
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn cleanup_temp_dir(dir: PathBuf) {
    if dir.exists() {
        fs::remove_dir_all(dir).expect("remove temp dir");
    }
}
