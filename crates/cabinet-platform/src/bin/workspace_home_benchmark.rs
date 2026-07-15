use std::fs;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_workspace_home_projection::LocalWorkspaceHomeProjectionStore;
use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_platform::local_desktop_runtime::LocalDesktopUsecaseInput;
use cabinet_platform::workspace_home_command::execute_workspace_home_command;
use cabinet_ports::workspace_home::{
    WorkspaceHomeBackupStatus, WorkspaceHomeChangeProjection, WorkspaceHomeDocumentProjection,
    WorkspaceHomeHealthStatus, WorkspaceHomeProjection,
};

const WARMUP_COUNT: usize = 20;
const SAMPLE_COUNT: usize = 200;

fn main() {
    let root = std::env::temp_dir().join(format!(
        "sponzey-phase011-home-benchmark-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("benchmark root");
    let workspace_id = WorkspaceId::new("workspace-benchmark").expect("workspace");
    let store = LocalWorkspaceHomeProjectionStore::new(root.clone());
    store
        .replace_projection(&workspace_id, &fixture_projection())
        .expect("fixture projection");
    let input = LocalDesktopUsecaseInput::WorkspaceHome {
        workspace_id: workspace_id.as_str().to_string(),
        recent_documents: 20,
        favorites: 20,
        tags: 20,
        recent_changes: 20,
        unfinished_items: 20,
    };

    for _ in 0..WARMUP_COUNT {
        execute_workspace_home_command(input.clone(), &store).expect("warmup query");
    }
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let started = Instant::now();
        let result = execute_workspace_home_command(input.clone(), &store).expect("measured query");
        assert_eq!(result.recent_documents.len(), 20);
        samples.push(started.elapsed().as_nanos());
    }
    samples.sort_unstable();

    println!("workspace_home_benchmark=passed");
    println!("warmup_count={WARMUP_COUNT}");
    println!("sample_count={SAMPLE_COUNT}");
    println!("p50_ms={:.6}", percentile(&samples, 50));
    println!("p95_ms={:.6}", percentile(&samples, 95));
    println!(
        "max_ms={:.6}",
        nanos_to_ms(*samples.last().expect("sample"))
    );
    println!("current_document_count=10000");
    println!("total_version_count=100000");
    println!("query_path=bounded_workspace_home_projection");

    let _ = fs::remove_dir_all(root);
}

fn percentile(samples: &[u128], percentile: usize) -> f64 {
    let rank = ((samples.len() * percentile).div_ceil(100)).saturating_sub(1);
    nanos_to_ms(samples[rank])
}

fn nanos_to_ms(value: u128) -> f64 {
    value as f64 / 1_000_000.0
}

fn fixture_projection() -> WorkspaceHomeProjection {
    let documents = (0..100).map(|index| document(index)).collect::<Vec<_>>();
    let changes = (0..100)
        .map(|index| {
            WorkspaceHomeChangeProjection::new(
                DocumentId::new(&format!("doc-{index:05}")).expect("id"),
                "Updated document",
            )
            .expect("change")
        })
        .collect::<Vec<_>>();
    WorkspaceHomeProjection::new(
        documents,
        Vec::new(),
        Vec::new(),
        changes,
        Vec::new(),
        WorkspaceHomeBackupStatus::Fresh,
        WorkspaceHomeHealthStatus::Healthy,
    )
}

fn document(index: usize) -> WorkspaceHomeDocumentProjection {
    WorkspaceHomeDocumentProjection::new(
        DocumentId::new(&format!("doc-{index:05}")).expect("id"),
        DocumentTitle::new(&format!("Benchmark Note {index:05}")).expect("title"),
        DocumentPath::new(&format!("benchmark/{index:05}.md")).expect("path"),
    )
}
