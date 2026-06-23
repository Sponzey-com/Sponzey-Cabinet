use cabinet_domain::document::DocumentBody;
use cabinet_ports::html_renderer::{HtmlDocument, HtmlRenderer, HtmlRendererError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalHtmlRenderer;

impl LocalHtmlRenderer {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for LocalHtmlRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlRenderer for LocalHtmlRenderer {
    fn render(&self, body: &DocumentBody) -> Result<HtmlDocument, HtmlRendererError> {
        let mut html = String::new();
        for line in body.as_str().lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((level, text)) = parse_heading(trimmed) {
                html.push_str(&format!("<h{level}>{}</h{level}>\n", escape_html(text)));
            } else {
                html.push_str(&format!("<p>{}</p>\n", escape_html(trimmed)));
            }
        }
        HtmlDocument::new(&html)
    }
}

fn parse_heading(line: &str) -> Option<(usize, &str)> {
    let level = line.bytes().take_while(|byte| *byte == b'#').count();
    if (1..=6).contains(&level) && line.as_bytes().get(level) == Some(&b' ') {
        Some((level, line[level + 1..].trim()))
    } else {
        None
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
