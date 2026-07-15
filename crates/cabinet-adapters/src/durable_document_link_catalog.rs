use crate::local_atomic_file::write_text_atomically;
use cabinet_domain::document::{DocumentId, DocumentPath, DocumentSlug, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_link_catalog::{
    DocumentLinkCatalog, DocumentLinkCatalogError, DocumentLinkCatalogRecord,
};
use cabinet_ports::link_target_resolver::{
    DocumentLinkTargetResolver, LinkTargetResolution, LinkTargetResolverError,
    ResolvedDocumentLinkTarget,
};
use std::{fs, io::ErrorKind, path::PathBuf};

const HEADER: &str = "schema\t1";

#[derive(Debug, Clone)]
pub struct DurableDocumentLinkCatalog {
    root: PathBuf,
}

impl DurableDocumentLinkCatalog {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn path(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.root
            .join("document-link-catalog")
            .join(format!("{}.snapshot", hex(workspace_id.as_str())))
    }
}

impl DocumentLinkCatalog for DurableDocumentLinkCatalog {
    fn upsert(
        &mut self,
        workspace_id: &WorkspaceId,
        record: DocumentLinkCatalogRecord,
    ) -> Result<(), DocumentLinkCatalogError> {
        let mut records = self.list(workspace_id)?;
        records.retain(|current| current.document_id() != record.document_id());
        records.push(record);
        records.sort_by(|left, right| {
            left.document_id()
                .as_str()
                .cmp(right.document_id().as_str())
        });
        write_text_atomically(&self.path(workspace_id), encode(&records))
            .map(|_| ())
            .map_err(|_| DocumentLinkCatalogError::StorageUnavailable)
    }

    fn remove(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<bool, DocumentLinkCatalogError> {
        let mut records = self.list(workspace_id)?;
        let previous_len = records.len();
        records.retain(|record| record.document_id() != document_id);
        if records.len() == previous_len {
            return Ok(false);
        }
        write_text_atomically(&self.path(workspace_id), encode(&records))
            .map(|_| true)
            .map_err(|_| DocumentLinkCatalogError::StorageUnavailable)
    }

    fn list(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<DocumentLinkCatalogRecord>, DocumentLinkCatalogError> {
        match fs::read_to_string(self.path(workspace_id)) {
            Ok(content) => decode(&content),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Vec::new()),
            Err(_) => Err(DocumentLinkCatalogError::StorageUnavailable),
        }
    }
}

impl DocumentLinkTargetResolver for DurableDocumentLinkCatalog {
    fn resolve(
        &self,
        workspace_id: &WorkspaceId,
        target: &str,
    ) -> Result<LinkTargetResolution, LinkTargetResolverError> {
        let target = target.trim();
        if target.is_empty() {
            return Err(LinkTargetResolverError::InvalidTarget);
        }
        let records = self
            .list(workspace_id)
            .map_err(|_| LinkTargetResolverError::Unavailable)?;
        let target_title = DocumentTitle::new(target).ok();
        let target_slug = target_title
            .as_ref()
            .and_then(|title| DocumentSlug::from_title(title).ok());
        let normalized_title = target.to_lowercase();
        let mut matches = records
            .into_iter()
            .filter(|record| {
                record.path().as_str() == target
                    || record.title().as_str().to_lowercase() == normalized_title
                    || target_slug
                        .as_ref()
                        .is_some_and(|slug| slug == record.slug())
            })
            .collect::<Vec<_>>();
        matches.dedup_by(|left, right| left.document_id() == right.document_id());
        match matches.as_slice() {
            [] => target_slug
                .map(LinkTargetResolution::Unresolved)
                .ok_or(LinkTargetResolverError::InvalidTarget),
            [record] => Ok(LinkTargetResolution::Resolved(
                ResolvedDocumentLinkTarget::new(
                    record.document_id().clone(),
                    record.path().clone(),
                ),
            )),
            _ => Err(LinkTargetResolverError::Ambiguous),
        }
    }
}

fn encode(records: &[DocumentLinkCatalogRecord]) -> String {
    let payload = records
        .iter()
        .map(|record| {
            format!(
                "record\t{}\t{}\t{}",
                hex(record.document_id().as_str()),
                hex(record.title().as_str()),
                hex(record.path().as_str())
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let payload = if payload.is_empty() {
        String::new()
    } else {
        format!("{payload}\n")
    };
    format!(
        "{HEADER}\nchecksum\t{:016x}\n{payload}",
        checksum(payload.as_bytes())
    )
}

fn decode(content: &str) -> Result<Vec<DocumentLinkCatalogRecord>, DocumentLinkCatalogError> {
    let mut lines = content.lines();
    if lines.next() != Some(HEADER) {
        return Err(DocumentLinkCatalogError::CorruptedCatalog);
    }
    let expected = lines
        .next()
        .and_then(|line| line.strip_prefix("checksum\t"))
        .and_then(|value| u64::from_str_radix(value, 16).ok())
        .ok_or(DocumentLinkCatalogError::CorruptedCatalog)?;
    let remaining = lines.collect::<Vec<_>>();
    let payload = if remaining.is_empty() {
        String::new()
    } else {
        format!("{}\n", remaining.join("\n"))
    };
    if checksum(payload.as_bytes()) != expected {
        return Err(DocumentLinkCatalogError::CorruptedCatalog);
    }
    remaining
        .into_iter()
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            let ["record", id, title, path] = fields.as_slice() else {
                return Err(DocumentLinkCatalogError::CorruptedCatalog);
            };
            DocumentLinkCatalogRecord::new(
                DocumentId::new(&unhex(id)?)
                    .map_err(|_| DocumentLinkCatalogError::CorruptedCatalog)?,
                DocumentTitle::new(&unhex(title)?)
                    .map_err(|_| DocumentLinkCatalogError::CorruptedCatalog)?,
                DocumentPath::new(&unhex(path)?)
                    .map_err(|_| DocumentLinkCatalogError::CorruptedCatalog)?,
            )
            .map_err(|_| DocumentLinkCatalogError::CorruptedCatalog)
        })
        .collect()
}

fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn unhex(value: &str) -> Result<String, DocumentLinkCatalogError> {
    if value.len() % 2 != 0 {
        return Err(DocumentLinkCatalogError::CorruptedCatalog);
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            std::str::from_utf8(pair)
                .ok()
                .and_then(|text| u8::from_str_radix(text, 16).ok())
                .ok_or(DocumentLinkCatalogError::CorruptedCatalog)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| DocumentLinkCatalogError::CorruptedCatalog)
}
