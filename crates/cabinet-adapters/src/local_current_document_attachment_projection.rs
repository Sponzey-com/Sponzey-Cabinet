use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use cabinet_domain::asset::{AssetAssociation, AssetId, AssetReference};
use cabinet_ports::asset_association_catalog::{
    AssetAssociationCatalog, AssetAssociationCatalogError,
};
use cabinet_ports::current_document_attachment_projection::{
    CurrentDocumentAttachmentProjectionError, CurrentDocumentAttachmentProjectionOutcome,
    CurrentDocumentAttachmentProjectionRequest, CurrentDocumentAttachmentProjectionWriter,
};
use serde::{Deserialize, Serialize};

use crate::durable_asset_association_catalog::DurableAssetAssociationCatalog;
use crate::local_atomic_file::write_text_atomically;

pub const LOCAL_CURRENT_DOCUMENT_ATTACHMENT_PROJECTION_ROOT: &str =
    "current-document-attachment-projections";
pub const LOCAL_CURRENT_DOCUMENT_ATTACHMENT_PROJECTION_MARKER_FILE: &str = "current.json";

const PROJECTION_SCHEMA_VERSION: u32 = 1;
const MAX_DOCUMENT_ATTACHMENTS: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MarkerState {
    Applying,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectionMarker {
    schema_version: u32,
    state: MarkerState,
    revision_number: u64,
    references: Vec<ProjectionReference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectionReference {
    asset_id: String,
    label: String,
}

pub struct LocalCurrentDocumentAttachmentProjection {
    root: PathBuf,
    catalog: DurableAssetAssociationCatalog,
}

impl LocalCurrentDocumentAttachmentProjection {
    pub fn new(app_data_root: PathBuf) -> Self {
        Self {
            catalog: DurableAssetAssociationCatalog::new(app_data_root.clone()),
            root: app_data_root.join(LOCAL_CURRENT_DOCUMENT_ATTACHMENT_PROJECTION_ROOT),
        }
    }

    fn marker_path(&self, request: &CurrentDocumentAttachmentProjectionRequest) -> PathBuf {
        self.root
            .join(hex(request.workspace_id().as_str()))
            .join(hex(request.document_id().as_str()))
            .join(LOCAL_CURRENT_DOCUMENT_ATTACHMENT_PROJECTION_MARKER_FILE)
    }

    fn catalog_matches(
        &self,
        request: &CurrentDocumentAttachmentProjectionRequest,
    ) -> Result<bool, CurrentDocumentAttachmentProjectionError> {
        let current = self
            .catalog
            .list_assets(
                request.workspace_id(),
                request.document_id(),
                MAX_DOCUMENT_ATTACHMENTS,
            )
            .map_err(map_catalog_error)?;
        Ok(associations_match_references(
            &current,
            request.references(),
        ))
    }

    fn reconcile(
        &mut self,
        request: &CurrentDocumentAttachmentProjectionRequest,
    ) -> Result<(), CurrentDocumentAttachmentProjectionError> {
        let current = self
            .catalog
            .list_assets(
                request.workspace_id(),
                request.document_id(),
                MAX_DOCUMENT_ATTACHMENTS,
            )
            .map_err(map_catalog_error)?;
        for association in current {
            let desired = request
                .references()
                .iter()
                .find(|reference| reference.asset_id() == association.asset_id());
            if desired.is_none_or(|reference| reference.label() != association.label()) {
                self.catalog
                    .unlink(
                        request.workspace_id(),
                        association.asset_id(),
                        request.document_id(),
                    )
                    .map_err(map_catalog_error)?;
            }
        }
        for reference in request.references() {
            let association = AssetAssociation::new(
                reference.asset_id().clone(),
                request.document_id().clone(),
                reference.label(),
            )
            .map_err(|_| CurrentDocumentAttachmentProjectionError::InvalidRequest)?;
            self.catalog
                .link(request.workspace_id(), association)
                .map_err(map_catalog_error)?;
        }
        Ok(())
    }
}

impl CurrentDocumentAttachmentProjectionWriter for LocalCurrentDocumentAttachmentProjection {
    fn replace_current_document_attachments(
        &mut self,
        request: CurrentDocumentAttachmentProjectionRequest,
    ) -> Result<CurrentDocumentAttachmentProjectionOutcome, CurrentDocumentAttachmentProjectionError>
    {
        if request.references().len() > MAX_DOCUMENT_ATTACHMENTS {
            return Err(CurrentDocumentAttachmentProjectionError::InvalidRequest);
        }
        let marker_path = self.marker_path(&request);
        let existing = read_marker(&marker_path)?;
        if let Some(marker) = existing.as_ref() {
            validate_marker(marker)?;
            if marker.revision_number > request.revision_number().value() {
                return Err(CurrentDocumentAttachmentProjectionError::Conflict);
            }
            if marker.revision_number == request.revision_number().value() {
                if marker_references(marker)? != request.references() {
                    return Err(CurrentDocumentAttachmentProjectionError::Conflict);
                }
                if marker.state == MarkerState::Ready && self.catalog_matches(&request)? {
                    return Ok(CurrentDocumentAttachmentProjectionOutcome::AlreadyCurrent);
                }
            }
        }

        write_marker(&marker_path, marker(&request, MarkerState::Applying))?;
        self.reconcile(&request)?;
        write_marker(&marker_path, marker(&request, MarkerState::Ready))?;
        Ok(CurrentDocumentAttachmentProjectionOutcome::Applied)
    }
}

fn marker(
    request: &CurrentDocumentAttachmentProjectionRequest,
    state: MarkerState,
) -> ProjectionMarker {
    ProjectionMarker {
        schema_version: PROJECTION_SCHEMA_VERSION,
        state,
        revision_number: request.revision_number().value(),
        references: request
            .references()
            .iter()
            .map(|reference| ProjectionReference {
                asset_id: reference.asset_id().as_str().to_string(),
                label: reference.label().to_string(),
            })
            .collect(),
    }
}

fn read_marker(
    path: &Path,
) -> Result<Option<ProjectionMarker>, CurrentDocumentAttachmentProjectionError> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(CurrentDocumentAttachmentProjectionError::StorageUnavailable),
    };
    serde_json::from_str(&content)
        .map(Some)
        .map_err(|_| CurrentDocumentAttachmentProjectionError::CorruptedProjection)
}

