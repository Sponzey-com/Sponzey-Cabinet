use cabinet_adapters::durable_local_graph_projection::DurableLocalGraphProjectionStore;
use cabinet_adapters::durable_local_link_index::DurableLocalLinkIndex;
use cabinet_adapters::durable_local_search_index::DurableLocalSearchIndex;
use cabinet_adapters::durable_projection_work_repository::DurableProjectionWorkRepository;
use cabinet_desktop_shell::{
    DesktopDocumentAuthoringRequestDto, DesktopDocumentAuthoringRuntime, DesktopProjectionRuntime,
};
use cabinet_domain::document::{DocumentBodyPolicy, DocumentId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::graph_projection::GraphProjectionStore;
use cabinet_ports::link_index::LinkIndex;
use cabinet_ports::projection_work::ProjectionWorkRepository;
use cabinet_ports::search_index::{SearchIndex, SearchQuery};
use cabinet_usecases::document::DocumentChangeEvent;
use cabinet_usecases::projection_work::EnqueueProjectionWorkUsecase;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn projection_runtime_materializes_search_links_and_graph_across_restart() {
    let temp = Temp::new("projection-e2e");
    let authoring = DesktopDocumentAuthoringRuntime::new(temp.path.clone(), 4096).unwrap();
    assert!(
        authoring
            .execute(create(
                "target",
                "target.md",
                "# Target\ntarget body",
                "tv1"
            ))
            .ok
    );
    assert!(
        authoring
            .execute(create(
                "source",
                "source.md",
                "# Source\nsearch needle [[Target]]",
                "sv1",
            ))
            .ok
    );
    drop(authoring);

    let runtime = DesktopProjectionRuntime::new(temp.path.clone(), 4096, 20, 3).unwrap();
    let stale = runtime.get_freshness("workspace-1", "source");
    assert!(stale.ok);
    assert_eq!(stale.state.as_deref(), Some("stale"));
    let response = runtime.run_once();
    assert!(response.ok);
    assert_eq!(response.ready_count, 6);
    assert_eq!(response.retry_scheduled_count, 0);
    assert_eq!(response.failed_count, 0);
    let ready = runtime.get_freshness("workspace-1", "source");
    assert_eq!(ready.state.as_deref(), Some("ready"));
    assert_eq!(ready.current_version_id.as_deref(), Some("sv1"));
    assert_eq!(ready.projections.len(), 3);

    let reindex = runtime.request_reindex("workspace-1", "source");
    assert!(reindex.ok);
    assert_eq!(reindex.reset_count, 3);
    assert_eq!(
        runtime
            .get_freshness("workspace-1", "source")
            .state
            .as_deref(),
        Some("stale")
    );
    let repaired = runtime.run_once();
    assert_eq!(repaired.ready_count, 3);
    assert_eq!(
        runtime
            .get_freshness("workspace-1", "source")
            .state
            .as_deref(),
        Some("ready")
    );
    drop(runtime);

    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let source = DocumentId::new("source").unwrap();
    let search =
        DurableLocalSearchIndex::new(temp.path.clone(), DocumentBodyPolicy::new(4096).unwrap());
    let page = search
        .search(&workspace, SearchQuery::new("needle", 10).unwrap())
        .unwrap();
    assert_eq!(page.results().len(), 1);
    assert_eq!(page.results()[0].document_id().as_str(), "source");

    let links = DurableLocalLinkIndex::new(temp.path.clone());
    let projected = links
        .get_document_links(&workspace, &source)
        .unwrap()
        .expect("link projection");
    assert_eq!(projected.backlinks().len(), 1);
    assert_eq!(
        projected.backlinks()[0].target_document_id().as_str(),
        "target"
    );

    let graph = DurableLocalGraphProjectionStore::new(temp.path.clone())
        .get_projection(&workspace, &source)
        .unwrap()
        .expect("graph projection");
    assert_eq!(graph.freshness_revision(), "sv1");
    assert_eq!(graph.graph().edges().len(), 1);

    assert!(
        DurableProjectionWorkRepository::new(temp.path.clone())
            .list_resumable(20)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn projection_runtime_rejects_invalid_startup_policy_without_environment_lookup() {
    let temp = Temp::new("invalid-policy");
    assert_eq!(
        DesktopProjectionRuntime::new(temp.path.clone(), 0, 20, 3)
            .err()
            .expect("body policy"),
        "PROJECTION_INVALID_BODY_POLICY"
    );
    assert_eq!(
        DesktopProjectionRuntime::new(temp.path.clone(), 4096, 0, 3)
            .err()
            .expect("worker policy"),
        "PROJECTION_INVALID_WORKER_POLICY"
    );
}

#[test]
fn deleted_work_removes_all_document_projections_across_restart() {
    let temp = Temp::new("projection-delete");
    let authoring = DesktopDocumentAuthoringRuntime::new(temp.path.clone(), 4096).unwrap();
    assert!(
        authoring
            .execute(create(
                "source",
                "source.md",
                "# Source\nsearch needle [[Missing]]",
                "sv1",
            ))
            .ok
    );
    drop(authoring);
    let runtime = DesktopProjectionRuntime::new(temp.path.clone(), 4096, 20, 3).unwrap();
    assert_eq!(runtime.run_once().ready_count, 3);
    drop(runtime);

    let mut work = DurableProjectionWorkRepository::new(temp.path.clone());
    let queued = EnqueueProjectionWorkUsecase::new()
        .execute(
            DocumentChangeEvent::DocumentDeleted {
                workspace_id: "workspace-1".to_string(),
                document_id: "source".to_string(),
                version_id: "sv1".to_string(),
            },
            &mut work,
        )
        .unwrap();
    assert_eq!(queued.enqueued_count(), 3);
    drop(work);

    let runtime = DesktopProjectionRuntime::new(temp.path.clone(), 4096, 20, 3).unwrap();
    let removed = runtime.run_once();
    assert_eq!(removed.ready_count, 3);
    assert_eq!(removed.failed_count, 0);
    drop(runtime);

    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let source = DocumentId::new("source").unwrap();
    let search =
        DurableLocalSearchIndex::new(temp.path.clone(), DocumentBodyPolicy::new(4096).unwrap());
    assert!(
        search
            .search(&workspace, SearchQuery::new("needle", 10).unwrap())
            .unwrap()
            .results()
            .is_empty()
    );
    assert!(
        DurableLocalLinkIndex::new(temp.path.clone())
            .get_document_links(&workspace, &source)
            .unwrap()
            .is_none()
    );
    assert!(
        DurableLocalGraphProjectionStore::new(temp.path.clone())
            .get_projection(&workspace, &source)
            .unwrap()
            .is_none()
    );
}

fn create(
    document_id: &str,
    path: &str,
    body: &str,
    version_id: &str,
) -> DesktopDocumentAuthoringRequestDto {
    DesktopDocumentAuthoringRequestDto::Create {
        workspace_id: "workspace-1".to_string(),
        document_id: document_id.to_string(),
        path: path.to_string(),
        body: body.to_string(),
        version_id: version_id.to_string(),
        snapshot_ref: format!("snapshot-{version_id}"),
        author: "local-user".to_string(),
        summary: "Created".to_string(),
    }
}

struct Temp {
    path: PathBuf,
}

impl Temp {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cabinet-desktop-projection-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
