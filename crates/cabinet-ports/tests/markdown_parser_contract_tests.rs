use cabinet_domain::asset::AssetId;
use cabinet_domain::link::SourceRange;
use cabinet_ports::markdown_parser::{
    MarkdownHeading, MarkdownParserError, ParsedAssetReference, ParsedWikilink,
};

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn markdown_heading_accepts_levels_one_to_six() {
    let heading = MarkdownHeading::new(2, "Overview", SourceRange::new(0, 11).expect("range"))
        .expect("heading");

    assert_eq!(heading.level(), 2);
    assert_eq!(heading.text(), "Overview");
    assert_eq!(heading.source_range().start(), 0);
}

#[test]
fn markdown_heading_rejects_invalid_level_or_empty_text() {
    assert_eq!(
        MarkdownHeading::new(0, "Overview", SourceRange::new(0, 8).expect("range"))
            .expect_err("level zero must fail"),
        MarkdownParserError::InvalidHeadingLevel
    );
    assert_eq!(
        MarkdownHeading::new(7, "Overview", SourceRange::new(0, 8).expect("range"))
            .expect_err("level seven must fail"),
        MarkdownParserError::InvalidHeadingLevel
    );
    assert_eq!(
        MarkdownHeading::new(1, "  ", SourceRange::new(0, 2).expect("range"))
            .expect_err("empty heading must fail"),
        MarkdownParserError::EmptyHeadingText
    );
}

#[test]
fn parsed_wikilink_and_asset_reference_expose_range_without_object_bytes() {
    let wikilink = ParsedWikilink::new(
        "Target Page",
        Some("Label"),
        SourceRange::new(4, 25).expect("range"),
    )
    .expect("wikilink");
    let asset = ParsedAssetReference::new(
        AssetId::from_sha256_hex(HASH_A).expect("asset id"),
        "Diagram",
        SourceRange::new(30, 120).expect("range"),
    )
    .expect("asset reference");

    assert_eq!(wikilink.target(), "Target Page");
    assert_eq!(wikilink.label(), Some("Label"));
    assert_eq!(asset.asset_id().as_str(), HASH_A);
    assert_eq!(asset.label(), "Diagram");
}
