use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;

const QUERY_LIMIT_MAX: u16 = 100;
const VIEW_KEY_MAX: usize = 64;
const FILTER_MAX: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigatorViewKind {
    Tree,
    Collection,
    Tag,
    Recent,
    Favorite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentNavigatorItem {
    document_id: DocumentId,
    title: DocumentTitle,
    path: DocumentPath,
    collections: Vec<String>,
    tags: Vec<String>,
    favorite: bool,
    recent_rank: u64,
}

impl DocumentNavigatorItem {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        document_id: DocumentId,
        title: DocumentTitle,
        path: DocumentPath,
        collections: Vec<String>,
        tags: Vec<String>,
        favorite: bool,
        recent_rank: u64,
    ) -> Result<Self, DocumentNavigatorProjectionError> {
        Ok(Self {
            document_id,
            title,
            path,
            collections: normalize_keys(collections)?,
            tags: normalize_keys(tags)?,
            favorite,
            recent_rank,
        })
    }

    pub fn document_id(&self) -> &str {
        self.document_id.as_str()
    }

    pub fn title(&self) -> &str {
        self.title.as_str()
    }

    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    pub fn collections(&self) -> &[String] {
        &self.collections
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub const fn favorite(&self) -> bool {
        self.favorite
    }

    pub const fn recent_rank(&self) -> u64 {
        self.recent_rank
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentNavigatorProjectionQuery {
    view: NavigatorViewKind,
    view_key: Option<String>,
    filter: Option<String>,
    offset: u32,
    limit: u16,
}

impl DocumentNavigatorProjectionQuery {
    pub fn new(
        view: NavigatorViewKind,
        view_key: Option<&str>,
        filter: Option<&str>,
        offset: u32,
        limit: u16,
    ) -> Result<Self, DocumentNavigatorProjectionError> {
        if limit == 0 || limit > QUERY_LIMIT_MAX {
            return Err(DocumentNavigatorProjectionError::InvalidQuery);
        }
        let view_key = view_key.map(normalize_view_key).transpose()?;
        let requires_key = matches!(view, NavigatorViewKind::Collection | NavigatorViewKind::Tag);
        if requires_key != view_key.is_some() {
            return Err(DocumentNavigatorProjectionError::InvalidQuery);
        }
        let filter = filter
            .map(normalize_filter)
            .transpose()?
            .filter(|value| !value.is_empty());
        Ok(Self {
            view,
            view_key,
            filter,
            offset,
            limit,
        })
    }

    pub const fn view(&self) -> NavigatorViewKind {
        self.view
    }

    pub fn view_key(&self) -> Option<&str> {
        self.view_key.as_deref()
    }

    pub fn filter(&self) -> Option<&str> {
        self.filter.as_deref()
    }

    pub const fn offset(&self) -> u32 {
        self.offset
    }

    pub const fn limit(&self) -> u16 {
        self.limit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentNavigatorPage {
    items: Vec<DocumentNavigatorItem>,
    next_offset: Option<u32>,
    degraded: bool,
}

impl DocumentNavigatorPage {
    pub fn new(
        items: Vec<DocumentNavigatorItem>,
        next_offset: Option<u32>,
        degraded: bool,
    ) -> Self {
        Self {
            items,
            next_offset,
            degraded,
        }
    }

    pub fn empty(degraded: bool) -> Self {
        Self::new(Vec::new(), None, degraded)
    }

    pub fn items(&self) -> &[DocumentNavigatorItem] {
        &self.items
    }

    pub const fn next_offset(&self) -> Option<u32> {
        self.next_offset
    }

    pub const fn degraded(&self) -> bool {
        self.degraded
    }
}

pub trait DocumentNavigatorProjectionPort {
    fn load_navigator_page(
        &self,
        workspace_id: &WorkspaceId,
        query: &DocumentNavigatorProjectionQuery,
    ) -> Result<DocumentNavigatorPage, DocumentNavigatorProjectionError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentNavigatorProjectionError {
    InvalidQuery,
    InvalidProjectionItem,
    StorageUnavailable,
    CorruptedProjection,
}

impl DocumentNavigatorProjectionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidQuery => "document_navigator_projection.invalid_query",
            Self::InvalidProjectionItem => "document_navigator_projection.invalid_item",
            Self::StorageUnavailable => "document_navigator_projection.storage_unavailable",
            Self::CorruptedProjection => "document_navigator_projection.corrupted",
        }
    }
}

fn normalize_keys(values: Vec<String>) -> Result<Vec<String>, DocumentNavigatorProjectionError> {
    let mut normalized = values
        .into_iter()
        .map(|value| normalize_view_key(&value))
        .collect::<Result<Vec<_>, _>>()?;
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn normalize_view_key(value: &str) -> Result<String, DocumentNavigatorProjectionError> {
    normalize_text(value, VIEW_KEY_MAX, false)
}

fn normalize_filter(value: &str) -> Result<String, DocumentNavigatorProjectionError> {
    normalize_text(value, FILTER_MAX, true)
}

fn normalize_text(
    value: &str,
    max_len: usize,
    allow_empty: bool,
) -> Result<String, DocumentNavigatorProjectionError> {
    let normalized = value.trim().to_lowercase();
    if (!allow_empty && normalized.is_empty())
        || normalized.chars().count() > max_len
        || normalized.chars().any(char::is_control)
    {
        return Err(DocumentNavigatorProjectionError::InvalidQuery);
    }
    Ok(normalized)
}
