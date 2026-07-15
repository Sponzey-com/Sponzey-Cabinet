use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::local_atomic_file::write_text_atomically;
use cabinet_domain::asset::{
    AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId, AssetMediaType,
    AssetMetadata, AssetPreviewCapability,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_metadata_catalog::{
    AssetMetadataCatalog, AssetMetadataCatalogError, AssetMetadataPage, AssetMetadataPutOutcome,
};

const SCHEMA: &str = "schema\t1";
#[derive(Debug, Clone)]
pub struct DurableAssetMetadataCatalog {
    root: PathBuf,
}
impl DurableAssetMetadataCatalog {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    fn workspace_root(&self, workspace: &WorkspaceId) -> PathBuf {
        self.root
            .join("assets/metadata")
            .join(hex(workspace.as_str()))
    }
    fn path(&self, workspace: &WorkspaceId, id: &AssetId) -> PathBuf {
        self.workspace_root(workspace)
            .join(format!("{}.asset", id.as_str()))
    }
}
impl AssetMetadataCatalog for DurableAssetMetadataCatalog {
    fn put(
        &mut self,
        workspace: &WorkspaceId,
        record: AssetCatalogRecord,
    ) -> Result<AssetMetadataPutOutcome, AssetMetadataCatalogError> {
        let path = self.path(workspace, record.metadata().id());
        if path.exists() {
            let current = read(&path)?;
            if current != record {
                return Err(AssetMetadataCatalogError::Conflict);
            }
            return Ok(AssetMetadataPutOutcome::AlreadyPresent);
        }
        write_text_atomically(&path, encode(&record))
            .map_err(|_| AssetMetadataCatalogError::StorageUnavailable)?;
        Ok(AssetMetadataPutOutcome::Created)
    }
    fn get(
        &self,
        workspace: &WorkspaceId,
        id: &AssetId,
    ) -> Result<Option<AssetCatalogRecord>, AssetMetadataCatalogError> {
        match fs::read_to_string(self.path(workspace, id)) {
            Ok(text) => decode(&text).map(Some),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(_) => Err(AssetMetadataCatalogError::StorageUnavailable),
        }
    }
    fn list(
        &self,
        workspace: &WorkspaceId,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<AssetMetadataPage, AssetMetadataCatalogError> {
        if limit == 0 || limit > 500 {
            return Err(AssetMetadataCatalogError::InvalidLimit);
        }
        let cursor = cursor
            .map(|value| {
                AssetId::from_sha256_hex(value)
                    .map_err(|_| AssetMetadataCatalogError::InvalidCursor)
            })
            .transpose()?;
        let entries = match fs::read_dir(self.workspace_root(workspace)) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(AssetMetadataPage::new(Vec::new(), None));
            }
            Err(_) => return Err(AssetMetadataCatalogError::StorageUnavailable),
        };
        let mut paths = entries
            .map(|entry| {
                entry
                    .map(|value| value.path())
                    .map_err(|_| AssetMetadataCatalogError::StorageUnavailable)
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        let mut records = Vec::new();
        for path in paths {
            if path.extension().and_then(|value| value.to_str()) != Some("asset") {
                continue;
            }
            let record = read(&path)?;
            if cursor
                .as_ref()
                .is_some_and(|cursor| record.metadata().id().as_str() <= cursor.as_str())
            {
                continue;
            }
            records.push(record);
            if records.len() > limit {
                break;
            }
        }
        let has_more = records.len() > limit;
        records.truncate(limit);
        let next = has_more.then(|| {
            records
                .last()
                .expect("nonempty page")
                .metadata()
                .id()
                .as_str()
                .to_string()
        });
        Ok(AssetMetadataPage::new(records, next))
    }
}

fn read(path: &Path) -> Result<AssetCatalogRecord, AssetMetadataCatalogError> {
    fs::read_to_string(path)
        .map_err(|_| AssetMetadataCatalogError::StorageUnavailable)
        .and_then(|text| decode(&text))
}
fn encode(record: &AssetCatalogRecord) -> String {
    let m = record.metadata();
    let payload = format!(
        "id\t{}\nname\t{}\nmedia\t{}\nsize\t{}\nversion\t{}\npreview\t{}\nextraction\t{}\n",
        m.id().as_str(),
        hex(m.file_name().as_str()),
        hex(m.media_type().as_str()),
        m.byte_size(),
        record.version(),
        preview(record.preview()),
        extraction(record.extraction())
    );
    format!(
        "{SCHEMA}\nchecksum\t{:016x}\n{payload}",
        checksum(payload.as_bytes())
    )
}
fn decode(text: &str) -> Result<AssetCatalogRecord, AssetMetadataCatalogError> {
    let mut lines = text.lines();
    match lines.next() {
        Some(SCHEMA) => {}
        Some(line) if line.starts_with("schema\t") => {
            return Err(AssetMetadataCatalogError::UnsupportedSchema);
        }
        _ => return Err(AssetMetadataCatalogError::CorruptedRecord),
    }
    let expected = lines
        .next()
        .and_then(|line| line.strip_prefix("checksum\t"))
        .and_then(|value| u64::from_str_radix(value, 16).ok())
        .ok_or(AssetMetadataCatalogError::CorruptedRecord)?;
    let payload = format!("{}\n", lines.collect::<Vec<_>>().join("\n"));
    if checksum(payload.as_bytes()) != expected {
        return Err(AssetMetadataCatalogError::CorruptedRecord);
    }
    let fields = payload
        .lines()
        .map(|line| line.split_once('\t'))
        .collect::<Option<Vec<_>>>()
        .ok_or(AssetMetadataCatalogError::CorruptedRecord)?;
    let find = |key: &str| {
        fields
            .iter()
            .find_map(|(name, value)| (*name == key).then_some(*value))
            .ok_or(AssetMetadataCatalogError::CorruptedRecord)
    };
    let metadata = AssetMetadata::new(
        AssetId::from_sha256_hex(find("id")?)
            .map_err(|_| AssetMetadataCatalogError::CorruptedRecord)?,
        AssetFileName::new(&unhex(find("name")?)?)
            .map_err(|_| AssetMetadataCatalogError::CorruptedRecord)?,
        AssetMediaType::new(&unhex(find("media")?)?)
            .map_err(|_| AssetMetadataCatalogError::CorruptedRecord)?,
        find("size")?
            .parse()
            .map_err(|_| AssetMetadataCatalogError::CorruptedRecord)?,
    )
    .map_err(|_| AssetMetadataCatalogError::CorruptedRecord)?;
    AssetCatalogRecord::new(
        metadata,
        find("version")?
            .parse()
            .map_err(|_| AssetMetadataCatalogError::CorruptedRecord)?,
        parse_preview(find("preview")?)?,
        parse_extraction(find("extraction")?)?,
    )
    .map_err(|_| AssetMetadataCatalogError::CorruptedRecord)
}
const fn preview(value: AssetPreviewCapability) -> &'static str {
    match value {
        AssetPreviewCapability::Image => "image",
        AssetPreviewCapability::Pdf => "pdf",
        AssetPreviewCapability::Text => "text",
        AssetPreviewCapability::Unsupported => "unsupported",
    }
}
fn parse_preview(value: &str) -> Result<AssetPreviewCapability, AssetMetadataCatalogError> {
    match value {
        "image" => Ok(AssetPreviewCapability::Image),
        "pdf" => Ok(AssetPreviewCapability::Pdf),
        "text" => Ok(AssetPreviewCapability::Text),
        "unsupported" => Ok(AssetPreviewCapability::Unsupported),
        _ => Err(AssetMetadataCatalogError::CorruptedRecord),
    }
}
const fn extraction(value: AssetExtractionStatus) -> &'static str {
    match value {
        AssetExtractionStatus::NotRequested => "not_requested",
        AssetExtractionStatus::Pending => "pending",
        AssetExtractionStatus::Ready => "ready",
        AssetExtractionStatus::Unsupported => "unsupported",
        AssetExtractionStatus::Failed => "failed",
    }
}
fn parse_extraction(value: &str) -> Result<AssetExtractionStatus, AssetMetadataCatalogError> {
    match value {
        "not_requested" => Ok(AssetExtractionStatus::NotRequested),
        "pending" => Ok(AssetExtractionStatus::Pending),
        "ready" => Ok(AssetExtractionStatus::Ready),
        "unsupported" => Ok(AssetExtractionStatus::Unsupported),
        "failed" => Ok(AssetExtractionStatus::Failed),
        _ => Err(AssetMetadataCatalogError::CorruptedRecord),
    }
}
fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}
fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn unhex(value: &str) -> Result<String, AssetMetadataCatalogError> {
    if !value.len().is_multiple_of(2) {
        return Err(AssetMetadataCatalogError::CorruptedRecord);
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)
                .map_err(|_| AssetMetadataCatalogError::CorruptedRecord)?;
            u8::from_str_radix(text, 16).map_err(|_| AssetMetadataCatalogError::CorruptedRecord)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| AssetMetadataCatalogError::CorruptedRecord)
}
