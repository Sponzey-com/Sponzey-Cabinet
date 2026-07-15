use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_asset_association_catalog::DurableAssetAssociationCatalog;
use cabinet_adapters::durable_asset_metadata_catalog::DurableAssetMetadataCatalog;
use cabinet_adapters::local_document_repository::LocalDocumentRepository;
use cabinet_desktop_shell::{
    DesktopAssetDetailRequestDto, DesktopAssetImportRequestDto, DesktopAssetImportSelectionRuntime,
    DesktopDocumentAssetsRuntime, DesktopLocalCommandPayloadDto, DesktopLocalCommandRequestDto,
    DesktopWorkspaceAssetsRequestDto,
};
use cabinet_domain::asset::{
    AssetAssociation, AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId,
    AssetMediaType, AssetMetadata, AssetPreviewCapability,
};
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::version::CurrentDocumentSnapshot;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_association_catalog::AssetAssociationCatalog;
use cabinet_ports::asset_metadata_catalog::AssetMetadataCatalog;
use cabinet_ports::document_repository::{CurrentDocumentRecord, DocumentRepository};

const WORKSPACE_ASSET_COUNT: usize = 300;
const DOCUMENT_ASSET_COUNT: usize = 120;
const SAMPLE_COUNT: usize = 40;
const QUERY_BUDGET: Duration = Duration::from_millis(300);

#[test]
fn standard_asset_fixture_list_and_detail_p95_remain_within_budget() {
    let root = TempRoot::new("performance");
    seed_document(&root.path, "doc-1");
    seed_asset_fixture(&root.path);
    let runtime =
        DesktopDocumentAssetsRuntime::new(root.path.clone(), 10 * 1024 * 1024).expect("runtime");

    let mut list_samples = Vec::with_capacity(SAMPLE_COUNT);
    let mut workspace_samples = Vec::with_capacity(SAMPLE_COUNT);
    let mut detail_samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let started = Instant::now();
        let response = runtime.execute(list_request("doc-1"));
        list_samples.push(started.elapsed());
        assert!(response.ok, "response={response:?}");
        assert_eq!(
            response.data.expect("list data").assets.len(),
            DOCUMENT_ASSET_COUNT
        );

        let started = Instant::now();
        let response = runtime.list_workspace(DesktopWorkspaceAssetsRequestDto {
            workspace_id: "workspace-1".into(),
            cursor: None,
            limit: 200,
        });
        workspace_samples.push(started.elapsed());
        assert!(response.ok, "workspace response={response:?}");
        assert_eq!(response.data.expect("workspace data").assets.len(), 200);

        let started = Instant::now();
        let response = runtime.detail(DesktopAssetDetailRequestDto {
            workspace_id: "workspace-1".into(),
            asset_id: asset_hex(1),
        });
        detail_samples.push(started.elapsed());
        assert!(response.ok, "response={response:?}");
    }

    let list = summary(&mut list_samples);
    let workspace = summary(&mut workspace_samples);
    let detail = summary(&mut detail_samples);
    eprintln!(
        "asset_gate fixture_workspace_assets={WORKSPACE_ASSET_COUNT} fixture_document_assets={DOCUMENT_ASSET_COUNT} samples={SAMPLE_COUNT} document_list_p50_us={} document_list_p95_us={} document_list_max_us={} workspace_list_p50_us={} workspace_list_p95_us={} workspace_list_max_us={} detail_p50_us={} detail_p95_us={} detail_max_us={}",
        list.p50.as_micros(),
        list.p95.as_micros(),
        list.max.as_micros(),
        workspace.p50.as_micros(),
        workspace.p95.as_micros(),
        workspace.max.as_micros(),
        detail.p50.as_micros(),
        detail.p95.as_micros(),
        detail.max.as_micros(),
    );
    assert!(list.p95 < QUERY_BUDGET, "list p95 was {:?}", list.p95);
    assert!(
        workspace.p95 < QUERY_BUDGET,
        "workspace list p95 was {:?}",
        workspace.p95
    );
    assert!(detail.p95 < QUERY_BUDGET, "detail p95 was {:?}", detail.p95);
}

