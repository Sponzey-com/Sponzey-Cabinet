use std::collections::HashMap;
use std::fs::{self, File, Metadata};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use cabinet_domain::asset::{AssetImportDescriptor, AssetImportHandle};
use cabinet_ports::asset_import_source::{
    AssetImportChunk, AssetImportSource, AssetImportSourceError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalAssetImportSourceConfig {
    max_chunk_bytes: usize,
}

impl LocalAssetImportSourceConfig {
    pub fn new(max_chunk_bytes: usize) -> Result<Self, AssetImportSourceError> {
        if max_chunk_bytes == 0 {
            return Err(AssetImportSourceError::InvalidChunkLimit);
        }
        Ok(Self { max_chunk_bytes })
    }
}

#[derive(Debug)]
struct RegisteredSource {
    path: PathBuf,
    descriptor: AssetImportDescriptor,
    identity: SourceIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceIdentity {
    byte_size: u64,
    modified_nanos: u128,
}

#[derive(Debug)]
pub struct LocalAssetImportSource {
    config: LocalAssetImportSourceConfig,
    sources: HashMap<AssetImportHandle, RegisteredSource>,
}

impl LocalAssetImportSource {
    pub fn new(config: LocalAssetImportSourceConfig) -> Self {
        Self {
            config,
            sources: HashMap::new(),
        }
    }

    pub fn register_selected_file(
        &mut self,
        handle: AssetImportHandle,
        selected_path: &Path,
    ) -> Result<AssetImportDescriptor, AssetImportSourceError> {
        let metadata = safe_regular_file_metadata(selected_path)?;
        let identity = source_identity(&metadata)?;
        let file_name = selected_path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or(AssetImportSourceError::UnsafeSource)?;
        let descriptor = AssetImportDescriptor::new(
            handle.clone(),
            file_name,
            media_type_for(selected_path),
            metadata.len(),
        )
        .map_err(|_| AssetImportSourceError::UnsafeSource)?;
        let canonical_path =
            fs::canonicalize(selected_path).map_err(|_| AssetImportSourceError::ReadUnavailable)?;
        self.sources.insert(
            handle,
            RegisteredSource {
                path: canonical_path,
                descriptor: descriptor.clone(),
                identity,
            },
        );
        Ok(descriptor)
    }

    fn registered(
        &self,
        handle: &AssetImportHandle,
    ) -> Result<&RegisteredSource, AssetImportSourceError> {
        self.sources
            .get(handle)
            .ok_or(AssetImportSourceError::HandleNotFound)
    }

    fn validate_unchanged(source: &RegisteredSource) -> Result<(), AssetImportSourceError> {
        let metadata = safe_regular_file_metadata(&source.path)?;
        if source_identity(&metadata)? != source.identity {
            return Err(AssetImportSourceError::SourceChanged);
        }
        Ok(())
    }
}

impl AssetImportSource for LocalAssetImportSource {
    fn describe(
        &self,
        handle: &AssetImportHandle,
    ) -> Result<AssetImportDescriptor, AssetImportSourceError> {
        let source = self.registered(handle)?;
        Self::validate_unchanged(source)?;
        Ok(source.descriptor.clone())
    }

    fn read_chunk(
        &self,
        handle: &AssetImportHandle,
        offset: u64,
        max_bytes: usize,
    ) -> Result<AssetImportChunk, AssetImportSourceError> {
        if max_bytes == 0 || max_bytes > self.config.max_chunk_bytes {
            return Err(AssetImportSourceError::InvalidChunkLimit);
        }
        let source = self.registered(handle)?;
        Self::validate_unchanged(source)?;
        if offset > source.identity.byte_size {
            return Err(AssetImportSourceError::SourceChanged);
        }

        let mut file =
            File::open(&source.path).map_err(|_| AssetImportSourceError::ReadUnavailable)?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|_| AssetImportSourceError::ReadUnavailable)?;
        let mut bytes = vec![0; max_bytes];
        let read = file
            .read(&mut bytes)
            .map_err(|_| AssetImportSourceError::ReadUnavailable)?;
        bytes.truncate(read);
        let eof = offset.saturating_add(read as u64) >= source.identity.byte_size;
        AssetImportChunk::new(offset, bytes, eof, max_bytes)
    }
}

fn safe_regular_file_metadata(path: &Path) -> Result<Metadata, AssetImportSourceError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| AssetImportSourceError::ReadUnavailable)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() == 0 {
        return Err(AssetImportSourceError::UnsafeSource);
    }
    Ok(metadata)
}

fn source_identity(metadata: &Metadata) -> Result<SourceIdentity, AssetImportSourceError> {
    let modified_nanos = metadata
        .modified()
        .map_err(|_| AssetImportSourceError::ReadUnavailable)?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AssetImportSourceError::ReadUnavailable)?
        .as_nanos();
    Ok(SourceIdentity {
        byte_size: metadata.len(),
        modified_nanos,
    })
}

fn media_type_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("txt" | "md" | "csv") => "text/plain",
        Some("pdf") => "application/pdf",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}
