use cabinet_domain::document::DocumentId;
use cabinet_domain::link::{Backlink, SourceRange};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::link_index::{
    BacklinkPage, BacklinkPageReader, BacklinkPageRequest, LinkIndexError,
};
use cabinet_usecases::graph::{GetLinkOverviewInput, GetLinkOverviewUsecase};

struct FakeBacklinkPageReader {
    records: Vec<Backlink>,
}

impl BacklinkPageReader for FakeBacklinkPageReader {
    fn list_backlinks_page(
        &self,
        _workspace_id: &WorkspaceId,
        _target_document_id: &DocumentId,
        request: BacklinkPageRequest,
    ) -> Result<BacklinkPage, LinkIndexError> {
        let start = request.offset();
        let end = (start + request.limit()).min(self.records.len());
        let records = self.records[start..end].to_vec();
        let next_offset = (end < self.records.len()).then_some(end);
        Ok(BacklinkPage::new(records, next_offset))
    }
}

#[test]
fn get_link_overview_returns_a_bounded_page_and_cursor() {
    let reader = FakeBacklinkPageReader {
        records: (0..75).map(backlink).collect(),
    };

    let first = GetLinkOverviewUsecase::new()
        .execute(
            GetLinkOverviewInput::new("workspace-1", "target-doc", None, 50),
            &reader,
        )
        .expect("first page");
    assert_eq!(first.backlinks().len(), 50);
    assert_eq!(first.next_cursor(), Some("50"));

    let second = GetLinkOverviewUsecase::new()
        .execute(
            GetLinkOverviewInput::new("workspace-1", "target-doc", first.next_cursor(), 50),
            &reader,
        )
        .expect("second page");
    assert_eq!(second.backlinks().len(), 25);
    assert_eq!(second.next_cursor(), None);
}

#[test]
fn get_link_overview_rejects_invalid_identity_cursor_and_limit_before_calling_reader() {
    let reader = FakeBacklinkPageReader { records: vec![] };
    for input in [
        GetLinkOverviewInput::new("", "target-doc", None, 50),
        GetLinkOverviewInput::new("workspace-1", "", None, 50),
        GetLinkOverviewInput::new("workspace-1", "target-doc", Some("not-a-number"), 50),
        GetLinkOverviewInput::new("workspace-1", "target-doc", None, 0),
        GetLinkOverviewInput::new("workspace-1", "target-doc", None, 501),
    ] {
        assert_eq!(
            GetLinkOverviewUsecase::new()
                .execute(input, &reader)
                .unwrap_err()
                .code(),
            "link_overview.invalid_input"
        );
    }
}

fn backlink(index: usize) -> Backlink {
    Backlink::new(
        DocumentId::new(&format!("source-{index:03}")).expect("source"),
        DocumentId::new("target-doc").expect("target"),
        SourceRange::new(index, index + 1).expect("range"),
    )
}
