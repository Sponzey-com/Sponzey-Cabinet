use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use cabinet_domain::document::DocumentId;
use cabinet_domain::user::UserId;
use cabinet_domain::workflow::{PublishWorkflowState, ReviewRequest};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::review_workflow::{
    ReviewRequestRecord, ReviewRequestStatus, ReviewWorkflowRepository,
    ReviewWorkflowRepositoryError,
};

use crate::local_atomic_file::write_text_atomically;

pub const LOCAL_REVIEW_WORKFLOWS_DIR: &str = "review-workflows";
pub const LOCAL_WORKFLOW_STATES_DIR: &str = "states";
pub const LOCAL_REVIEW_REQUESTS_DIR: &str = "requests";
pub const LOCAL_REVIEW_REQUESTS_BY_ID_DIR: &str = "by-id";
pub const LOCAL_REVIEW_REQUESTS_BY_DOCUMENT_DIR: &str = "by-document";

#[derive(Clone, PartialEq, Eq)]
pub struct LocalReviewWorkflowRepository {
    root: PathBuf,
}

impl fmt::Debug for LocalReviewWorkflowRepository {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalReviewWorkflowRepository")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl LocalReviewWorkflowRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn workspace_root(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.root
            .join(LOCAL_REVIEW_WORKFLOWS_DIR)
            .join(hex_encode(workspace_id.as_str()))
    }

    fn state_path(&self, workspace_id: &WorkspaceId, document_id: &DocumentId) -> PathBuf {
        self.workspace_root(workspace_id)
            .join(LOCAL_WORKFLOW_STATES_DIR)
            .join(format!("{}.state", hex_encode(document_id.as_str())))
    }

    fn request_path(&self, workspace_id: &WorkspaceId, review_request_id: &str) -> PathBuf {
        self.workspace_root(workspace_id)
            .join(LOCAL_REVIEW_REQUESTS_DIR)
            .join(LOCAL_REVIEW_REQUESTS_BY_ID_DIR)
            .join(format!("{}.request", hex_encode(review_request_id)))
    }

    fn document_index_path(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        review_request_id: &str,
    ) -> PathBuf {
        self.workspace_root(workspace_id)
            .join(LOCAL_REVIEW_REQUESTS_DIR)
            .join(LOCAL_REVIEW_REQUESTS_BY_DOCUMENT_DIR)
            .join(hex_encode(document_id.as_str()))
            .join(format!("{}.idx", hex_encode(review_request_id)))
    }

    fn write_request(
        &self,
        workspace_id: &WorkspaceId,
        record: &ReviewRequestRecord,
    ) -> Result<(), ReviewWorkflowRepositoryError> {
        if !record.workspace_matches(workspace_id) {
            return Err(ReviewWorkflowRepositoryError::CorruptedState);
        }
        write_text_atomically(
            &self.request_path(workspace_id, record.review_request_id()),
            encode_request_record(record),
        )
        .map(|_| ())
        .map_err(|_| ReviewWorkflowRepositoryError::StorageUnavailable)?;
        write_text_atomically(
            &self.document_index_path(
                workspace_id,
                record.request().document_id(),
                record.review_request_id(),
            ),
            format!("{}\n", hex_encode(record.review_request_id())),
        )
        .map(|_| ())
        .map_err(|_| ReviewWorkflowRepositoryError::StorageUnavailable)
    }

    fn load_request(
        &self,
        workspace_id: &WorkspaceId,
        review_request_id: &str,
    ) -> Result<Option<ReviewRequestRecord>, ReviewWorkflowRepositoryError> {
        validate_review_request_id(review_request_id)?;
        let path = self.request_path(workspace_id, review_request_id);
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(ReviewWorkflowRepositoryError::StorageUnavailable),
        };
        let record = decode_request_record(&content)?;
        if !record.workspace_matches(workspace_id) {
            return Err(ReviewWorkflowRepositoryError::CorruptedState);
        }
        Ok(Some(record))
    }

    fn list_workspace_review_requests(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<ReviewRequestRecord>, ReviewWorkflowRepositoryError> {
        let root = self
            .workspace_root(workspace_id)
            .join(LOCAL_REVIEW_REQUESTS_DIR)
            .join(LOCAL_REVIEW_REQUESTS_BY_ID_DIR);
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(ReviewWorkflowRepositoryError::StorageUnavailable),
        };
        let mut records = Vec::new();
        for entry in entries {
            let path = entry
                .map_err(|_| ReviewWorkflowRepositoryError::StorageUnavailable)?
                .path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("request") {
                continue;
            }
            let record = decode_request_record(
                &fs::read_to_string(path)
                    .map_err(|_| ReviewWorkflowRepositoryError::StorageUnavailable)?,
            )?;
            if !record.workspace_matches(workspace_id) {
                return Err(ReviewWorkflowRepositoryError::CorruptedState);
            }
            records.push(record);
        }
        Ok(records)
    }

    fn list_review_requests_by_document(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Vec<ReviewRequestRecord>, ReviewWorkflowRepositoryError> {
        let root = self
            .workspace_root(workspace_id)
            .join(LOCAL_REVIEW_REQUESTS_DIR)
            .join(LOCAL_REVIEW_REQUESTS_BY_DOCUMENT_DIR)
            .join(hex_encode(document_id.as_str()));
        let request_ids = read_request_ids_from_index_dir(&root)?;
        let mut records = Vec::new();
        for request_id in request_ids {
            let Some(record) = self.load_request(workspace_id, &request_id)? else {
                return Err(ReviewWorkflowRepositoryError::CorruptedState);
            };
            if record.request().document_id() != document_id {
                return Err(ReviewWorkflowRepositoryError::CorruptedState);
            }
            records.push(record);
        }
        Ok(records)
    }
}

