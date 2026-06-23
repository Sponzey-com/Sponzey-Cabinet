use cabinet_ports::html_renderer::{HtmlDocument, HtmlRendererError};

#[test]
fn html_document_rejects_empty_html() {
    assert_eq!(
        HtmlDocument::new("  ").expect_err("empty html must fail"),
        HtmlRendererError::EmptyHtml
    );
}

#[test]
fn html_document_exposes_rendered_html() {
    let document = HtmlDocument::new("<p>body</p>").expect("html");

    assert_eq!(document.as_str(), "<p>body</p>");
}
