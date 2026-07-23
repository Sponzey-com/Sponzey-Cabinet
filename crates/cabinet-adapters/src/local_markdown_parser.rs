use cabinet_domain::asset::AssetId;
use cabinet_domain::document::DocumentBody;
use cabinet_domain::link::SourceRange;
use cabinet_ports::markdown_parser::{
    MarkdownHeading, MarkdownParser, MarkdownParserError, ParsedAssetReference, ParsedDocumentLink,
    ParsedExternalLink, ParsedMarkdown, ParsedWikilink,
};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};

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
        let external_links = parse_external_links(source)?;
        let document_links = parse_document_links(source)?;
        Ok(ParsedMarkdown::new(headings, wikilinks, asset_references)
            .with_external_links(external_links)
            .with_document_links(document_links))
    }
}

fn parse_external_links(source: &str) -> Result<Vec<ParsedExternalLink>, MarkdownParserError> {
    let mut links = Vec::new();
    let mut active: Option<(String, String, std::ops::Range<usize>)> = None;

    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) if is_external_target(dest_url.as_ref()) => {
                active = Some((dest_url.into_string(), String::new(), range));
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, label, _)) = active.as_mut() {
                    label.push_str(&text);
                }
            }
            Event::End(TagEnd::Link) => {
                if let Some((target, label, range)) = active.take() {
                    let source_range = SourceRange::new(range.start, range.end)
                        .map_err(|_| MarkdownParserError::InvalidSourceRange)?;
                    if let Ok(link) = ParsedExternalLink::new(&target, &label, source_range) {
                        links.push(link);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(links)
}

fn is_external_target(target: &str) -> bool {
    let target = target.to_ascii_lowercase();
    target.starts_with("https://") || target.starts_with("http://") || target.starts_with("mailto:")
}

fn parse_document_links(source: &str) -> Result<Vec<ParsedDocumentLink>, MarkdownParserError> {
    let mut links = Vec::new();
    let mut active: Option<(String, String, std::ops::Range<usize>)> = None;

    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) if is_relative_document_target(&dest_url) => {
                active = Some((dest_url.into_string(), String::new(), range));
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, label, _)) = active.as_mut() {
                    label.push_str(&text);
                }
            }
            Event::End(TagEnd::Link) => {
                if let Some((target, label, range)) = active.take() {
                    let source_range = SourceRange::new(range.start, range.end)
                        .map_err(|_| MarkdownParserError::InvalidSourceRange)?;
                    if let Ok(link) = ParsedDocumentLink::new(&target, &label, source_range) {
                        links.push(link);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(links)
}

fn is_relative_document_target(target: &str) -> bool {
    let target = target.trim();
    if target.is_empty()
        || target.starts_with('/')
        || target.contains('\\')
        || target.contains('?')
        || target.contains(':')
        || target.chars().any(char::is_control)
    {
        return false;
    }
    target
        .split_once('#')
        .map_or(target, |(path, _)| path)
        .to_ascii_lowercase()
        .ends_with(".md")
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
