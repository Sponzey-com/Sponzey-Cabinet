use cabinet_adapters::local_html_renderer::LocalHtmlRenderer;
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy};
use cabinet_ports::html_renderer::HtmlRenderer;

#[test]
fn local_html_renderer_renders_headings_paragraphs_and_escapes_html() {
    let renderer = LocalHtmlRenderer::new();
    let body = body("# Title <unsafe>\n\nParagraph & content");

    let html = renderer.render(&body).expect("html");

    assert!(html.as_str().contains("<h1>Title &lt;unsafe&gt;</h1>"));
    assert!(html.as_str().contains("<p>Paragraph &amp; content</p>"));
}

fn body(value: &str) -> DocumentBody {
    DocumentBody::new(value, DocumentBodyPolicy::new(1024).expect("policy")).expect("body")
}
