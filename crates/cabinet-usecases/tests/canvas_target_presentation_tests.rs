use cabinet_domain::asset::{
    AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId, AssetMediaType,
    AssetMetadata, AssetPreviewCapability,
};
use cabinet_domain::canvas::{
    CanvasGeometry, CanvasGeometryPolicy, CanvasNode, CanvasNodeId, CanvasNodeTarget,
    CanvasPosition, CanvasSize, CanvasTextCard,
};
use cabinet_domain::document::{DocumentId, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_metadata_catalog::{
    AssetMetadataCatalog, AssetMetadataCatalogError, AssetMetadataPage, AssetMetadataPutOutcome,
};
use cabinet_ports::document_title_reader::{DocumentTitleReader, DocumentTitleReaderError};
use cabinet_ports::document_title_reader::DocumentTitleLookup;
use cabinet_usecases::canvas_target_presentation::{
    CanvasTargetStatus, ResolveCanvasTargetPresentationsInput,
    ResolveCanvasTargetPresentationsUsecase,
};

#[test]
fn presentations_use_one_batch_current_label_lookup_without_exposing_missing_identity() {
    let nodes = vec![
        node(
            "document",
            CanvasNodeTarget::Document(DocumentId::new("doc-1").unwrap()),
        ),
        node(
            "missing-document",
            CanvasNodeTarget::Document(DocumentId::new("doc-missing").unwrap()),
        ),
        node(
            "duplicate-document",
            CanvasNodeTarget::Document(DocumentId::new("doc-1").unwrap()),
        ),
        node(
            "asset",
            CanvasNodeTarget::Attachment(AssetId::from_sha256_hex(&"a".repeat(64)).unwrap()),
        ),
        node(
            "missing-asset",
            CanvasNodeTarget::Attachment(AssetId::from_sha256_hex(&"b".repeat(64)).unwrap()),
        ),
        node(
            "text",
            CanvasNodeTarget::TextCard(CanvasTextCard::new("Memo").unwrap()),
        ),
    ];

    let output = ResolveCanvasTargetPresentationsUsecase::new()
        .execute(
            ResolveCanvasTargetPresentationsInput::new("workspace-1", &nodes),
            &Titles::default(),
            &Assets,
        )
        .unwrap();

    assert_eq!(
        output.presentations()[0].display_label(),
        "Renamed document"
    );
    assert_eq!(
        output.presentations()[0].status(),
        CanvasTargetStatus::Available
    );
    assert_eq!(output.presentations()[1].target_id(), "doc-missing");
    assert_eq!(output.presentations()[1].display_label(), "찾을 수 없는 문서");
    assert_ne!(output.presentations()[1].display_label(), output.presentations()[1].target_id());
    assert_eq!(
        output.presentations()[1].status(),
        CanvasTargetStatus::Missing
    );
    assert_eq!(output.presentations()[2].display_label(), "Renamed document");
    assert_eq!(output.presentations()[3].display_label(), "design.pdf");
    assert_eq!(
        output.presentations()[4].status(),
        CanvasTargetStatus::Missing
    );
    assert_eq!(output.presentations()[4].display_label(), "찾을 수 없는 첨부 파일");
    assert_ne!(output.presentations()[4].display_label(), output.presentations()[4].target_id());
    assert_eq!(output.presentations()[5].display_label(), "Memo");
}

fn node(id: &str, target: CanvasNodeTarget) -> CanvasNode {
    let policy = CanvasGeometryPolicy::new(1, 1000, 1, 1000, 10, 400).unwrap();
    CanvasNode::with_geometry(
        CanvasNodeId::new(id).unwrap(),
        target,
        CanvasGeometry::new(
            CanvasPosition::new(0, 0),
            CanvasSize::new(100, 100, &policy).unwrap(),
        ),
    )
    .unwrap()
}

#[derive(Default)]
struct Titles;
impl DocumentTitleReader for Titles {
    fn get_current_title(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<Option<DocumentTitle>, DocumentTitleReaderError> {
        panic!("Canvas presentation must use one batch title lookup")
    }

    fn get_current_titles(
        &self,
        _: &WorkspaceId,
        documents: &[DocumentId],
    ) -> Result<Vec<DocumentTitleLookup>, DocumentTitleReaderError> {
        assert_eq!(documents.iter().map(DocumentId::as_str).collect::<Vec<_>>(), vec!["doc-1", "doc-missing"]);
        Ok(documents.iter().cloned().map(|document| {
            let title = (document.as_str() == "doc-1").then(|| DocumentTitle::new("Renamed document").unwrap());
            DocumentTitleLookup::new(document, title)
        }).collect())
    }
}

struct Assets;
impl AssetMetadataCatalog for Assets {
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
        asset: &AssetId,
    ) -> Result<Option<AssetCatalogRecord>, AssetMetadataCatalogError> {
        if asset.as_str() != "a".repeat(64) {
            return Ok(None);
        }
        let metadata = AssetMetadata::new(
            asset.clone(),
            AssetFileName::new("design.pdf").unwrap(),
            AssetMediaType::new("application/pdf").unwrap(),
            42,
        )
        .unwrap();
        Ok(Some(
            AssetCatalogRecord::new(
                metadata,
                1,
                AssetPreviewCapability::Pdf,
                AssetExtractionStatus::NotRequested,
            )
            .unwrap(),
        ))
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
