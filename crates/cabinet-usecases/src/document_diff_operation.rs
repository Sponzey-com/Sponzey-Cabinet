use cabinet_domain::document_diff_operation::{
    DocumentDiffOperation, DocumentDiffOperationEvent, DocumentDiffOperationId,
    DocumentDiffOperationState,
};
use cabinet_domain::document_diff_query::DocumentDiffQueryTarget;
use cabinet_ports::committed_version_record_reader::CommittedVersionRecordReader;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;

use crate::authoritative_document_diff::{
    CompareAuthoritativeDocumentRevisionsError, CompareAuthoritativeDocumentRevisionsInput,
    CompareAuthoritativeDocumentRevisionsOutput, CompareAuthoritativeDocumentRevisionsUsecase,
};
use crate::document_diff::{DiffComputation, DocumentLineDiffService};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentDiffOperationEntry {
    operation: DocumentDiffOperation,
    target: DocumentDiffQueryTarget,
    payload: DocumentDiffOperationPayload,
}

impl DocumentDiffOperationEntry {
    pub fn new(
        operation: DocumentDiffOperation,
        target: DocumentDiffQueryTarget,
    ) -> Result<Self, DocumentDiffOperationEntryError> {
        Self::with_payload(operation, target, DocumentDiffOperationPayload::Pending)
    }

    pub fn with_payload(
        operation: DocumentDiffOperation,
        target: DocumentDiffQueryTarget,
        payload: DocumentDiffOperationPayload,
    ) -> Result<Self, DocumentDiffOperationEntryError> {
        let valid = matches!(
            (operation.state(), &payload),
            (
                DocumentDiffOperationState::Accepted
                    | DocumentDiffOperationState::Running
                    | DocumentDiffOperationState::Cancelled
                    | DocumentDiffOperationState::Expired,
                DocumentDiffOperationPayload::Pending,
            ) | (
                DocumentDiffOperationState::Completed,
                DocumentDiffOperationPayload::Completed(_),
            ) | (
                DocumentDiffOperationState::Failed,
                DocumentDiffOperationPayload::Failed { .. },
            )
        );
        if !valid {
            return Err(DocumentDiffOperationEntryError::StatePayloadMismatch);
        }
        if matches!(
            &payload,
            DocumentDiffOperationPayload::Failed { error_code }
                if error_code.trim().is_empty() || error_code.chars().any(char::is_control)
        ) {
            return Err(DocumentDiffOperationEntryError::InvalidFailureCode);
        }
        Ok(Self {
            operation,
            target,
            payload,
        })
    }

    pub const fn operation(&self) -> &DocumentDiffOperation {
        &self.operation
    }

    pub const fn target(&self) -> &DocumentDiffQueryTarget {
        &self.target
    }

