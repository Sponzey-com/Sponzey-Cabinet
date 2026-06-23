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

fn body(value: &str) -> DocumentBody {
    DocumentBody::new(value, DocumentBodyPolicy::new(4096).expect("policy")).expect("body")
}
