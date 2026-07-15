use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::local_atomic_file::write_text_atomically;
use cabinet_domain::asset::{AssetAssociation, AssetId};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_association_catalog::{
    AssetAssociationCatalog, AssetAssociationCatalogError, AssetAssociationLinkOutcome,
    AssetAssociationUnlinkOutcome,
};

const SCHEMA: &str = "schema\t1";
#[derive(Debug, Clone)]
pub struct DurableAssetAssociationCatalog {
    root: PathBuf,
}
impl DurableAssetAssociationCatalog {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    fn workspace_root(&self, workspace: &WorkspaceId) -> PathBuf {
        self.root
            .join("assets/associations")
            .join(hex(workspace.as_str()))
            .join("by-asset")
    }
    fn asset_root(&self, workspace: &WorkspaceId, asset: &AssetId) -> PathBuf {
        self.workspace_root(workspace).join(asset.as_str())
    }
    fn path(&self, workspace: &WorkspaceId, asset: &AssetId, document: &DocumentId) -> PathBuf {
        self.asset_root(workspace, asset)
            .join(format!("{}.link", hex(document.as_str())))
    }
}
impl AssetAssociationCatalog for DurableAssetAssociationCatalog {
    fn link(
        &mut self,
        workspace: &WorkspaceId,
        association: AssetAssociation,
    ) -> Result<AssetAssociationLinkOutcome, AssetAssociationCatalogError> {
        let path = self.path(workspace, association.asset_id(), association.document_id());
        if path.exists() {
            if read(&path)? != association {
                return Err(AssetAssociationCatalogError::Conflict);
            }
            return Ok(AssetAssociationLinkOutcome::AlreadyLinked);
        }
        write_text_atomically(&path, encode(&association))
            .map_err(|_| AssetAssociationCatalogError::StorageUnavailable)?;
        Ok(AssetAssociationLinkOutcome::Linked)
    }
    fn unlink(
        &mut self,
        workspace: &WorkspaceId,
        asset: &AssetId,
        document: &DocumentId,
    ) -> Result<AssetAssociationUnlinkOutcome, AssetAssociationCatalogError> {
        match fs::remove_file(self.path(workspace, asset, document)) {
            Ok(()) => Ok(AssetAssociationUnlinkOutcome::Unlinked),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                Ok(AssetAssociationUnlinkOutcome::NotLinked)
            }
            Err(_) => Err(AssetAssociationCatalogError::StorageUnavailable),
        }
    }
    fn list_documents(
        &self,
        workspace: &WorkspaceId,
        asset: &AssetId,
        limit: usize,
    ) -> Result<Vec<AssetAssociation>, AssetAssociationCatalogError> {
        validate_limit(limit)?;
        let mut paths = read_paths(self.asset_root(workspace, asset))?;
        paths.sort();
        paths
            .into_iter()
            .filter(|path| path.extension().and_then(|v| v.to_str()) == Some("link"))
            .take(limit)
            .map(|path| read(&path))
            .collect()
    }
    fn list_assets(
        &self,
        workspace: &WorkspaceId,
        document: &DocumentId,
        limit: usize,
    ) -> Result<Vec<AssetAssociation>, AssetAssociationCatalogError> {
        validate_limit(limit)?;
        let roots = read_paths(self.workspace_root(workspace))?;
        let mut result = Vec::new();
        for root in roots {
            let path = root.join(format!("{}.link", hex(document.as_str())));
            if path.exists() {
                result.push(read(&path)?);
                if result.len() == limit {
                    break;
                }
            }
        }
        result.sort_by(|a, b| a.asset_id().as_str().cmp(b.asset_id().as_str()));
        Ok(result)
    }
    fn reference_count(
        &self,
        workspace: &WorkspaceId,
        asset: &AssetId,
    ) -> Result<u64, AssetAssociationCatalogError> {
        Ok(self.list_documents(workspace, asset, 500)?.len() as u64)
    }
}
fn validate_limit(limit: usize) -> Result<(), AssetAssociationCatalogError> {
    if limit == 0 || limit > 500 {
        Err(AssetAssociationCatalogError::InvalidLimit)
    } else {
        Ok(())
    }
}
fn read_paths(root: PathBuf) -> Result<Vec<PathBuf>, AssetAssociationCatalogError> {
    match fs::read_dir(root) {
        Ok(entries) => entries
            .map(|entry| {
                entry
                    .map(|value| value.path())
                    .map_err(|_| AssetAssociationCatalogError::StorageUnavailable)
            })
            .collect(),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(Vec::new()),
        Err(_) => Err(AssetAssociationCatalogError::StorageUnavailable),
    }
}
fn encode(value: &AssetAssociation) -> String {
    let payload = format!(
        "asset\t{}\ndocument\t{}\nlabel\t{}\n",
        value.asset_id().as_str(),
        hex(value.document_id().as_str()),
        hex(value.label())
    );
    format!(
        "{SCHEMA}\nchecksum\t{:016x}\n{payload}",
        checksum(payload.as_bytes())
    )
}
fn read(path: &Path) -> Result<AssetAssociation, AssetAssociationCatalogError> {
    let text =
        fs::read_to_string(path).map_err(|_| AssetAssociationCatalogError::StorageUnavailable)?;
    decode(&text)
}
fn decode(text: &str) -> Result<AssetAssociation, AssetAssociationCatalogError> {
    let mut lines = text.lines();
    match lines.next() {
        Some(SCHEMA) => {}
        Some(line) if line.starts_with("schema\t") => {
            return Err(AssetAssociationCatalogError::UnsupportedSchema);
        }
        _ => return Err(AssetAssociationCatalogError::CorruptedRecord),
    }
    let expected = lines
        .next()
        .and_then(|line| line.strip_prefix("checksum\t"))
        .and_then(|v| u64::from_str_radix(v, 16).ok())
        .ok_or(AssetAssociationCatalogError::CorruptedRecord)?;
    let payload = format!("{}\n", lines.collect::<Vec<_>>().join("\n"));
    if checksum(payload.as_bytes()) != expected {
        return Err(AssetAssociationCatalogError::CorruptedRecord);
    }
    let fields = payload
        .lines()
        .map(|line| line.split_once('\t'))
        .collect::<Option<Vec<_>>>()
        .ok_or(AssetAssociationCatalogError::CorruptedRecord)?;
    let find = |key: &str| {
        fields
            .iter()
            .find_map(|(name, value)| (*name == key).then_some(*value))
            .ok_or(AssetAssociationCatalogError::CorruptedRecord)
    };
    AssetAssociation::new(
        AssetId::from_sha256_hex(find("asset")?)
            .map_err(|_| AssetAssociationCatalogError::CorruptedRecord)?,
        DocumentId::new(&unhex(find("document")?)?)
            .map_err(|_| AssetAssociationCatalogError::CorruptedRecord)?,
        &unhex(find("label")?)?,
    )
    .map_err(|_| AssetAssociationCatalogError::CorruptedRecord)
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
fn unhex(value: &str) -> Result<String, AssetAssociationCatalogError> {
    if !value.len().is_multiple_of(2) {
        return Err(AssetAssociationCatalogError::CorruptedRecord);
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)
                .map_err(|_| AssetAssociationCatalogError::CorruptedRecord)?;
            u8::from_str_radix(text, 16).map_err(|_| AssetAssociationCatalogError::CorruptedRecord)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| AssetAssociationCatalogError::CorruptedRecord)
}
