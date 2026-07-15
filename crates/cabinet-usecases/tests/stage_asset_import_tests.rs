use std::cell::RefCell;

use cabinet_domain::asset::{AssetImportDescriptor, AssetImportHandle};
use cabinet_domain::asset_import_operation::AssetImportOperationId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_import_source::{
    AssetImportChunk, AssetImportSource, AssetImportSourceError,
};
use cabinet_ports::asset_staging::{AssetStagingError, AssetStagingWriter, StagedAsset};
use cabinet_usecases::asset_import::{StageAssetImportInput, StageAssetImportUsecase};

struct FakeSource {
    bytes: Vec<u8>,
    requests: RefCell<Vec<(u64, usize)>>,
}
impl AssetImportSource for FakeSource {
    fn describe(
        &self,
        handle: &AssetImportHandle,
    ) -> Result<AssetImportDescriptor, AssetImportSourceError> {
        AssetImportDescriptor::new(
            handle.clone(),
            "notes.txt",
            "text/plain",
            self.bytes.len() as u64,
        )
        .map_err(|_| AssetImportSourceError::UnsafeSource)
    }
    fn read_chunk(
        &self,
        _handle: &AssetImportHandle,
        offset: u64,
        max: usize,
    ) -> Result<AssetImportChunk, AssetImportSourceError> {
        self.requests.borrow_mut().push((offset, max));
        let start = offset as usize;
        let end = (start + max).min(self.bytes.len());
        AssetImportChunk::new(
            offset,
            self.bytes[start..end].to_vec(),
            end == self.bytes.len(),
            max,
        )
    }
}

#[derive(Default)]
struct FakeWriter {
    bytes: Vec<u8>,
    cleaned: bool,
}
impl AssetStagingWriter for FakeWriter {
    fn begin(
        &mut self,
        _workspace: &WorkspaceId,
        _operation: &AssetImportOperationId,
    ) -> Result<(), AssetStagingError> {
        self.bytes.clear();
        Ok(())
    }
    fn append(
        &mut self,
        _workspace: &WorkspaceId,
        _operation: &AssetImportOperationId,
        offset: u64,
        bytes: &[u8],
    ) -> Result<(), AssetStagingError> {
        if offset != self.bytes.len() as u64 {
            return Err(AssetStagingError::OffsetConflict);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn finalize(
        &mut self,
        _workspace: &WorkspaceId,
        operation: &AssetImportOperationId,
        expected: u64,
    ) -> Result<StagedAsset, AssetStagingError> {
        StagedAsset::new(operation.clone(), self.bytes.len() as u64, expected)
    }
    fn cleanup(
        &mut self,
        _workspace: &WorkspaceId,
        _operation: &AssetImportOperationId,
    ) -> Result<(), AssetStagingError> {
        self.cleaned = true;
        Ok(())
    }
}

#[test]
fn staging_usecase_streams_bounded_chunks_without_whole_file_input() {
    let source = FakeSource {
        bytes: b"abcdefgh".to_vec(),
        requests: RefCell::new(Vec::new()),
    };
    let mut writer = FakeWriter::default();
    let result = StageAssetImportUsecase::new()
        .execute(input(3), &source, &mut writer)
        .expect("stage");
    assert_eq!(result.byte_size(), 8);
    assert_eq!(writer.bytes, b"abcdefgh");
    assert_eq!(&*source.requests.borrow(), &[(0, 3), (3, 3), (6, 3)]);
}

struct ShortSource;
impl AssetImportSource for ShortSource {
    fn describe(
        &self,
        handle: &AssetImportHandle,
    ) -> Result<AssetImportDescriptor, AssetImportSourceError> {
        AssetImportDescriptor::new(handle.clone(), "notes.txt", "text/plain", 8)
            .map_err(|_| AssetImportSourceError::UnsafeSource)
    }
    fn read_chunk(
        &self,
        _handle: &AssetImportHandle,
        offset: u64,
        max: usize,
    ) -> Result<AssetImportChunk, AssetImportSourceError> {
        AssetImportChunk::new(offset, b"abc".to_vec(), true, max)
    }
}

#[test]
fn staging_usecase_cleans_partial_file_when_source_ends_before_descriptor_size() {
    let mut writer = FakeWriter::default();
    let error = StageAssetImportUsecase::new()
        .execute(input(4), &ShortSource, &mut writer)
        .expect_err("short source");
    assert_eq!(error.code(), "asset_import.size_mismatch");
    assert!(writer.cleaned);
}

fn input(chunk: usize) -> StageAssetImportInput {
    StageAssetImportInput::new("workspace-1", "import-1", "picker:1", chunk).expect("input")
}