    pub const fn payload(&self) -> &DocumentDiffOperationPayload {
        &self.payload
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentDiffOperationPayload {
    Pending,
    Completed(CompareAuthoritativeDocumentRevisionsOutput),
    Failed { error_code: &'static str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentDiffOperationEntryError {
    StatePayloadMismatch,
    InvalidFailureCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentDiffOperationCreateOutcome {
    Created,
    AlreadyExists,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentDiffOperationRegistryError {
    CapacityExceeded,
    Conflict,
    Unavailable,
}

impl DocumentDiffOperationRegistryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::CapacityExceeded => "document_diff_operation_registry.capacity_exceeded",
            Self::Conflict => "document_diff_operation_registry.conflict",
            Self::Unavailable => "document_diff_operation_registry.unavailable",
        }
    }
}

pub trait DocumentDiffOperationRegistry {
    fn create(
        &mut self,
        entry: DocumentDiffOperationEntry,
    ) -> Result<DocumentDiffOperationCreateOutcome, DocumentDiffOperationRegistryError>;

    fn get(
        &self,
        operation_id: &DocumentDiffOperationId,
    ) -> Result<Option<DocumentDiffOperationEntry>, DocumentDiffOperationRegistryError>;

    fn replace(
        &mut self,
        entry: DocumentDiffOperationEntry,
        expected_state: DocumentDiffOperationState,
    ) -> Result<(), DocumentDiffOperationRegistryError>;
}

pub trait DocumentDiffOperationIdGenerator {
    fn next_id(&mut self) -> Result<String, ()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartDocumentDiffOperationInput {
    CurrentToVersion {
        workspace_id: String,
        document_id: String,
        version_id: String,
    },
    Versions {
        workspace_id: String,
        document_id: String,
        left_version_id: String,
        right_version_id: String,
    },
}

impl StartDocumentDiffOperationInput {
    pub fn current_to_version(workspace_id: &str, document_id: &str, version_id: &str) -> Self {
        Self::CurrentToVersion {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            version_id: version_id.to_string(),
        }
    }

    pub fn versions(
        workspace_id: &str,
        document_id: &str,
        left_version_id: &str,
        right_version_id: &str,
    ) -> Self {
        Self::Versions {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            left_version_id: left_version_id.to_string(),
            right_version_id: right_version_id.to_string(),
        }
    }

    fn into_target(self) -> Result<DocumentDiffQueryTarget, StartDocumentDiffOperationError> {
        match self {
            Self::CurrentToVersion {
                workspace_id,
                document_id,
                version_id,
            } => DocumentDiffQueryTarget::current_to_version(
                &workspace_id,
                &document_id,
                &version_id,
            ),
            Self::Versions {
                workspace_id,
                document_id,
                left_version_id,
                right_version_id,
            } => DocumentDiffQueryTarget::versions(
                &workspace_id,
                &document_id,
                &left_version_id,
                &right_version_id,
            ),
        }
        .map_err(|_| StartDocumentDiffOperationError::InvalidInput)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartDocumentDiffOperationOutput {
    operation_id: DocumentDiffOperationId,
    state: DocumentDiffOperationState,
    product_log_event: &'static str,
}

impl StartDocumentDiffOperationOutput {
    pub const fn operation_id(&self) -> &DocumentDiffOperationId {
        &self.operation_id
    }

    pub const fn state(&self) -> DocumentDiffOperationState {
        self.state
    }

    pub const fn product_log_event(&self) -> &'static str {
        self.product_log_event
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartDocumentDiffOperationUsecase;

impl StartDocumentDiffOperationUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute<G: DocumentDiffOperationIdGenerator, R: DocumentDiffOperationRegistry>(
        &self,
        input: StartDocumentDiffOperationInput,
        ids: &mut G,
        registry: &mut R,
    ) -> Result<StartDocumentDiffOperationOutput, StartDocumentDiffOperationError> {
        let target = input.into_target()?;
        let raw_operation_id = ids
            .next_id()
            .map_err(|_| StartDocumentDiffOperationError::OperationIdUnavailable)?;
        let operation_id = DocumentDiffOperationId::new(&raw_operation_id)
            .map_err(|_| StartDocumentDiffOperationError::OperationIdUnavailable)?;
        let operation = DocumentDiffOperation::accepted(operation_id.clone());
        let entry = DocumentDiffOperationEntry::new(operation, target)
            .map_err(|_| StartDocumentDiffOperationError::InvalidInput)?;

        match registry.create(entry).map_err(map_start_registry_error)? {
            DocumentDiffOperationCreateOutcome::Created => Ok(StartDocumentDiffOperationOutput {
                operation_id,
                state: DocumentDiffOperationState::Accepted,
                product_log_event: DocumentDiffOperationState::Accepted.product_log_event(),
            }),
            DocumentDiffOperationCreateOutcome::AlreadyExists => {
                Err(StartDocumentDiffOperationError::AlreadyExists)
            }
        }
    }
}

impl Default for StartDocumentDiffOperationUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartDocumentDiffOperationError {
    InvalidInput,
    OperationIdUnavailable,
    AlreadyExists,
    CapacityExceeded,
    RegistryUnavailable,
}

impl StartDocumentDiffOperationError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "document_diff_operation.invalid_input",
            Self::OperationIdUnavailable => "document_diff_operation.id_unavailable",
            Self::AlreadyExists => "document_diff_operation.already_exists",
            Self::CapacityExceeded => "document_diff_operation.capacity_exceeded",
            Self::RegistryUnavailable => "document_diff_operation.registry_unavailable",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(
            self,
            Self::OperationIdUnavailable | Self::RegistryUnavailable
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetDocumentDiffOperationStatusInput {
    operation_id: String,
}

impl GetDocumentDiffOperationStatusInput {
    pub fn new(operation_id: &str) -> Self {
        Self {
            operation_id: operation_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetDocumentDiffOperationStatusOutput {
    operation_id: DocumentDiffOperationId,
    state: DocumentDiffOperationState,
    target: Option<DocumentDiffQueryTarget>,
    product_log_event: Option<&'static str>,
    result: Option<CompareAuthoritativeDocumentRevisionsOutput>,
    failure_code: Option<&'static str>,
}

impl GetDocumentDiffOperationStatusOutput {
    pub const fn operation_id(&self) -> &DocumentDiffOperationId {
        &self.operation_id
    }

    pub const fn state(&self) -> DocumentDiffOperationState {
        self.state
    }

    pub const fn target(&self) -> Option<&DocumentDiffQueryTarget> {
        self.target.as_ref()
    }

    pub const fn product_log_event(&self) -> Option<&'static str> {
        self.product_log_event
    }

    pub const fn result(&self) -> Option<&CompareAuthoritativeDocumentRevisionsOutput> {
        self.result.as_ref()
    }

    pub const fn failure_code(&self) -> Option<&'static str> {
        self.failure_code
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetDocumentDiffOperationStatusUsecase;

impl GetDocumentDiffOperationStatusUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute<R: DocumentDiffOperationRegistry>(
        &self,
        input: GetDocumentDiffOperationStatusInput,
        registry: &R,
    ) -> Result<GetDocumentDiffOperationStatusOutput, GetDocumentDiffOperationStatusError> {
        let operation_id = DocumentDiffOperationId::new(&input.operation_id)
            .map_err(|_| GetDocumentDiffOperationStatusError::InvalidInput)?;
        let entry = registry
            .get(&operation_id)
            .map_err(map_status_registry_error)?;
        Ok(match entry {
            Some(entry) => status_from_entry(operation_id, &entry),
            None => expired_status(operation_id),
        })
    }
}

impl Default for GetDocumentDiffOperationStatusUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetDocumentDiffOperationStatusError {
    InvalidInput,
    RegistryUnavailable,
}

impl GetDocumentDiffOperationStatusError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "document_diff_operation.invalid_input",
            Self::RegistryUnavailable => "document_diff_operation.registry_unavailable",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(self, Self::RegistryUnavailable)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelDocumentDiffOperationInput {
    operation_id: String,
}

impl CancelDocumentDiffOperationInput {
    pub fn new(operation_id: &str) -> Self {
        Self {
            operation_id: operation_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelDocumentDiffOperationOutput {
    operation_id: DocumentDiffOperationId,
    state: DocumentDiffOperationState,
    side_effect: Option<cabinet_domain::document_diff_operation::DocumentDiffOperationSideEffect>,
    product_log_event: Option<&'static str>,
}

impl CancelDocumentDiffOperationOutput {
    pub const fn operation_id(&self) -> &DocumentDiffOperationId {
        &self.operation_id
    }

    pub const fn state(&self) -> DocumentDiffOperationState {
        self.state
    }

    pub const fn side_effect(
        &self,
    ) -> Option<cabinet_domain::document_diff_operation::DocumentDiffOperationSideEffect> {
        self.side_effect
    }

    pub const fn product_log_event(&self) -> Option<&'static str> {
        self.product_log_event
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CancelDocumentDiffOperationUsecase;

impl CancelDocumentDiffOperationUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute<R: DocumentDiffOperationRegistry>(
        &self,
        input: CancelDocumentDiffOperationInput,
        registry: &mut R,
    ) -> Result<CancelDocumentDiffOperationOutput, CancelDocumentDiffOperationError> {
        let operation_id = DocumentDiffOperationId::new(&input.operation_id)
            .map_err(|_| CancelDocumentDiffOperationError::InvalidInput)?;
        let Some(entry) = registry
            .get(&operation_id)
            .map_err(map_cancel_registry_error)?
        else {
            return Ok(expired_cancel(operation_id, true));
        };

        let current_state = entry.operation().state();
        match current_state {
            DocumentDiffOperationState::Cancelled => Ok(CancelDocumentDiffOperationOutput {
                operation_id,
                state: DocumentDiffOperationState::Cancelled,
                side_effect: None,
                product_log_event: None,
            }),
            DocumentDiffOperationState::Expired => Ok(expired_cancel(operation_id, false)),
            DocumentDiffOperationState::Completed | DocumentDiffOperationState::Failed => {
                Err(CancelDocumentDiffOperationError::CancellationTooLate)
            }
            DocumentDiffOperationState::Accepted | DocumentDiffOperationState::Running => {
                let transition = entry
                    .operation()
                    .transition(
                        cabinet_domain::document_diff_operation::DocumentDiffOperationEvent::Cancel,
                    )
                    .map_err(|_| CancelDocumentDiffOperationError::InvalidTransition)?;
                let side_effect = transition.side_effect();
                let product_log_event = transition.product_log_event();
                let next_entry = DocumentDiffOperationEntry::new(
                    transition.into_operation(),
                    entry.target().clone(),
                )
                .map_err(|_| CancelDocumentDiffOperationError::InvalidTransition)?;
                registry
                    .replace(next_entry, current_state)
                    .map_err(map_cancel_registry_error)?;
                Ok(CancelDocumentDiffOperationOutput {
                    operation_id,
                    state: DocumentDiffOperationState::Cancelled,
                    side_effect,
                    product_log_event: Some(product_log_event),
                })
            }
        }
    }
}

impl Default for CancelDocumentDiffOperationUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelDocumentDiffOperationError {
    InvalidInput,
    InvalidTransition,
    CancellationTooLate,
    Conflict,
    RegistryUnavailable,
}

impl CancelDocumentDiffOperationError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "document_diff_operation.invalid_input",
            Self::InvalidTransition => "document_diff_operation.invalid_transition",
            Self::CancellationTooLate => "document_diff_operation.cancellation_too_late",
            Self::Conflict => "document_diff_operation.conflict",
            Self::RegistryUnavailable => "document_diff_operation.registry_unavailable",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(self, Self::Conflict | Self::RegistryUnavailable)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunDocumentDiffOperationInput {
    operation_id: String,
}

impl RunDocumentDiffOperationInput {
    pub fn new(operation_id: &str) -> Self {
        Self {
            operation_id: operation_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunDocumentDiffOperationOutput {
    operation_id: DocumentDiffOperationId,
    state: DocumentDiffOperationState,
    product_log_event: Option<&'static str>,
    failure_code: Option<&'static str>,
}

impl RunDocumentDiffOperationOutput {
    pub const fn operation_id(&self) -> &DocumentDiffOperationId {
        &self.operation_id
    }

    pub const fn state(&self) -> DocumentDiffOperationState {
        self.state
    }

    pub const fn product_log_event(&self) -> Option<&'static str> {
        self.product_log_event
    }

    pub const fn failure_code(&self) -> Option<&'static str> {
        self.failure_code
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunDocumentDiffOperationUsecase {
    executor: CompareAuthoritativeDocumentRevisionsUsecase,
}

impl RunDocumentDiffOperationUsecase {
    pub const fn with_diff_service(diff_service: DocumentLineDiffService) -> Self {
        Self {
            executor: CompareAuthoritativeDocumentRevisionsUsecase::with_policy(
                diff_service.policy(),
            ),
        }
    }

    pub fn execute<R: DocumentDiffOperationRegistry>(
        &self,
        input: RunDocumentDiffOperationInput,
        registry: &mut R,
        pointer: &impl CurrentDocumentVersionPointerPort,
        versions: &impl CommittedVersionRecordReader,
    ) -> Result<RunDocumentDiffOperationOutput, RunDocumentDiffOperationError> {
        let operation_id = DocumentDiffOperationId::new(&input.operation_id)
            .map_err(|_| RunDocumentDiffOperationError::InvalidInput)?;
        let Some(entry) = registry
            .get(&operation_id)
            .map_err(map_run_registry_error)?
        else {
            return Ok(run_terminal_output(
                operation_id,
                DocumentDiffOperationState::Expired,
                None,
            ));
        };

        match entry.operation().state() {
            DocumentDiffOperationState::Accepted => {}
            DocumentDiffOperationState::Running => {
                return Err(RunDocumentDiffOperationError::AlreadyRunning);
            }
            DocumentDiffOperationState::Completed
            | DocumentDiffOperationState::Cancelled
            | DocumentDiffOperationState::Expired
            | DocumentDiffOperationState::Failed => {
                let failure_code = match entry.payload() {
                    DocumentDiffOperationPayload::Failed { error_code } => Some(*error_code),
                    _ => None,
                };
                return Ok(run_terminal_output(
                    operation_id,
                    entry.operation().state(),
                    failure_code,
                ));
            }
        }

        let running_transition = entry
            .operation()
            .transition(DocumentDiffOperationEvent::Start)
            .map_err(|_| RunDocumentDiffOperationError::InvalidState)?;
        let running_operation = running_transition.into_operation();
        let running_entry =
            DocumentDiffOperationEntry::new(running_operation.clone(), entry.target().clone())
                .map_err(|_| RunDocumentDiffOperationError::InvalidState)?;
        registry
            .replace(running_entry, DocumentDiffOperationState::Accepted)
            .map_err(map_run_registry_error)?;

        let compare_input = authoritative_compare_input(entry.target());
        match self.executor.execute(compare_input, pointer, versions) {
            Ok(result) if matches!(result.computation(), DiffComputation::Complete(_)) => {
                let transition = running_operation
                    .transition(DocumentDiffOperationEvent::Complete)
                    .map_err(|_| RunDocumentDiffOperationError::InvalidState)?;
                let product_log_event = transition.product_log_event();
                let completed_entry = DocumentDiffOperationEntry::with_payload(
                    transition.into_operation(),
                    entry.target().clone(),
                    DocumentDiffOperationPayload::Completed(result),
                )
                .map_err(|_| RunDocumentDiffOperationError::InvalidState)?;
                registry
                    .replace(completed_entry, DocumentDiffOperationState::Running)
                    .map_err(map_run_registry_error)?;
                Ok(RunDocumentDiffOperationOutput {
                    operation_id,
                    state: DocumentDiffOperationState::Completed,
                    product_log_event: Some(product_log_event),
                    failure_code: None,
                })
            }
            Ok(result) if matches!(result.computation(), DiffComputation::TooLarge(_)) => {
                fail_running_operation(
                    operation_id,
                    running_operation,
                    entry.target(),
                    "document.diff.background_limit_exceeded",
                    registry,
                )
            }
            Err(error) => fail_running_operation(
                operation_id,
                running_operation,
                entry.target(),
                executor_failure_code(error),
                registry,
            ),
            Ok(_) => unreachable!("diff computation variants are exhaustively guarded"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunDocumentDiffOperationError {
    InvalidInput,
    InvalidState,
    AlreadyRunning,
    Conflict,
    RegistryUnavailable,
}

impl RunDocumentDiffOperationError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "document_diff_operation.invalid_input",
            Self::InvalidState => "document_diff_operation.invalid_state",
            Self::AlreadyRunning => "document_diff_operation.already_running",
            Self::Conflict => "document_diff_operation.conflict",
            Self::RegistryUnavailable => "document_diff_operation.registry_unavailable",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(self, Self::Conflict | Self::RegistryUnavailable)
    }
}

fn fail_running_operation<R: DocumentDiffOperationRegistry>(
    operation_id: DocumentDiffOperationId,
    running_operation: DocumentDiffOperation,
    target: &DocumentDiffQueryTarget,
    failure_code: &'static str,
    registry: &mut R,
) -> Result<RunDocumentDiffOperationOutput, RunDocumentDiffOperationError> {
    let transition = running_operation
        .transition(DocumentDiffOperationEvent::Fail)
        .map_err(|_| RunDocumentDiffOperationError::InvalidState)?;
    let product_log_event = transition.product_log_event();
    let failed_entry = DocumentDiffOperationEntry::with_payload(
        transition.into_operation(),
        target.clone(),
        DocumentDiffOperationPayload::Failed {
            error_code: failure_code,
        },
    )
    .map_err(|_| RunDocumentDiffOperationError::InvalidState)?;
    registry
        .replace(failed_entry, DocumentDiffOperationState::Running)
        .map_err(map_run_registry_error)?;
    Ok(RunDocumentDiffOperationOutput {
        operation_id,
        state: DocumentDiffOperationState::Failed,
        product_log_event: Some(product_log_event),
        failure_code: Some(failure_code),
    })
}

fn run_terminal_output(
    operation_id: DocumentDiffOperationId,
    state: DocumentDiffOperationState,
    failure_code: Option<&'static str>,
) -> RunDocumentDiffOperationOutput {
    RunDocumentDiffOperationOutput {
        operation_id,
        state,
        product_log_event: None,
        failure_code,
    }
}

fn executor_failure_code(error: CompareAuthoritativeDocumentRevisionsError) -> &'static str {
    error.code()
}

fn authoritative_compare_input(
    target: &DocumentDiffQueryTarget,
) -> CompareAuthoritativeDocumentRevisionsInput {
    if let Some(version_id) = target.current_version_id() {
        return CompareAuthoritativeDocumentRevisionsInput::current_to_version(
            target.workspace_id().as_str(),
            target.document_id().as_str(),
            version_id.as_str(),
        );
    }
    let (left_version_id, right_version_id) = target
        .version_pair()
        .expect("validated diff target must have one query kind");
    CompareAuthoritativeDocumentRevisionsInput::versions(
        target.workspace_id().as_str(),
        target.document_id().as_str(),
        left_version_id.as_str(),
        right_version_id.as_str(),
    )
}

fn map_run_registry_error(
    error: DocumentDiffOperationRegistryError,
) -> RunDocumentDiffOperationError {
    match error {
        DocumentDiffOperationRegistryError::CapacityExceeded => {
            RunDocumentDiffOperationError::RegistryUnavailable
        }
        DocumentDiffOperationRegistryError::Conflict => RunDocumentDiffOperationError::Conflict,
        DocumentDiffOperationRegistryError::Unavailable => {
            RunDocumentDiffOperationError::RegistryUnavailable
        }
    }
}

fn expired_status(operation_id: DocumentDiffOperationId) -> GetDocumentDiffOperationStatusOutput {
    GetDocumentDiffOperationStatusOutput {
        operation_id,
        state: DocumentDiffOperationState::Expired,
        target: None,
        product_log_event: Some(DocumentDiffOperationState::Expired.product_log_event()),
        result: None,
        failure_code: None,
    }
}

fn status_from_entry(
    operation_id: DocumentDiffOperationId,
    entry: &DocumentDiffOperationEntry,
) -> GetDocumentDiffOperationStatusOutput {
    let (result, failure_code) = match entry.payload() {
        DocumentDiffOperationPayload::Pending => (None, None),
        DocumentDiffOperationPayload::Completed(result) => (Some(result.clone()), None),
        DocumentDiffOperationPayload::Failed { error_code } => (None, Some(*error_code)),
    };
    GetDocumentDiffOperationStatusOutput {
        operation_id,
        state: entry.operation().state(),
        target: Some(entry.target().clone()),
        product_log_event: None,
        result,
        failure_code,
    }
}

fn expired_cancel(
    operation_id: DocumentDiffOperationId,
    newly_expired: bool,
) -> CancelDocumentDiffOperationOutput {
    CancelDocumentDiffOperationOutput {
        operation_id,
        state: DocumentDiffOperationState::Expired,
        side_effect: None,
        product_log_event: newly_expired
            .then_some(DocumentDiffOperationState::Expired.product_log_event()),
    }
}

fn map_status_registry_error(
    _error: DocumentDiffOperationRegistryError,
) -> GetDocumentDiffOperationStatusError {
    GetDocumentDiffOperationStatusError::RegistryUnavailable
}

fn map_cancel_registry_error(
    error: DocumentDiffOperationRegistryError,
) -> CancelDocumentDiffOperationError {
    match error {
        DocumentDiffOperationRegistryError::CapacityExceeded => {
            CancelDocumentDiffOperationError::RegistryUnavailable
        }
        DocumentDiffOperationRegistryError::Conflict => CancelDocumentDiffOperationError::Conflict,
        DocumentDiffOperationRegistryError::Unavailable => {
            CancelDocumentDiffOperationError::RegistryUnavailable
        }
    }
}

fn map_start_registry_error(
    error: DocumentDiffOperationRegistryError,
) -> StartDocumentDiffOperationError {
    match error {
        DocumentDiffOperationRegistryError::CapacityExceeded => {
            StartDocumentDiffOperationError::CapacityExceeded
        }
        DocumentDiffOperationRegistryError::Conflict
        | DocumentDiffOperationRegistryError::Unavailable => {
            StartDocumentDiffOperationError::RegistryUnavailable
        }
    }
}