#[test]
fn metadata_query_is_not_blocked_by_asset_hashing_and_responses_are_safe() {
    let root = TempRoot::new("concurrency");
    seed_document(&root.path, "doc-1");
    seed_asset(&root.path, 1, "doc-1");
    let source = root.path.join("large-local-source.bin");
    fs::write(&source, vec![0x5a; 32 * 1024 * 1024]).expect("large fixture");
    let import_runtime = DesktopAssetImportSelectionRuntime::with_app_data_root(
        root.path.clone(),
        "workspace-1",
        64 * 1024,
    )
    .expect("import runtime");
    let selection = import_runtime.register_selected_paths(vec![source]);
    let descriptor = &selection.data.expect("selection").files[0];
    let request = DesktopAssetImportRequestDto {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        handle: descriptor.handle.clone(),
        label: "Large fixture".into(),
    };
    let started = import_runtime.start(request.clone());
    let operation_id = started.operation_id.expect("operation id");
    let worker_runtime = import_runtime.clone();
    let worker_operation = operation_id.clone();
    let worker = thread::spawn(move || worker_runtime.run_started(request, &worker_operation));

    let query_runtime =
        DesktopDocumentAssetsRuntime::new(root.path.clone(), 10 * 1024 * 1024).expect("query");
    let query_started = Instant::now();
    let list = query_runtime.execute(list_request("doc-1"));
    let query_elapsed = query_started.elapsed();
    let detail = query_runtime.detail(DesktopAssetDetailRequestDto {
        workspace_id: "workspace-1".into(),
        asset_id: asset_hex(1),
    });
    let imported = worker.join().expect("worker");

    assert!(list.ok, "list={list:?}");
    assert!(detail.ok, "detail={detail:?}");
    assert!(imported.ok, "import={imported:?}");
    assert!(query_elapsed < QUERY_BUDGET, "query took {query_elapsed:?}");
    let serialized = format!(
        "{}\n{}\n{}",
        serde_json::to_string(&list).expect("list json"),
        serde_json::to_string(&detail).expect("detail json"),
        serde_json::to_string(&imported).expect("import json")
    );
    assert!(!serialized.contains(root.path.to_string_lossy().as_ref()));
    assert!(!serialized.contains("large-local-source.bin"));
    assert!(!serialized.contains(&"5a".repeat(64)));
    assert!(!serialized.contains("bytes"));
}

fn seed_asset_fixture(root: &Path) {
    for index in 1..=WORKSPACE_ASSET_COUNT {
        let document = if index <= DOCUMENT_ASSET_COUNT {
            "doc-1"
        } else {
            "other-doc"
        };
        seed_asset(root, index, document);
    }
}

fn seed_asset(root: &Path, index: usize, document: &str) {
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let id = AssetId::from_sha256_hex(&asset_hex(index)).expect("asset id");
    let metadata = AssetMetadata::new(
        id.clone(),
        AssetFileName::new(&format!("fixture-{index}.txt")).expect("file name"),
        AssetMediaType::new("text/plain").expect("media type"),
        index as u64,
    )
    .expect("metadata");
    let record = AssetCatalogRecord::new(
        metadata,
        1,
        AssetPreviewCapability::Text,
        AssetExtractionStatus::NotRequested,
    )
    .expect("record");
    DurableAssetMetadataCatalog::new(root.to_path_buf())
        .put(&workspace, record)
        .expect("metadata put");
    DurableAssetAssociationCatalog::new(root.to_path_buf())
        .link(
            &workspace,
            AssetAssociation::new(
                id,
                DocumentId::new(document).expect("document id"),
                "Fixture",
            )
            .expect("association"),
        )
        .expect("association link");
}

fn seed_document(root: &Path, id: &str) {
    let document_id = DocumentId::new(id).expect("document id");
    let metadata = DocumentMetadata::new(
        document_id.clone(),
        DocumentTitle::new("Asset Host").expect("title"),
        DocumentPath::new("asset-host.md").expect("path"),
    )
    .expect("metadata");
    let snapshot = CurrentDocumentSnapshot::new(
        document_id,
        DocumentBody::new(
            "# Asset Host",
            DocumentBodyPolicy::new(1024).expect("body policy"),
        )
        .expect("body"),
    );
    LocalDocumentRepository::new(root.join("authoring-current"))
        .put_current(
            &WorkspaceId::new("workspace-1").expect("workspace"),
            CurrentDocumentRecord::new(metadata, snapshot).expect("record"),
        )
        .expect("put document");
}

fn list_request(document_id: &str) -> DesktopLocalCommandRequestDto {
    DesktopLocalCommandRequestDto {
        command_name: "list_document_assets".into(),
        payload: DesktopLocalCommandPayloadDto::DocumentIdentity {
            workspace_id: "workspace-1".into(),
            document_id: document_id.into(),
        },
    }
}

fn asset_hex(index: usize) -> String {
    format!("{index:064x}")
}

struct TimingSummary {
    p50: Duration,
    p95: Duration,
    max: Duration,
}

fn summary(samples: &mut [Duration]) -> TimingSummary {
    samples.sort_unstable();
    TimingSummary {
        p50: samples[samples.len() / 2],
        p95: samples[((samples.len() * 95).div_ceil(100)).saturating_sub(1)],
        max: *samples.last().expect("samples"),
    }
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-phase012-asset-gate-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp root");
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
