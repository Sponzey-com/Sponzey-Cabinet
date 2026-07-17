use cabinet_domain::asset::AssetId;
use cabinet_domain::asset::{
    AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetMediaType, AssetMetadata,
    AssetPreviewCapability,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_external_open::{AssetExternalOpenError, AssetExternalOpener};
use cabinet_ports::asset_metadata_catalog::{
    AssetMetadataCatalog, AssetMetadataCatalogError, AssetMetadataPage, AssetMetadataPutOutcome,
};
use cabinet_usecases::asset_external_open::{
    AssetExternalOpenProductEvent, AssetExternalOpenProductLogger, OpenAssetExternallyInput,
    OpenAssetExternallyUsecase,
};
use std::sync::Mutex;

#[test]
fn validated_asset_identity_is_opened_without_returning_a_path() {
    let opener = RecordingOpener::new(Ok(()));
    let mut logger = RecordingProductLogger::default();
    let output = OpenAssetExternallyUsecase::new()
        .execute(
            OpenAssetExternallyInput::new("workspace-1", &"a".repeat(64)).expect("input"),
            &Catalog::new("a"),
            &opener,
            &mut logger,
        )
        .expect("opened");

    assert!(output.opened());
    assert_eq!(opener.calls(), vec![("workspace-1".into(), "a".repeat(64))]);
    assert!(!format!("{output:?}").contains('/'));
    assert!(logger.events.is_empty());
}

#[test]
fn invalid_identity_and_launcher_failure_return_stable_path_free_errors() {
    let invalid = OpenAssetExternallyInput::new("", "not-a-digest").expect_err("invalid");
    assert_eq!(invalid.code(), "asset_external_open.invalid_input");

    let mut logger = RecordingProductLogger::default();
    let error = OpenAssetExternallyUsecase::new()
        .execute(
            OpenAssetExternallyInput::new("workspace-1", &"b".repeat(64)).expect("input"),
            &Catalog::new("b"),
            &RecordingOpener::new(Err(AssetExternalOpenError::LauncherUnavailable)),
            &mut logger,
        )
        .expect_err("launcher failure");
    assert_eq!(error.code(), "asset_external_open.launcher_unavailable");
    assert!(error.retryable());
    assert!(!format!("{error:?}").contains('/'));
    assert_eq!(
        logger.events,
        vec![AssetExternalOpenProductEvent::Failed {
            error_code: "asset_external_open.launcher_unavailable",
        }]
    );
}

#[derive(Default)]
struct RecordingProductLogger {
    events: Vec<AssetExternalOpenProductEvent>,
}

impl AssetExternalOpenProductLogger for RecordingProductLogger {
    fn write_product(&mut self, event: AssetExternalOpenProductEvent) {
        self.events.push(event);
    }
}

struct RecordingOpener {
    result: Result<(), AssetExternalOpenError>,
    calls: Mutex<Vec<(String, String)>>,
}

impl RecordingOpener {
    fn new(result: Result<(), AssetExternalOpenError>) -> Self {
        Self {
            result,
            calls: Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<(String, String)> {
        self.calls.lock().expect("calls").clone()
    }
}

impl AssetExternalOpener for RecordingOpener {
    fn open(
        &self,
        workspace: &WorkspaceId,
        asset: &AssetId,
        _: &AssetFileName,
    ) -> Result<(), AssetExternalOpenError> {
        self.calls
            .lock()
            .expect("calls")
            .push((workspace.as_str().into(), asset.as_str().into()));
        self.result
    }
}

struct Catalog(AssetCatalogRecord);

impl Catalog {
    fn new(digest: &str) -> Self {
        let media_type = AssetMediaType::new("text/plain").expect("media");
        let metadata = AssetMetadata::new(
            AssetId::from_sha256_hex(&digest.repeat(64)).expect("asset"),
            AssetFileName::new("fixture.txt").expect("name"),
            media_type.clone(),
            7,
        )
        .expect("metadata");
        Self(
            AssetCatalogRecord::new(
                metadata,
                1,
                AssetPreviewCapability::for_media_type(&media_type),
                AssetExtractionStatus::NotRequested,
            )
            .expect("record"),
        )
    }
}

impl AssetMetadataCatalog for Catalog {
    fn put(
        &mut self,
        _: &WorkspaceId,
        _: AssetCatalogRecord,
    ) -> Result<AssetMetadataPutOutcome, AssetMetadataCatalogError> {
        unreachable!()
    }
    fn get(
        &self,
        _: &WorkspaceId,
        _: &AssetId,
    ) -> Result<Option<AssetCatalogRecord>, AssetMetadataCatalogError> {
        Ok(Some(self.0.clone()))
    }
    fn list(
        &self,
        _: &WorkspaceId,
        _: Option<&str>,
        _: usize,
    ) -> Result<AssetMetadataPage, AssetMetadataCatalogError> {
        unreachable!()
    }
}
