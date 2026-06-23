use cabinet_domain::document::DocumentBody;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlDocument {
    html: String,
}

impl HtmlDocument {
    pub fn new(html: &str) -> Result<Self, HtmlRendererError> {
        if html.trim().is_empty() {
            return Err(HtmlRendererError::EmptyHtml);
        }
        Ok(Self {
            html: html.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.html
    }
}

pub trait HtmlRenderer {
    fn render(&self, body: &DocumentBody) -> Result<HtmlDocument, HtmlRendererError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlRendererError {
    EmptyHtml,
    RenderFailed,
}

impl HtmlRendererError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptyHtml => "html_renderer.empty_html",
            Self::RenderFailed => "html_renderer.render_failed",
        }
    }
}
