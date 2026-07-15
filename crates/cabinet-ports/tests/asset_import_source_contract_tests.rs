use std::collections::HashMap;

use cabinet_domain::asset::{AssetImportDescriptor, AssetImportHandle};
use cabinet_ports::asset_import_source::{
    AssetImportChunk, AssetImportSource, AssetImportSourceError,
};

struct FakeImportSource {
    descriptors: HashMap<String, AssetImportDescriptor>,
    bytes: HashMap<String, Vec<u8>>,
}

impl AssetImportSource for FakeImportSource {
    fn describe(
        &self,
        handle: &AssetImportHandle,
    ) -> Result<AssetImportDescriptor, AssetImportSourceError> {
        self.descriptors
            .get(handle.as_str())
            .cloned()
            .ok_or(AssetImportSourceError::HandleNotFound)
    }

    fn read_chunk(
        &self,
        handle: &AssetImportHandle,
        offset: u64,
        max_bytes: usize,
    ) -> Result<AssetImportChunk, AssetImportSourceError> {
        if max_bytes == 0 {
            return Err(AssetImportSourceError::InvalidChunkLimit);
        }
        let bytes = self
            .bytes
            .get(handle.as_str())
            .ok_or(AssetImportSourceError::HandleNotFound)?;
        let start = usize::try_from(offset).map_err(|_| AssetImportSourceError::SourceChanged)?;
        if start > bytes.len() {
            return Err(AssetImportSourceError::SourceChanged);
        }
        let end = start.saturating_add(max_bytes).min(bytes.len());
        AssetImportChunk::new(
            offset,
            bytes[start..end].to_vec(),
            end == bytes.len(),
            max_bytes,
        )
    }
}

#[test]
fn source_contract_describes_and_reads_only_bounded_chunks_by_opaque_handle() {
    let handle = AssetImportHandle::new("picker:contract").expect("handle");
    let descriptor = AssetImportDescriptor::new(handle.clone(), "notes.txt", "text/plain", 5)
        .expect("descriptor");
    let source = FakeImportSource {
        descriptors: HashMap::from([(handle.as_str().to_string(), descriptor.clone())]),
        bytes: HashMap::from([(handle.as_str().to_string(), b"hello".to_vec())]),
    };

    assert_eq!(source.describe(&handle).expect("describe"), descriptor);
    let first = source.read_chunk(&handle, 0, 2).expect("first chunk");
    let second = source.read_chunk(&handle, 2, 8).expect("second chunk");

    assert_eq!(first.offset(), 0);
    assert_eq!(first.bytes(), b"he");
    assert!(!first.is_eof());
    assert_eq!(second.bytes(), b"llo");
    assert!(second.is_eof());
}

#[test]
fn chunk_contract_rejects_zero_limit_and_adapter_overrun() {
    assert_eq!(
        AssetImportChunk::new(0, vec![1], false, 0).expect_err("zero limit"),
        AssetImportSourceError::InvalidChunkLimit
    );
    assert_eq!(
        AssetImportChunk::new(0, vec![1, 2, 3], false, 2).expect_err("overrun"),
        AssetImportSourceError::ChunkExceedsLimit
    );
}

#[test]
fn source_errors_have_stable_non_sensitive_codes() {
    assert_eq!(
        AssetImportSourceError::HandleNotFound.code(),
        "asset_import.handle_not_found"
    );
    assert_eq!(
        AssetImportSourceError::SourceChanged.code(),
        "asset_import.source_changed"
    );
    assert_eq!(
        AssetImportSourceError::UnsafeSource.code(),
        "asset_import.unsafe_source"
    );
    assert_eq!(
        AssetImportSourceError::ReadUnavailable.code(),
        "asset_import.read_unavailable"
    );
}
