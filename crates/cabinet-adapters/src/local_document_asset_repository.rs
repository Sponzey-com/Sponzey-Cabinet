use std::fs;
use std::path::{Path, PathBuf};

use cabinet_domain::asset::{
    AssetFileName, AssetId, AssetMediaType, AssetMetadata, AssetReference,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_asset_repository::{
    DocumentAssetAttachOutcome, DocumentAssetRecord, DocumentAssetRepository,
    DocumentAssetRepositoryError,
};

use crate::local_atomic_file::write_text_atomically;

pub const DOCUMENT_ASSETS_DIR: &str = "document-assets";
pub const DOCUMENT_ASSETS_BY_DOCUMENT_DIR: &str = "by-document";
pub const DOCUMENT_ASSETS_FILE: &str = "assets.tsv";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDocumentAssetRepository {
    association_root: PathBuf,
}

impl LocalDocumentAssetRepository {
    pub fn new(association_root: PathBuf) -> Self {
        Self { association_root }
    }

    fn document_assets_path(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> PathBuf {
        self.association_root
            .join(encode_path_segment(workspace_id.as_str()))
            .join(DOCUMENT_ASSETS_DIR)
            .join(DOCUMENT_ASSETS_BY_DOCUMENT_DIR)
            .join(format!(
                "{}.{}",
                encode_path_segment(document_id.as_str()),
                DOCUMENT_ASSETS_FILE
            ))
    }
}

impl DocumentAssetRepository for LocalDocumentAssetRepository {
    fn attach_asset(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        record: DocumentAssetRecord,
    ) -> Result<DocumentAssetAttachOutcome, DocumentAssetRepositoryError> {
        let path = self.document_assets_path(workspace_id, document_id);
        let mut records = read_records(&path)?;
        if records
            .iter()
            .any(|existing| existing.asset_id() == record.asset_id())
        {
            return Ok(DocumentAssetAttachOutcome::AlreadyAttached);
        }

        records.push(record);
        write_records(path, &records)?;
        Ok(DocumentAssetAttachOutcome::Attached)
    }

    fn list_assets(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Vec<DocumentAssetRecord>, DocumentAssetRepositoryError> {
        read_records(&self.document_assets_path(workspace_id, document_id))
    }
}

fn read_records(path: &Path) -> Result<Vec<DocumentAssetRecord>, DocumentAssetRepositoryError> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(DocumentAssetRepositoryError::StorageUnavailable),
    };

    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_record)
        .collect()
}

fn write_records(
    path: PathBuf,
    records: &[DocumentAssetRecord],
) -> Result<(), DocumentAssetRepositoryError> {
    let content = records
        .iter()
        .map(format_record)
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    write_text_atomically(&path, format!("{content}\n"))
        .map(|_| ())
        .map_err(|_| DocumentAssetRepositoryError::StorageUnavailable)
}

fn format_record(record: &DocumentAssetRecord) -> Result<String, DocumentAssetRepositoryError> {
    Ok(format!(
        "{}\t{}\t{}\t{}\t{}",
        record.asset_id().as_str(),
        encode_value(record.reference().label()),
        encode_value(record.metadata().file_name().as_str()),
        encode_value(record.metadata().media_type().as_str()),
        record.metadata().byte_size()
    ))
}

fn parse_record(line: &str) -> Result<DocumentAssetRecord, DocumentAssetRepositoryError> {
    let mut parts = line.split('\t');
    let asset_id = AssetId::from_sha256_hex(
        parts
            .next()
            .ok_or(DocumentAssetRepositoryError::CorruptedMetadata)?,
    )
    .map_err(|_| DocumentAssetRepositoryError::CorruptedMetadata)?;
    let label = decode_value(
        parts
            .next()
            .ok_or(DocumentAssetRepositoryError::CorruptedMetadata)?,
    )?;
    let file_name = decode_value(
        parts
            .next()
            .ok_or(DocumentAssetRepositoryError::CorruptedMetadata)?,
    )?;
    let media_type = decode_value(
        parts
            .next()
            .ok_or(DocumentAssetRepositoryError::CorruptedMetadata)?,
    )?;
    let byte_size = parts
        .next()
        .ok_or(DocumentAssetRepositoryError::CorruptedMetadata)?
        .parse::<u64>()
        .map_err(|_| DocumentAssetRepositoryError::CorruptedMetadata)?;
    if parts.next().is_some() {
        return Err(DocumentAssetRepositoryError::CorruptedMetadata);
    }

    let metadata = AssetMetadata::new(
        asset_id.clone(),
        AssetFileName::new(&file_name)
            .map_err(|_| DocumentAssetRepositoryError::CorruptedMetadata)?,
        AssetMediaType::new(&media_type)
            .map_err(|_| DocumentAssetRepositoryError::CorruptedMetadata)?,
        byte_size,
    )
    .map_err(|_| DocumentAssetRepositoryError::CorruptedMetadata)?;
    let reference = AssetReference::new(asset_id, &label)
        .map_err(|_| DocumentAssetRepositoryError::CorruptedMetadata)?;
    DocumentAssetRecord::new(reference, metadata)
}

fn encode_value(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn decode_value(value: &str) -> Result<String, DocumentAssetRepositoryError> {
    if !value.len().is_multiple_of(2) {
        return Err(DocumentAssetRepositoryError::CorruptedMetadata);
    }

    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| {
            let hex = std::str::from_utf8(chunk)
                .map_err(|_| DocumentAssetRepositoryError::CorruptedMetadata)?;
            u8::from_str_radix(hex, 16).map_err(|_| DocumentAssetRepositoryError::CorruptedMetadata)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| DocumentAssetRepositoryError::CorruptedMetadata)
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' {
            encoded.push(byte as char);
        } else {
            encoded.push('~');
            encoded.push_str(&format!("{byte:02x}"));
        }
    }
    encoded
}
