use cabinet_domain::document::{
    DocumentError, DocumentId, DocumentMetadata, DocumentPath, DocumentSlug, DocumentTitle,
};

#[test]
fn document_id_and_title_validate_stable_values() {
    let id = DocumentId::new(" doc-1 ").expect("id should be valid");
    let title = DocumentTitle::new(" Getting Started ").expect("title should be valid");

    assert_eq!(id.as_str(), "doc-1");
    assert_eq!(title.as_str(), "Getting Started");
    assert_eq!(
        DocumentId::new(" ").expect_err("empty id must fail"),
        DocumentError::EmptyId
    );
    assert_eq!(
        DocumentTitle::new("bad\ntitle").expect_err("control character must fail"),
        DocumentError::InvalidTitleCharacter
    );
    assert_eq!(
        DocumentTitle::new(&"a".repeat(121)).expect_err("too long title must fail"),
        DocumentError::TitleTooLong { max: 120 }
    );
}

#[test]
fn document_path_is_logical_markdown_path() {
    let path = DocumentPath::new("guides/getting-started.md").expect("path should be valid");

    assert_eq!(path.as_str(), "guides/getting-started.md");
    assert_eq!(
        DocumentPath::new("/tmp/getting-started.md").expect_err("absolute path must fail"),
        DocumentError::AbsoluteDocumentPath
    );
    assert_eq!(
        DocumentPath::new("../getting-started.md").expect_err("traversal must fail"),
        DocumentError::TraversalPathSegment
    );
    assert_eq!(
        DocumentPath::new("guides/readme.txt").expect_err("non markdown file must fail"),
        DocumentError::InvalidDocumentExtension
    );
}

#[test]
fn document_slug_is_normalized_from_title() {
    let title = DocumentTitle::new(" Hello, Cabinet 101! ").expect("title should be valid");

    let slug = DocumentSlug::from_title(&title).expect("slug should be generated");

    assert_eq!(slug.as_str(), "hello-cabinet-101");
}

#[test]
fn document_metadata_keeps_identity_and_path_when_title_changes() {
    let metadata = DocumentMetadata::new(
        DocumentId::new("doc-1").expect("id"),
        DocumentTitle::new("Getting Started").expect("title"),
        DocumentPath::new("guides/getting-started.md").expect("path"),
    )
    .expect("metadata should be valid");

    let renamed = metadata
        .with_title(DocumentTitle::new("Install Guide").expect("title"))
        .expect("rename should be valid");

    assert_eq!(renamed.id().as_str(), "doc-1");
    assert_eq!(renamed.path().as_str(), "guides/getting-started.md");
    assert_eq!(renamed.title().as_str(), "Install Guide");
    assert_eq!(renamed.slug().as_str(), "install-guide");
}