impl ReviewWorkflowRepository for LocalReviewWorkflowRepository {
    fn get_workflow_state(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<PublishWorkflowState>, ReviewWorkflowRepositoryError> {
        let path = self.state_path(workspace_id, document_id);
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(ReviewWorkflowRepositoryError::StorageUnavailable),
        };
        parse_workflow_state(content.trim()).map(Some)
    }

    fn save_workflow_state(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        state: PublishWorkflowState,
    ) -> Result<(), ReviewWorkflowRepositoryError> {
        write_text_atomically(
            &self.state_path(workspace_id, document_id),
            format!("{}\n", workflow_state_name(state)),
        )
        .map(|_| ())
        .map_err(|_| ReviewWorkflowRepositoryError::StorageUnavailable)
    }

    fn save_review_request(
        &mut self,
        workspace_id: &WorkspaceId,
        review_request_id: &str,
        request: ReviewRequest,
    ) -> Result<ReviewRequestRecord, ReviewWorkflowRepositoryError> {
        validate_review_request_id(review_request_id)?;
        let record = ReviewRequestRecord::new(
            workspace_id,
            review_request_id,
            request,
            ReviewRequestStatus::ReviewRequested,
        )?;
        self.write_request(workspace_id, &record)?;
        Ok(record)
    }

    fn get_review_request(
        &self,
        workspace_id: &WorkspaceId,
        review_request_id: &str,
    ) -> Result<Option<ReviewRequestRecord>, ReviewWorkflowRepositoryError> {
        self.load_request(workspace_id, review_request_id)
    }

    fn update_review_request_status(
        &mut self,
        workspace_id: &WorkspaceId,
        review_request_id: &str,
        status: ReviewRequestStatus,
    ) -> Result<Option<ReviewRequestRecord>, ReviewWorkflowRepositoryError> {
        let Some(record) = self.load_request(workspace_id, review_request_id)? else {
            return Ok(None);
        };
        let updated = record.with_status(status);
        self.write_request(workspace_id, &updated)?;
        Ok(Some(updated))
    }

    fn list_review_requests(
        &self,
        workspace_id: &WorkspaceId,
        document_id: Option<&DocumentId>,
    ) -> Result<Vec<ReviewRequestRecord>, ReviewWorkflowRepositoryError> {
        let mut records = match document_id {
            Some(document_id) => {
                self.list_review_requests_by_document(workspace_id, document_id)?
            }
            None => self.list_workspace_review_requests(workspace_id)?,
        };
        records.sort_by(|left, right| left.review_request_id().cmp(right.review_request_id()));
        Ok(records)
    }
}

fn read_request_ids_from_index_dir(
    dir: &Path,
) -> Result<Vec<String>, ReviewWorkflowRepositoryError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(ReviewWorkflowRepositoryError::StorageUnavailable),
    };
    let mut ids = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|_| ReviewWorkflowRepositoryError::StorageUnavailable)?
            .path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("idx") {
            continue;
        }
        let id = hex_decode(
            fs::read_to_string(path)
                .map_err(|_| ReviewWorkflowRepositoryError::StorageUnavailable)?
                .trim(),
        )?;
        validate_review_request_id(&id)?;
        ids.push(id);
    }
    ids.sort();
    Ok(ids)
}

