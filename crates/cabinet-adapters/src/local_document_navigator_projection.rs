use std::collections::HashSet;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_navigator::{
    DocumentNavigatorItem, DocumentNavigatorPage, DocumentNavigatorProjectionError,
    DocumentNavigatorProjectionPort, DocumentNavigatorProjectionQuery, NavigatorViewKind,
};

use crate::local_atomic_file::write_text_atomically;

const SCHEMA_HEADER: &str = "schema\t1";
const CAPACITY_MAX: usize = 50_000;

#[derive(Debug, Clone)]
pub struct LocalDocumentNavigatorProjectionStore {
    root: PathBuf,
    capacity: usize,
}

impl LocalDocumentNavigatorProjectionStore {
    pub fn new(root: PathBuf, capacity: usize) -> Result<Self, DocumentNavigatorProjectionError> {
        if capacity == 0 || capacity > CAPACITY_MAX {
            return Err(DocumentNavigatorProjectionError::InvalidQuery);
        }
        Ok(Self { root, capacity })
    }

    pub fn replace_workspace_items(
        &self,
        workspace_id: &WorkspaceId,
        items: Vec<DocumentNavigatorItem>,
    ) -> Result<(), DocumentNavigatorProjectionError> {
        let items = deduplicate_and_cap(items, self.capacity);
        write_text_atomically(&self.projection_path(workspace_id), encode_items(&items))
            .map(|_| ())
            .map_err(|_| DocumentNavigatorProjectionError::StorageUnavailable)
    }

    fn projection_path(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.root
            .join("navigator-projections")
            .join(format!("{}.snapshot", hex_encode(workspace_id.as_str())))
    }
}

impl DocumentNavigatorProjectionPort for LocalDocumentNavigatorProjectionStore {
    fn load_navigator_page(
        &self,
        workspace_id: &WorkspaceId,
        query: &DocumentNavigatorProjectionQuery,
    ) -> Result<DocumentNavigatorPage, DocumentNavigatorProjectionError> {
        let text = match fs::read_to_string(self.projection_path(workspace_id)) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(DocumentNavigatorPage::empty(false));
            }
            Err(_) => return Err(DocumentNavigatorProjectionError::StorageUnavailable),
        };
        let mut items = decode_items(&text)?;
        items.retain(|item| matches_view(item, query));
        if let Some(filter) = query.filter() {
            items.retain(|item| matches_filter(item, filter));
        }
        sort_items(&mut items, query.view());

        let total = items.len();
        let offset = usize::try_from(query.offset()).unwrap_or(usize::MAX);
        if offset >= total {
            return Ok(DocumentNavigatorPage::empty(false));
        }
        let end = offset.saturating_add(query.limit() as usize).min(total);
        let page = items[offset..end].to_vec();
        let next_offset = (end < total).then(|| u32::try_from(end).unwrap_or(u32::MAX));
        Ok(DocumentNavigatorPage::new(page, next_offset, false))
    }
}

fn deduplicate_and_cap(
    items: Vec<DocumentNavigatorItem>,
    capacity: usize,
) -> Vec<DocumentNavigatorItem> {
    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(item.document_id().to_string()))
        .take(capacity)
        .collect()
}

fn matches_view(item: &DocumentNavigatorItem, query: &DocumentNavigatorProjectionQuery) -> bool {
    match query.view() {
        NavigatorViewKind::Tree | NavigatorViewKind::Recent => true,
        NavigatorViewKind::Collection => query
            .view_key()
            .is_some_and(|key| item.collections().iter().any(|value| value == key)),
        NavigatorViewKind::Tag => query
            .view_key()
            .is_some_and(|key| item.tags().iter().any(|value| value == key)),
        NavigatorViewKind::Favorite => item.favorite(),
    }
}

fn matches_filter(item: &DocumentNavigatorItem, filter: &str) -> bool {
    item.title().to_lowercase().contains(filter) || item.path().to_lowercase().contains(filter)
}

fn sort_items(items: &mut [DocumentNavigatorItem], view: NavigatorViewKind) {
    if view == NavigatorViewKind::Recent {
        items.sort_by(|left, right| {
            left.recent_rank()
                .cmp(&right.recent_rank())
                .then_with(|| left.document_id().cmp(right.document_id()))
        });
    } else {
        items.sort_by(|left, right| {
            left.path()
                .cmp(right.path())
                .then_with(|| left.document_id().cmp(right.document_id()))
        });
    }
}

fn encode_items(items: &[DocumentNavigatorItem]) -> String {
    let mut lines = vec![SCHEMA_HEADER.to_string()];
    lines.extend(items.iter().map(|item| {
        format!(
            "item\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            hex_encode(item.document_id()),
            hex_encode(item.title()),
            hex_encode(item.path()),
            u8::from(item.favorite()),
            item.recent_rank(),
            encode_list(item.collections()),
            encode_list(item.tags()),
        )
    }));
    format!("{}\n", lines.join("\n"))
}

fn decode_items(
    text: &str,
) -> Result<Vec<DocumentNavigatorItem>, DocumentNavigatorProjectionError> {
    let mut lines = text.lines();
    if lines.next() != Some(SCHEMA_HEADER) {
        return Err(DocumentNavigatorProjectionError::CorruptedProjection);
    }
    lines
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            let [
                "item",
                id,
                title,
                path,
                favorite,
                recent_rank,
                collections,
                tags,
            ] = fields.as_slice()
            else {
                return Err(DocumentNavigatorProjectionError::CorruptedProjection);
            };
            let favorite = match *favorite {
                "0" => false,
                "1" => true,
                _ => return Err(DocumentNavigatorProjectionError::CorruptedProjection),
            };
            let recent_rank = recent_rank
                .parse::<u64>()
                .map_err(|_| DocumentNavigatorProjectionError::CorruptedProjection)?;
            DocumentNavigatorItem::new(
                DocumentId::new(&hex_decode(id)?)
                    .map_err(|_| DocumentNavigatorProjectionError::CorruptedProjection)?,
                DocumentTitle::new(&hex_decode(title)?)
                    .map_err(|_| DocumentNavigatorProjectionError::CorruptedProjection)?,
                DocumentPath::new(&hex_decode(path)?)
                    .map_err(|_| DocumentNavigatorProjectionError::CorruptedProjection)?,
                decode_list(collections)?,
                decode_list(tags)?,
                favorite,
                recent_rank,
            )
            .map_err(|_| DocumentNavigatorProjectionError::CorruptedProjection)
        })
        .collect()
}

fn encode_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| hex_encode(value))
        .collect::<Vec<_>>()
        .join(",")
}

fn decode_list(value: &str) -> Result<Vec<String>, DocumentNavigatorProjectionError> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    value.split(',').map(hex_decode).collect()
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> Result<String, DocumentNavigatorProjectionError> {
    if value.len() % 2 != 0 {
        return Err(DocumentNavigatorProjectionError::CorruptedProjection);
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)
                .map_err(|_| DocumentNavigatorProjectionError::CorruptedProjection)?;
            u8::from_str_radix(text, 16)
                .map_err(|_| DocumentNavigatorProjectionError::CorruptedProjection)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| DocumentNavigatorProjectionError::CorruptedProjection)
}
