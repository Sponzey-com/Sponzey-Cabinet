use cabinet_domain::asset::AssetId;
use cabinet_domain::document::DocumentBody;
use cabinet_domain::link::SourceRange;
use cabinet_ports::markdown_parser::{
    MarkdownHeading, MarkdownParser, MarkdownParserError, ParsedAssetReference, ParsedMarkdown,
    ParsedWikilink,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalMarkdownParser;

impl LocalMarkdownParser {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for LocalMarkdownParser {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownParser for LocalMarkdownParser {
    fn parse(&self, body: &DocumentBody) -> Result<ParsedMarkdown, MarkdownParserError> {
        let source = body.as_str();
        let headings = parse_headings(source)?;
        let (wikilinks, asset_references) = parse_inline_references(source)?;
        Ok(ParsedMarkdown::new(headings, wikilinks, asset_references))
    }
}

fn parse_headings(source: &str) -> Result<Vec<MarkdownHeading>, MarkdownParserError> {
    let mut headings = Vec::new();
    let mut offset = 0;

    for segment in source.split_inclusive('\n') {
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        let level = line.bytes().take_while(|byte| *byte == b'#').count();
        if (1..=6).contains(&level) && line.as_bytes().get(level) == Some(&b' ') {
            let range = SourceRange::new(offset, offset + line.len())
                .map_err(|_| MarkdownParserError::InvalidSourceRange)?;
            if let Ok(heading) = MarkdownHeading::new(level as u8, &line[level + 1..], range) {
                headings.push(heading);
            }
        }
        offset += segment.len();
    }

    Ok(headings)
}

fn parse_inline_references(
    source: &str,
) -> Result<(Vec<ParsedWikilink>, Vec<ParsedAssetReference>), MarkdownParserError> {
    let mut wikilinks = Vec::new();
    let mut asset_references = Vec::new();
    let mut cursor = 0;

    while let Some(relative_open) = source[cursor..].find("[[") {
        let open = cursor + relative_open;
        let is_asset_reference = open > 0 && source.as_bytes()[open - 1] == b'!';
        let start = if is_asset_reference { open - 1 } else { open };
        let content_start = open + 2;
        let Some(relative_close) = source[content_start..].find("]]") else {
            break;
        };
        let content_end = content_start + relative_close;
        let end = content_end + 2;
        let range =
            SourceRange::new(start, end).map_err(|_| MarkdownParserError::InvalidSourceRange)?;
        let content = &source[content_start..content_end];

        if is_asset_reference {
            if let Some(asset_reference) = parse_asset_reference(content, range) {
                asset_references.push(asset_reference);
            }
        } else if let Some(wikilink) = parse_wikilink(content, range) {
            wikilinks.push(wikilink);
        }

        cursor = end;
    }

    Ok((wikilinks, asset_references))
}

fn parse_wikilink(content: &str, range: SourceRange) -> Option<ParsedWikilink> {
    let (target, label) = match content.split_once('|') {
        Some((target, label)) => (target, Some(label)),
        None => (content, None),
    };
    ParsedWikilink::new(target, label, range).ok()
}

fn parse_asset_reference(content: &str, range: SourceRange) -> Option<ParsedAssetReference> {
    let rest = content.strip_prefix("asset:")?;
    let (asset_id, label) = rest.split_once('|')?;
    let asset_id = AssetId::from_sha256_hex(asset_id).ok()?;
    ParsedAssetReference::new(asset_id, label, range).ok()
}
