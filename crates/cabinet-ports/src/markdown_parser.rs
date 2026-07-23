use cabinet_domain::asset::AssetId;
use cabinet_domain::document::DocumentBody;
use cabinet_domain::link::SourceRange;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMarkdown {
    headings: Vec<MarkdownHeading>,
    wikilinks: Vec<ParsedWikilink>,
    asset_references: Vec<ParsedAssetReference>,
    external_links: Vec<ParsedExternalLink>,
    document_links: Vec<ParsedDocumentLink>,
}

impl ParsedMarkdown {
    pub fn new(
        headings: Vec<MarkdownHeading>,
        wikilinks: Vec<ParsedWikilink>,
        asset_references: Vec<ParsedAssetReference>,
    ) -> Self {
        Self {
            headings,
            wikilinks,
            asset_references,
            external_links: Vec::new(),
            document_links: Vec::new(),
        }
    }

    pub fn with_external_links(mut self, external_links: Vec<ParsedExternalLink>) -> Self {
        self.external_links = external_links;
        self
    }

    pub fn with_document_links(mut self, document_links: Vec<ParsedDocumentLink>) -> Self {
        self.document_links = document_links;
        self
    }

    pub fn headings(&self) -> &[MarkdownHeading] {
        &self.headings
    }

    pub fn wikilinks(&self) -> &[ParsedWikilink] {
        &self.wikilinks
    }

    pub fn asset_references(&self) -> &[ParsedAssetReference] {
        &self.asset_references
    }

    pub fn external_links(&self) -> &[ParsedExternalLink] {
        &self.external_links
    }

    pub fn document_links(&self) -> &[ParsedDocumentLink] {
        &self.document_links
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDocumentLink {
    target: String,
    label: String,
    source_range: SourceRange,
}

impl ParsedDocumentLink {
    pub fn new(
        target: &str,
        label: &str,
        source_range: SourceRange,
    ) -> Result<Self, MarkdownParserError> {
        let target = target.trim();
        if target.is_empty() {
            return Err(MarkdownParserError::EmptyDocumentLinkTarget);
        }
        let label = label.trim();
        if label.is_empty() {
            return Err(MarkdownParserError::EmptyDocumentLinkLabel);
        }
        Ok(Self {
            target: target.to_string(),
            label: label.to_string(),
            source_range,
        })
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn source_range(&self) -> SourceRange {
        self.source_range
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedExternalLink {
    target: String,
    label: String,
    source_range: SourceRange,
}

impl ParsedExternalLink {
    pub fn new(
        target: &str,
        label: &str,
        source_range: SourceRange,
    ) -> Result<Self, MarkdownParserError> {
        let target = target.trim();
        if target.is_empty() {
            return Err(MarkdownParserError::EmptyExternalLinkTarget);
        }
        let label = label.trim();
        if label.is_empty() {
            return Err(MarkdownParserError::EmptyExternalLinkLabel);
        }
        Ok(Self {
            target: target.to_string(),
            label: label.to_string(),
            source_range,
        })
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn source_range(&self) -> SourceRange {
        self.source_range
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownHeading {
    level: u8,
    text: String,
    source_range: SourceRange,
}

impl MarkdownHeading {
    pub fn new(
        level: u8,
        text: &str,
        source_range: SourceRange,
    ) -> Result<Self, MarkdownParserError> {
        if level == 0 || level > 6 {
            return Err(MarkdownParserError::InvalidHeadingLevel);
        }
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(MarkdownParserError::EmptyHeadingText);
        }
        Ok(Self {
            level,
            text: trimmed.to_string(),
            source_range,
        })
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn source_range(&self) -> SourceRange {
        self.source_range
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedWikilink {
    target: String,
    label: Option<String>,
    source_range: SourceRange,
}

impl ParsedWikilink {
    pub fn new(
        target: &str,
        label: Option<&str>,
        source_range: SourceRange,
    ) -> Result<Self, MarkdownParserError> {
        let target = target.trim();
        if target.is_empty() {
            return Err(MarkdownParserError::EmptyWikilinkTarget);
        }
        let label = label
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        Ok(Self {
            target: target.to_string(),
            label,
            source_range,
        })
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn source_range(&self) -> SourceRange {
        self.source_range
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAssetReference {
    asset_id: AssetId,
    label: String,
    source_range: SourceRange,
}

impl ParsedAssetReference {
    pub fn new(
        asset_id: AssetId,
        label: &str,
        source_range: SourceRange,
    ) -> Result<Self, MarkdownParserError> {
        let label = label.trim();
        if label.is_empty() {
            return Err(MarkdownParserError::EmptyAssetReferenceLabel);
        }
        Ok(Self {
            asset_id,
            label: label.to_string(),
            source_range,
        })
    }

    pub fn asset_id(&self) -> &AssetId {
        &self.asset_id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn source_range(&self) -> SourceRange {
        self.source_range
    }
}

pub trait MarkdownParser {
    fn parse(&self, body: &DocumentBody) -> Result<ParsedMarkdown, MarkdownParserError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownParserError {
    InvalidHeadingLevel,
    EmptyHeadingText,
    EmptyWikilinkTarget,
    EmptyAssetReferenceLabel,
    EmptyExternalLinkTarget,
    EmptyExternalLinkLabel,
    EmptyDocumentLinkTarget,
    EmptyDocumentLinkLabel,
    InvalidSourceRange,
}

impl MarkdownParserError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidHeadingLevel => "markdown_parser.invalid_heading_level",
            Self::EmptyHeadingText => "markdown_parser.empty_heading_text",
            Self::EmptyWikilinkTarget => "markdown_parser.empty_wikilink_target",
            Self::EmptyAssetReferenceLabel => "markdown_parser.empty_asset_reference_label",
            Self::EmptyExternalLinkTarget => "markdown_parser.empty_external_link_target",
            Self::EmptyExternalLinkLabel => "markdown_parser.empty_external_link_label",
            Self::EmptyDocumentLinkTarget => "markdown_parser.empty_document_link_target",
            Self::EmptyDocumentLinkLabel => "markdown_parser.empty_document_link_label",
            Self::InvalidSourceRange => "markdown_parser.invalid_source_range",
        }
    }
}
