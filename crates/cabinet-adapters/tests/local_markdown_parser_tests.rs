use cabinet_adapters::local_markdown_parser::LocalMarkdownParser;
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy};
use cabinet_ports::markdown_parser::MarkdownParser;

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn local_markdown_parser_extracts_headings_wikilinks_and_asset_references() {
    let body = body(&format!(
        "# Title\nSee [[Target Page|target label]] and [[Loose Page]].\n![[asset:{HASH_A}|Diagram]]\n"
    ));
    let parser = LocalMarkdownParser::new();

    let parsed = parser.parse(&body).expect("parse");

    assert_eq!(parsed.headings().len(), 1);
    assert_eq!(parsed.headings()[0].level(), 1);
    assert_eq!(parsed.headings()[0].text(), "Title");
    assert_eq!(parsed.wikilinks().len(), 2);
    assert_eq!(parsed.wikilinks()[0].target(), "Target Page");
    assert_eq!(parsed.wikilinks()[0].label(), Some("target label"));
    assert_eq!(parsed.wikilinks()[1].target(), "Loose Page");
    assert_eq!(parsed.wikilinks()[1].label(), None);
    assert_eq!(parsed.asset_references().len(), 1);
    assert_eq!(parsed.asset_references()[0].asset_id().as_str(), HASH_A);
    assert_eq!(parsed.asset_references()[0].label(), "Diagram");

    let link_range = parsed.wikilinks()[0].source_range();
    assert_eq!(
        &body.as_str()[link_range.start()..link_range.end()],
        "[[Target Page|target label]]"
    );
    let asset_range = parsed.asset_references()[0].source_range();
    assert_eq!(
        &body.as_str()[asset_range.start()..asset_range.end()],
        &format!("![[asset:{HASH_A}|Diagram]]")
    );
}

#[test]
fn local_markdown_parser_ignores_invalid_asset_reference_without_failing_parse() {
    let body = body("# Title\n![[asset:not-a-sha|Broken]]\n[[Still Parsed]]\n");
    let parser = LocalMarkdownParser::new();

    let parsed = parser.parse(&body).expect("parse");

    assert_eq!(parsed.headings().len(), 1);
    assert!(parsed.asset_references().is_empty());
    assert_eq!(parsed.wikilinks().len(), 1);
    assert_eq!(parsed.wikilinks()[0].target(), "Still Parsed");
}

#[test]
fn local_markdown_parser_extracts_standard_external_links_but_ignores_code_and_relative_targets() {
    let body = body(
        "# Links\n[Cabinet](https://user:secret@example.com/docs?q=private)\n\
         [Mail](mailto:team@example.com)\n[Relative](../local.md)\n\
         `[Code](https://code.example)`\n```md\n[Fence](https://fence.example)\n```\n",
    );

    let parsed = LocalMarkdownParser::new().parse(&body).expect("parse");

    assert_eq!(parsed.external_links().len(), 2);
    assert_eq!(
        parsed.external_links()[0].target(),
        "https://user:secret@example.com/docs?q=private"
    );
    assert_eq!(parsed.external_links()[0].label(), "Cabinet");
    assert_eq!(
        parsed.external_links()[1].target(),
        "mailto:team@example.com"
    );
    assert_eq!(parsed.external_links()[1].label(), "Mail");
    let range = parsed.external_links()[0].source_range();
    assert_eq!(
        &body.as_str()[range.start()..range.end()],
        "[Cabinet](https://user:secret@example.com/docs?q=private)"
    );
}

#[test]
fn local_markdown_parser_extracts_relative_document_links_with_fragments_and_ignores_images() {
    let body = body(
        "# Links\n[Sibling](sibling.md)\n[Parent](../shared/note.md#details)\n\
         ![Image](images/picture.md)\n`[Code](hidden.md)`\n[Query](note.md?q=private)\n",
    );

    let parsed = LocalMarkdownParser::new().parse(&body).expect("parse");

    assert_eq!(parsed.document_links().len(), 2);
    assert_eq!(parsed.document_links()[0].target(), "sibling.md");
    assert_eq!(parsed.document_links()[0].label(), "Sibling");
    assert_eq!(
        parsed.document_links()[1].target(),
        "../shared/note.md#details"
    );
    let range = parsed.document_links()[1].source_range();
    assert_eq!(
        &body.as_str()[range.start()..range.end()],
        "[Parent](../shared/note.md#details)"
    );
}

fn body(value: &str) -> DocumentBody {
    DocumentBody::new(value, DocumentBodyPolicy::new(4096).expect("policy")).expect("body")
}
