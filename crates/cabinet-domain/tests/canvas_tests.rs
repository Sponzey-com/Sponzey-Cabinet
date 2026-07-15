use cabinet_domain::asset::AssetId;
use cabinet_domain::canvas::{
    Canvas, CanvasEdge, CanvasEdgeId, CanvasEmbed, CanvasError, CanvasExternalLink, CanvasGeometry,
    CanvasGeometryPolicy, CanvasId, CanvasLifecycleEvent, CanvasLifecycleState, CanvasNode,
    CanvasNodeId, CanvasNodeTarget, CanvasPosition, CanvasRevision, CanvasSize, CanvasTextCard,
    CanvasTitle, CanvasViewport, transition_canvas_lifecycle,
};
use cabinet_domain::document::DocumentId;

#[test]
fn canvas_revision_title_geometry_and_viewport_validate_boundaries() {
    let revision = CanvasRevision::new(1).expect("revision");
    assert_eq!(revision.next().expect("next").value(), 2);
    assert_eq!(
        CanvasRevision::new(0).expect_err("zero"),
        CanvasError::InvalidRevision
    );
    assert_eq!(
        CanvasRevision::new(u64::MAX)
            .expect("max")
            .next()
            .expect_err("overflow"),
        CanvasError::RevisionOverflow
    );
    assert_eq!(
        CanvasTitle::new("  Product map  ").expect("title").as_str(),
        "Product map"
    );
    assert!(CanvasTitle::new("\n").is_err());

    let policy = CanvasGeometryPolicy::new(80, 1200, 60, 900, 25, 400).expect("policy");
    let geometry = CanvasGeometry::new(
        CanvasPosition::new(10, 20),
        CanvasSize::new(320, 180, &policy).expect("size"),
    );
    assert_eq!(geometry.size().width(), 320);
    assert!(CanvasSize::new(79, 180, &policy).is_err());
    assert_eq!(
        CanvasViewport::new(100, 200, 100, &policy)
            .expect("viewport")
            .zoom_percent(),
        100
    );
    assert!(CanvasViewport::new(0, 0, 401, &policy).is_err());
}

#[test]
fn canvas_rejects_edge_with_missing_node_reference() {
    let source = CanvasNode::new(
        CanvasNodeId::new("node-1").expect("node id"),
        CanvasNodeTarget::Document(document_id("doc-1")),
        CanvasPosition::new(0, 0),
    )
    .expect("source node");
    let edge = CanvasEdge::new(
        CanvasEdgeId::new("edge-1").expect("edge id"),
        source.id().clone(),
        CanvasNodeId::new("missing-node").expect("target id"),
    )
    .expect("edge");

    let error = Canvas::new(
        CanvasId::new("canvas-1").expect("canvas id"),
        vec![source],
        vec![edge],
        CanvasLifecycleState::Draft,
    )
    .expect_err("missing edge node");

    assert_eq!(error, CanvasError::MissingEdgeNode);
    assert_eq!(error.code(), "canvas.missing_edge_node");
}

#[test]
fn canvas_accepts_document_attachment_external_link_and_text_card_nodes() {
    let canvas = Canvas::new(
        CanvasId::new("canvas-1").expect("canvas id"),
        vec![
            CanvasNode::new(
                CanvasNodeId::new("doc-node").expect("node id"),
                CanvasNodeTarget::Document(document_id("doc-1")),
                CanvasPosition::new(0, 0),
            )
            .expect("document node"),
            CanvasNode::new(
                CanvasNodeId::new("asset-node").expect("node id"),
                CanvasNodeTarget::Attachment(asset_id()),
                CanvasPosition::new(100, 0),
            )
            .expect("attachment node"),
            CanvasNode::new(
                CanvasNodeId::new("external-node").expect("node id"),
                CanvasNodeTarget::ExternalLink(
                    CanvasExternalLink::new("https://example.com/reference").expect("external"),
                ),
                CanvasPosition::new(200, 0),
            )
            .expect("external link node"),
            CanvasNode::new(
                CanvasNodeId::new("text-node").expect("node id"),
                CanvasNodeTarget::TextCard(CanvasTextCard::new("Project note").expect("text")),
                CanvasPosition::new(300, 0),
            )
            .expect("text node"),
        ],
        vec![],
        CanvasLifecycleState::Draft,
    )
    .expect("canvas");

    assert_eq!(canvas.nodes().len(), 4);
    assert_eq!(canvas.state(), CanvasLifecycleState::Draft);
}

#[test]
fn canvas_embed_uses_stable_reference_without_raw_ui_state() {
    let embed = CanvasEmbed::new(CanvasId::new("canvas-1").expect("canvas id"));

    assert_eq!(embed.reference(), "canvas:canvas-1");
    assert!(!embed.reference().contains('{'));
    assert!(!embed.reference().contains("nodes"));
    assert!(!embed.reference().contains("edges"));
}

#[test]
fn canvas_lifecycle_uses_explicit_transitions() {
    assert_eq!(
        transition_canvas_lifecycle(CanvasLifecycleState::Draft, CanvasLifecycleEvent::Save)
            .expect("save"),
        CanvasLifecycleState::Saved,
    );
    assert_eq!(
        transition_canvas_lifecycle(CanvasLifecycleState::Saved, CanvasLifecycleEvent::Embed)
            .expect("embed"),
        CanvasLifecycleState::Embedded,
    );
    assert_eq!(
        transition_canvas_lifecycle(CanvasLifecycleState::Embedded, CanvasLifecycleEvent::Update)
            .expect("update"),
        CanvasLifecycleState::Updated,
    );
    assert_eq!(
        transition_canvas_lifecycle(CanvasLifecycleState::Updated, CanvasLifecycleEvent::Save)
            .expect("save updated"),
        CanvasLifecycleState::Saved,
    );
    assert_eq!(
        transition_canvas_lifecycle(CanvasLifecycleState::Saved, CanvasLifecycleEvent::Archive)
            .expect("archive"),
        CanvasLifecycleState::Archived,
    );

    let error =
        transition_canvas_lifecycle(CanvasLifecycleState::Archived, CanvasLifecycleEvent::Update)
            .expect_err("invalid transition");
    assert_eq!(error, CanvasError::InvalidLifecycleTransition);
    assert_eq!(error.code(), "canvas.invalid_lifecycle_transition");
}

fn document_id(value: &str) -> DocumentId {
    DocumentId::new(value).expect("document id")
}

fn asset_id() -> AssetId {
    AssetId::from_sha256_hex("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        .expect("asset id")
}