fn write_marker(
    path: &Path,
    marker: ProjectionMarker,
) -> Result<(), CurrentDocumentAttachmentProjectionError> {
    let content = serde_json::to_string(&marker)
        .map_err(|_| CurrentDocumentAttachmentProjectionError::CorruptedProjection)?;
    write_text_atomically(path, content)
        .map(|_| ())
        .map_err(|_| CurrentDocumentAttachmentProjectionError::StorageUnavailable)
}

fn validate_marker(
    marker: &ProjectionMarker,
) -> Result<(), CurrentDocumentAttachmentProjectionError> {
    if marker.schema_version != PROJECTION_SCHEMA_VERSION || marker.revision_number == 0 {
        return Err(CurrentDocumentAttachmentProjectionError::CorruptedProjection);
    }
    let references = marker_references(marker)?;
    if references.len() > MAX_DOCUMENT_ATTACHMENTS {
        return Err(CurrentDocumentAttachmentProjectionError::CorruptedProjection);
    }
    Ok(())
}

fn marker_references(
    marker: &ProjectionMarker,
) -> Result<Vec<AssetReference>, CurrentDocumentAttachmentProjectionError> {
    let references = marker
        .references
        .iter()
        .map(|reference| {
            AssetReference::new(
                AssetId::from_sha256_hex(&reference.asset_id)
                    .map_err(|_| CurrentDocumentAttachmentProjectionError::CorruptedProjection)?,
                &reference.label,
            )
            .map_err(|_| CurrentDocumentAttachmentProjectionError::CorruptedProjection)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if references
        .windows(2)
        .any(|pair| pair[0].asset_id().as_str() >= pair[1].asset_id().as_str())
    {
        return Err(CurrentDocumentAttachmentProjectionError::CorruptedProjection);
    }
    Ok(references)
}

fn associations_match_references(
    associations: &[AssetAssociation],
    references: &[AssetReference],
) -> bool {
    associations.len() == references.len()
        && associations
            .iter()
            .zip(references)
            .all(|(association, reference)| {
                association.asset_id() == reference.asset_id()
                    && association.label() == reference.label()
            })
}

const fn map_catalog_error(
    error: AssetAssociationCatalogError,
) -> CurrentDocumentAttachmentProjectionError {
    match error {
        AssetAssociationCatalogError::InvalidLimit => {
            CurrentDocumentAttachmentProjectionError::InvalidRequest
        }
        AssetAssociationCatalogError::Conflict => {
            CurrentDocumentAttachmentProjectionError::Conflict
        }
        AssetAssociationCatalogError::StorageUnavailable => {
            CurrentDocumentAttachmentProjectionError::StorageUnavailable
        }
        AssetAssociationCatalogError::CorruptedRecord
        | AssetAssociationCatalogError::UnsupportedSchema => {
            CurrentDocumentAttachmentProjectionError::CorruptedProjection
        }
    }
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
