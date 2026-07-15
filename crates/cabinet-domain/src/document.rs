const DOCUMENT_TITLE_MAX_LEN: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentMetadata {
    id: DocumentId,
    title: DocumentTitle,
    path: DocumentPath,
    slug: DocumentSlug,
}

impl DocumentMetadata {
    pub fn new(
        id: DocumentId,
        title: DocumentTitle,
        path: DocumentPath,
    ) -> Result<Self, DocumentError> {
        let slug = DocumentSlug::from_title(&title)?;
        Ok(Self {
            id,
            title,
            path,
            slug,
        })
    }

    pub fn with_title(&self, title: DocumentTitle) -> Result<Self, DocumentError> {
        Self::new(self.id.clone(), title, self.path.clone())
    }

    pub fn id(&self) -> &DocumentId {
        &self.id
    }

    pub fn title(&self) -> &DocumentTitle {
        &self.title
    }

    pub fn path(&self) -> &DocumentPath {
        &self.path
    }

    pub fn slug(&self) -> &DocumentSlug {
        &self.slug
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentId {
    value: String,
}

impl DocumentId {
    pub fn new(value: &str) -> Result<Self, DocumentError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(DocumentError::EmptyId);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTitle {
    value: String,
}

impl DocumentTitle {
    pub fn new(value: &str) -> Result<Self, DocumentError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(DocumentError::EmptyTitle);
        }
        if trimmed.chars().count() > DOCUMENT_TITLE_MAX_LEN {
            return Err(DocumentError::TitleTooLong {
                max: DOCUMENT_TITLE_MAX_LEN,
            });
        }
        if trimmed.chars().any(char::is_control) {
            return Err(DocumentError::InvalidTitleCharacter);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn from_markdown_body(body: &DocumentBody) -> Self {
        let first_line = body.as_str().lines().next().unwrap_or_default().trim();
        let without_heading = first_line
            .trim_start_matches('#')
            .trim()
            .trim_end_matches('#')
            .trim();
        let sanitized = without_heading
            .chars()
            .map(|character| if character.is_control() { ' ' } else { character })
            .collect::<String>();
        let bounded = sanitized
            .trim()
            .chars()
            .take(DOCUMENT_TITLE_MAX_LEN)
            .collect::<String>();
        let value = if bounded.chars().any(char::is_alphanumeric) {
            bounded
        } else {
            "제목 없는 문서".to_string()
        };
        Self { value }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentPath {
    value: String,
}

impl DocumentPath {
    pub fn new(value: &str) -> Result<Self, DocumentError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(DocumentError::EmptyPathSegment);
        }
        if trimmed.starts_with('/') || trimmed.contains('\\') || trimmed.contains(':') {
            return Err(DocumentError::AbsoluteDocumentPath);
        }
        if !trimmed.to_ascii_lowercase().ends_with(".md") {
            return Err(DocumentError::InvalidDocumentExtension);
        }

        for segment in trimmed.split('/') {
            if segment.is_empty() {
                return Err(DocumentError::EmptyPathSegment);
            }
            if segment == "." || segment == ".." {
                return Err(DocumentError::TraversalPathSegment);
            }
            if segment.chars().any(char::is_control) {
                return Err(DocumentError::InvalidPathCharacter);
            }
        }

        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSlug {
    value: String,
}

impl DocumentSlug {
    pub fn from_title(title: &DocumentTitle) -> Result<Self, DocumentError> {
        let mut value = String::new();
        let mut previous_was_separator = false;

        for character in title.as_str().chars() {
            if character.is_alphanumeric() {
                for lowercase in character.to_lowercase() {
                    value.push(lowercase);
                }
                previous_was_separator = false;
            } else if !previous_was_separator && !value.is_empty() {
                value.push('-');
                previous_was_separator = true;
            }
        }

        while value.ends_with('-') {
            value.pop();
        }

        if value.is_empty() {
            return Err(DocumentError::EmptySlug);
        }

        Ok(Self { value })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentBody {
    value: String,
}

impl DocumentBody {
    pub fn new(value: &str, policy: DocumentBodyPolicy) -> Result<Self, DocumentError> {
        let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
        if normalized.len() > policy.max_bytes {
            return Err(DocumentError::BodyTooLarge {
                max_bytes: policy.max_bytes,
            });
        }
        Ok(Self { value: normalized })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentBodyPolicy {
    max_bytes: usize,
}

impl DocumentBodyPolicy {
    pub fn new(max_bytes: usize) -> Result<Self, DocumentError> {
        if max_bytes == 0 {
            return Err(DocumentError::InvalidBodyPolicy);
        }
        Ok(Self { max_bytes })
    }

    pub fn max_bytes(self) -> usize {
        self.max_bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentLifecycleState {
    Draft,
    Saved,
    Editing,
    Archived,
    Deleted,
    Restored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentLifecycleEvent {
    Create,
    Save,
    StartEdit,
    Archive,
    Delete,
    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentLifecycleTransition {
    pub previous_state: DocumentLifecycleState,
    pub event: DocumentLifecycleEvent,
    pub next_state: DocumentLifecycleState,
}

pub fn transition_document_lifecycle(
    state: DocumentLifecycleState,
    event: DocumentLifecycleEvent,
) -> Result<DocumentLifecycleTransition, DocumentError> {
    let next_state = match (state, event) {
        (DocumentLifecycleState::Draft, DocumentLifecycleEvent::Create) => {
            DocumentLifecycleState::Draft
        }
        (DocumentLifecycleState::Draft, DocumentLifecycleEvent::Save) => {
            DocumentLifecycleState::Saved
        }
        (DocumentLifecycleState::Saved, DocumentLifecycleEvent::StartEdit) => {
            DocumentLifecycleState::Editing
        }
        (DocumentLifecycleState::Editing, DocumentLifecycleEvent::Save) => {
            DocumentLifecycleState::Saved
        }
        (
            DocumentLifecycleState::Saved | DocumentLifecycleState::Restored,
            DocumentLifecycleEvent::Archive,
        ) => DocumentLifecycleState::Archived,
        (DocumentLifecycleState::Archived, DocumentLifecycleEvent::Restore) => {
            DocumentLifecycleState::Restored
        }
        (
            DocumentLifecycleState::Saved
            | DocumentLifecycleState::Editing
            | DocumentLifecycleState::Archived
            | DocumentLifecycleState::Restored,
            DocumentLifecycleEvent::Delete,
        ) => DocumentLifecycleState::Deleted,
        (DocumentLifecycleState::Deleted, DocumentLifecycleEvent::Restore) => {
            DocumentLifecycleState::Restored
        }
        _ => {
            return Err(DocumentError::InvalidLifecycleTransition { state, event });
        }
    };

    Ok(DocumentLifecycleTransition {
        previous_state: state,
        event,
        next_state,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentError {
    EmptyId,
    EmptyTitle,
    TitleTooLong {
        max: usize,
    },
    InvalidTitleCharacter,
    AbsoluteDocumentPath,
    EmptyPathSegment,
    TraversalPathSegment,
    InvalidPathCharacter,
    InvalidDocumentExtension,
    EmptySlug,
    BodyTooLarge {
        max_bytes: usize,
    },
    InvalidBodyPolicy,
    InvalidLifecycleTransition {
        state: DocumentLifecycleState,
        event: DocumentLifecycleEvent,
    },
}
