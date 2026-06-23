use std::collections::HashMap;

use cabinet_domain::document::{DocumentId, DocumentSlug, DocumentTitle};
use cabinet_domain::link::{Backlink, DocumentLink, LinkTarget, SourceRange};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::link_index::{LinkIndex, LinkIndexError, LinkProjectionRecord};
use cabinet_usecases::graph::{
    GraphEdgeKind, GraphLiteProjectionInput, GraphLiteProjectionUsecase, GraphNodeKind,
};

#[derive(Default)]
struct FakeLinkIndex {
    records: HashMap<(String, String), LinkProjectionRecord>,
}

impl FakeLinkIndex {
    fn insert(&mut self, workspace_id: &str, record: LinkProjectionRecord) {
        self.records.insert(
            (
                workspace_id.to_string(),
                record.source_document_id().as_str().to_string(),
            ),
            record,
        );
    }
}

impl LinkIndex for FakeLinkIndex {
    fn replace_document_links(
        &mut self,
        workspace_id: &WorkspaceId,
        record: LinkProjectionRecord,
    ) -> Result<(), LinkIndexError> {
        self.insert(workspace_id.as_str(), record);
        Ok(())
    }

    fn get_document_links(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<LinkProjectionRecord>, LinkIndexError> {
        Ok(self
            .records
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
            ))
            .cloned())
    }

    fn list_backlinks(
        &self,
        workspace_id: &WorkspaceId,
        target_document_id: &DocumentId,
    ) -> Result<Vec<Backlink>, LinkIndexError> {
        Ok(self
            .records
            .iter()
            .filter(|((record_workspace, _), _)| record_workspace == workspace_id.as_str())
            .flat_map(|(_, record)| record.backlinks().iter())
            .filter(|backlink| backlink.target_document_id() == target_document_id)
            .cloned()
            .collect())
    }

    fn list_unresolved_links(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<DocumentLink>, LinkIndexError> {
        Ok(self
            .records
            .iter()
            .filter(|((record_workspace, _), _)| record_workspace == workspace_id.as_str())
            .flat_map(|(_, record)| record.unresolved_links().iter())
            .cloned()
            .collect())
    }

    fn list_orphan_documents(
        &self,
        _workspace_id: &WorkspaceId,
        _document_ids: &[DocumentId],
    ) -> Result<Vec<DocumentId>, LinkIndexError> {
        Ok(Vec::new())
    }
}

#[test]
fn graph_lite_projection_includes_incoming_outgoing_and_unresolved_depth_one_nodes() {
    let mut link_index = FakeLinkIndex::default();
    link_index.insert(
        "workspace-1",
        LinkProjectionRecord::new(
            document_id("center-doc"),
            vec![Backlink::new(
                document_id("center-doc"),
                document_id("target-doc"),
                SourceRange::new(0, 10).expect("range"),
            )],
            vec![DocumentLink::new(
                document_id("center-doc"),
                LinkTarget::unresolved(slug("Missing Page")),
                SourceRange::new(20, 34).expect("range"),
            )],
        )
        .expect("center projection"),
    );
    link_index.insert(
        "workspace-1",
        LinkProjectionRecord::new(
            document_id("incoming-doc"),
            vec![Backlink::new(
                document_id("incoming-doc"),
                document_id("center-doc"),
                SourceRange::new(0, 10).expect("range"),
            )],
            Vec::new(),
        )
        .expect("incoming projection"),
    );
    let usecase = GraphLiteProjectionUsecase::new();

    let output = usecase
        .execute(
            GraphLiteProjectionInput::new(
                "workspace-1",
                "center-doc",
                vec!["center-doc", "target-doc", "incoming-doc"],
            ),
            &link_index,
        )
        .expect("graph");

    assert!(
        output
            .nodes()
            .iter()
            .any(|node| node.id() == "center-doc" && node.kind() == GraphNodeKind::Document)
    );
    assert!(
        output
            .nodes()
            .iter()
            .any(|node| node.id() == "target-doc" && node.kind() == GraphNodeKind::Document)
    );
    assert!(
        output
            .nodes()
            .iter()
            .any(|node| node.id() == "incoming-doc" && node.kind() == GraphNodeKind::Document)
    );
    assert!(
        output
            .nodes()
            .iter()
            .any(|node| node.id() == "missing-page" && node.kind() == GraphNodeKind::Unresolved)
    );
    assert!(output.edges().iter().any(|edge| {
        edge.source_id() == "center-doc"
            && edge.target_id() == "target-doc"
            && edge.kind() == GraphEdgeKind::Resolved
    }));
    assert!(output.edges().iter().any(|edge| {
        edge.source_id() == "incoming-doc"
            && edge.target_id() == "center-doc"
            && edge.kind() == GraphEdgeKind::Resolved
    }));
    assert!(output.edges().iter().any(|edge| {
        edge.source_id() == "center-doc"
            && edge.target_id() == "missing-page"
            && edge.kind() == GraphEdgeKind::Unresolved
    }));
}

#[test]
fn graph_lite_projection_marks_resolved_target_missing_when_not_in_known_documents() {
    let mut link_index = FakeLinkIndex::default();
    link_index.insert(
        "workspace-1",
        LinkProjectionRecord::new(
            document_id("center-doc"),
            vec![Backlink::new(
                document_id("center-doc"),
                document_id("deleted-doc"),
                SourceRange::new(0, 10).expect("range"),
            )],
            Vec::new(),
        )
        .expect("projection"),
    );
    let usecase = GraphLiteProjectionUsecase::new();

    let output = usecase
        .execute(
            GraphLiteProjectionInput::new("workspace-1", "center-doc", vec!["center-doc"]),
            &link_index,
        )
        .expect("graph");

    assert!(
        output
            .nodes()
            .iter()
            .any(|node| node.id() == "deleted-doc" && node.kind() == GraphNodeKind::Missing)
    );
    assert!(output.edges().iter().any(|edge| {
        edge.source_id() == "center-doc"
            && edge.target_id() == "deleted-doc"
            && edge.kind() == GraphEdgeKind::MissingTarget
    }));
}

fn document_id(value: &str) -> DocumentId {
    DocumentId::new(value).expect("document id")
}

fn slug(title: &str) -> DocumentSlug {
    DocumentSlug::from_title(&DocumentTitle::new(title).expect("title")).expect("slug")
}