fn encode_request_record(record: &ReviewRequestRecord) -> String {
    format!(
        "workspace_id={}\nreview_request_id={}\ndocument_id={}\nrequested_by={}\nstatus={}\n",
        hex_encode(record.workspace_id().as_str()),
        hex_encode(record.review_request_id()),
        hex_encode(record.request().document_id().as_str()),
        hex_encode(record.request().requested_by().as_str()),
        record.status().as_str()
    )
}

fn decode_request_record(
    content: &str,
) -> Result<ReviewRequestRecord, ReviewWorkflowRepositoryError> {
    let mut workspace_id = None;
    let mut review_request_id = None;
    let mut document_id = None;
    let mut requested_by = None;
    let mut status = None;
    for line in content.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or(ReviewWorkflowRepositoryError::CorruptedState)?;
        match key {
            "workspace_id" => workspace_id = Some(hex_decode(value)?),
            "review_request_id" => review_request_id = Some(hex_decode(value)?),
            "document_id" => document_id = Some(hex_decode(value)?),
            "requested_by" => requested_by = Some(hex_decode(value)?),
            "status" => status = Some(parse_review_request_status(value)?),
            _ => return Err(ReviewWorkflowRepositoryError::CorruptedState),
        }
    }
    let workspace_id =
        WorkspaceId::new(&workspace_id.ok_or(ReviewWorkflowRepositoryError::CorruptedState)?)
            .map_err(|_| ReviewWorkflowRepositoryError::CorruptedState)?;
    let review_request_id =
        review_request_id.ok_or(ReviewWorkflowRepositoryError::CorruptedState)?;
    let request = ReviewRequest::new(
        DocumentId::new(&document_id.ok_or(ReviewWorkflowRepositoryError::CorruptedState)?)
            .map_err(|_| ReviewWorkflowRepositoryError::CorruptedState)?,
        UserId::new(&requested_by.ok_or(ReviewWorkflowRepositoryError::CorruptedState)?)
            .map_err(|_| ReviewWorkflowRepositoryError::CorruptedState)?,
    );
    ReviewRequestRecord::new(
        &workspace_id,
        &review_request_id,
        request,
        status.ok_or(ReviewWorkflowRepositoryError::CorruptedState)?,
    )
    .map_err(|_| ReviewWorkflowRepositoryError::CorruptedState)
}

fn validate_review_request_id(value: &str) -> Result<(), ReviewWorkflowRepositoryError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(ReviewWorkflowRepositoryError::InvalidReviewRequestId);
    }
    Ok(())
}

fn workflow_state_name(state: PublishWorkflowState) -> &'static str {
    match state {
        PublishWorkflowState::Editing => "editing",
        PublishWorkflowState::ReviewRequested => "review_requested",
        PublishWorkflowState::ChangesRequested => "changes_requested",
        PublishWorkflowState::Approved => "approved",
        PublishWorkflowState::Published => "published",
        PublishWorkflowState::Rejected => "rejected",
    }
}

fn parse_workflow_state(
    value: &str,
) -> Result<PublishWorkflowState, ReviewWorkflowRepositoryError> {
    match value {
        "editing" => Ok(PublishWorkflowState::Editing),
        "review_requested" => Ok(PublishWorkflowState::ReviewRequested),
        "changes_requested" => Ok(PublishWorkflowState::ChangesRequested),
        "approved" => Ok(PublishWorkflowState::Approved),
        "published" => Ok(PublishWorkflowState::Published),
        "rejected" => Ok(PublishWorkflowState::Rejected),
        _ => Err(ReviewWorkflowRepositoryError::CorruptedState),
    }
}

fn parse_review_request_status(
    value: &str,
) -> Result<ReviewRequestStatus, ReviewWorkflowRepositoryError> {
    match value {
        "review_requested" => Ok(ReviewRequestStatus::ReviewRequested),
        "approved" => Ok(ReviewRequestStatus::Approved),
        "rejected" => Ok(ReviewRequestStatus::Rejected),
        "changes_requested" => Ok(ReviewRequestStatus::ChangesRequested),
        "published" => Ok(ReviewRequestStatus::Published),
        _ => Err(ReviewWorkflowRepositoryError::CorruptedState),
    }
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> Result<String, ReviewWorkflowRepositoryError> {
    if !value.len().is_multiple_of(2) {
        return Err(ReviewWorkflowRepositoryError::CorruptedState);
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ReviewWorkflowRepositoryError::CorruptedState)?;
    String::from_utf8(bytes).map_err(|_| ReviewWorkflowRepositoryError::CorruptedState)
}
