use std::collections::BTreeSet;

use cabinet_domain::document::{DocumentId, DocumentSlug, DocumentTitle};
use cabinet_domain::link::LinkTarget;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::link_index::{LinkIndex, LinkIndexError};
use cabinet_ports::projection_work::ProjectionWorkRepository;

use crate::document::DocumentChangeEvent;
use crate::reindex_projection::{
    ReindexCurrentProjectionError, ReindexCurrentProjectionInput, ReindexCurrentProjectionUsecase,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceFanoutOutcome {
    Applied {
        affected_documents: usize,
        enqueued: usize,
        reset: usize,
        already_active: usize,
    },
    Ignored,
}

impl ReferenceFanoutOutcome {
    pub const fn affected_documents(self) -> usize {
        match self {
            Self::Applied {
                affected_documents, ..
            } => affected_documents,
            Self::Ignored => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReindexReferenceDependentsError {
    InvalidEvent,
    LinkIndexUnavailable,
    CorruptedLinkIndex,
    CurrentVersionUnavailable,
    CurrentVersionNotFound,
    WorkRepositoryUnavailable,
    WorkRepositoryCorrupted,
}

impl ReindexReferenceDependentsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidEvent => "reference_fanout.invalid_event",
            Self::LinkIndexUnavailable => "reference_fanout.link_index_unavailable",
            Self::CorruptedLinkIndex => "reference_fanout.link_index_corrupted",
            Self::CurrentVersionUnavailable => "reference_fanout.current_version_unavailable",
            Self::CurrentVersionNotFound => "reference_fanout.current_version_not_found",
            Self::WorkRepositoryUnavailable => "reference_fanout.work_repository_unavailable",
            Self::WorkRepositoryCorrupted => "reference_fanout.work_repository_corrupted",
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ReindexReferenceDependentsUsecase;

impl ReindexReferenceDependentsUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        event: &DocumentChangeEvent,
        links: &impl LinkIndex,
        pointer: &impl CurrentDocumentVersionPointerPort,
        work: &mut impl ProjectionWorkRepository,
    ) -> Result<ReferenceFanoutOutcome, ReindexReferenceDependentsError> {
        let (workspace, target, unresolved_title, include_resolved) = match event {
            DocumentChangeEvent::DocumentCreated {
                workspace_id,
                document_id,
                title,
                ..
            } => (workspace_id, document_id, Some(title.as_str()), false),
            DocumentChangeEvent::DocumentUpdated {
                workspace_id,
                document_id,
                title,
                ..
            } => (workspace_id, document_id, Some(title.as_str()), true),
            DocumentChangeEvent::DocumentRenamed {
                workspace_id,
                document_id,
                title,
                ..
            } => (workspace_id, document_id, Some(title.as_str()), true),
            DocumentChangeEvent::DocumentDeleted {
                workspace_id,
                document_id,
                ..
            } => (workspace_id, document_id, None, true),
            _ => return Ok(ReferenceFanoutOutcome::Ignored),
        };
        let workspace = WorkspaceId::new(workspace)
            .map_err(|_| ReindexReferenceDependentsError::InvalidEvent)?;
        let target =
            DocumentId::new(target).map_err(|_| ReindexReferenceDependentsError::InvalidEvent)?;
        let mut sources = BTreeSet::new();
        if include_resolved {
            for backlink in links
                .list_backlinks(&workspace, &target)
                .map_err(map_link_error)?
            {
                sources.insert(backlink.source_document_id().as_str().to_string());
            }
        }
        if let Some(title) = unresolved_title {
            let slug = DocumentSlug::from_title(
                &DocumentTitle::new(title)
                    .map_err(|_| ReindexReferenceDependentsError::InvalidEvent)?,
            )
            .map_err(|_| ReindexReferenceDependentsError::InvalidEvent)?;
            for link in links
                .list_unresolved_links(&workspace)
                .map_err(map_link_error)?
            {
                if matches!(link.target(), LinkTarget::Unresolved(target) if target == &slug) {
                    sources.insert(link.source_document_id().as_str().to_string());
                }
            }
        }

        let mut enqueued = 0;
        let mut reset = 0;
        let mut already_active = 0;
        for source in &sources {
            let output = ReindexCurrentProjectionUsecase::new()
                .execute(
                    ReindexCurrentProjectionInput::new(workspace.as_str(), source),
                    pointer,
                    work,
                )
                .map_err(map_reindex_error)?;
            enqueued += output.enqueued_count();
            reset += output.reset_count();
            already_active += output.already_active_count();
        }
        Ok(ReferenceFanoutOutcome::Applied {
            affected_documents: sources.len(),
            enqueued,
            reset,
            already_active,
        })
    }
}

const fn map_link_error(error: LinkIndexError) -> ReindexReferenceDependentsError {
    match error {
        LinkIndexError::StorageUnavailable => ReindexReferenceDependentsError::LinkIndexUnavailable,
        LinkIndexError::MismatchedSourceDocument
        | LinkIndexError::ResolvedLinkInUnresolvedProjection
        | LinkIndexError::CorruptedProjection => {
            ReindexReferenceDependentsError::CorruptedLinkIndex
        }
    }
}

const fn map_reindex_error(
    error: ReindexCurrentProjectionError,
) -> ReindexReferenceDependentsError {
    match error {
        ReindexCurrentProjectionError::InvalidInput => {
            ReindexReferenceDependentsError::InvalidEvent
        }
        ReindexCurrentProjectionError::CurrentVersionNotFound => {
            ReindexReferenceDependentsError::CurrentVersionNotFound
        }
        ReindexCurrentProjectionError::PointerUnavailable => {
            ReindexReferenceDependentsError::CurrentVersionUnavailable
        }
        ReindexCurrentProjectionError::RepositoryUnavailable
        | ReindexCurrentProjectionError::RepositoryConflict => {
            ReindexReferenceDependentsError::WorkRepositoryUnavailable
        }
        ReindexCurrentProjectionError::CorruptedState
        | ReindexCurrentProjectionError::InvalidTransition => {
            ReindexReferenceDependentsError::WorkRepositoryCorrupted
        }
    }
}
