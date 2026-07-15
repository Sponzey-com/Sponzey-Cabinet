use cabinet_domain::asset::{AssetImportDescriptor, AssetImportHandle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetImportChunk {
    offset: u64,
    bytes: Vec<u8>,
    eof: bool,
}

impl AssetImportChunk {
    pub fn new(
        offset: u64,
        bytes: Vec<u8>,
        eof: bool,
        requested_max_bytes: usize,
    ) -> Result<Self, AssetImportSourceError> {
        if requested_max_bytes == 0 {
            return Err(AssetImportSourceError::InvalidChunkLimit);
        }
        if bytes.len() > requested_max_bytes {
            return Err(AssetImportSourceError::ChunkExceedsLimit);
        }
        Ok(Self { offset, bytes, eof })
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn is_eof(&self) -> bool {
        self.eof
    }
}

pub trait AssetImportSource {
    fn describe(
        &self,
        handle: &AssetImportHandle,
    ) -> Result<AssetImportDescriptor, AssetImportSourceError>;

    fn read_chunk(
        &self,
        handle: &AssetImportHandle,
        offset: u64,
        max_bytes: usize,
    ) -> Result<AssetImportChunk, AssetImportSourceError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetImportSourceError {
    InvalidChunkLimit,
    ChunkExceedsLimit,
    HandleNotFound,
    SourceChanged,
    UnsafeSource,
    ReadUnavailable,
}

impl AssetImportSourceError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidChunkLimit => "asset_import.invalid_chunk_limit",
            Self::ChunkExceedsLimit => "asset_import.chunk_exceeds_limit",
            Self::HandleNotFound => "asset_import.handle_not_found",
            Self::SourceChanged => "asset_import.source_changed",
            Self::UnsafeSource => "asset_import.unsafe_source",
            Self::ReadUnavailable => "asset_import.read_unavailable",
        }
    }
}
