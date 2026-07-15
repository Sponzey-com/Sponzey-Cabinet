use crate::local_atomic_file::write_text_atomically;
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentPath, DocumentTitle,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::search_index::{
    SearchDocumentRecord, SearchIndex, SearchIndexError, SearchPage, SearchQuery, SearchResult,
};
use std::{cmp::Reverse, fs, io::ErrorKind, path::PathBuf};

const HEADER: &str = "schema\t1";

#[derive(Debug, Clone)]
pub struct DurableLocalSearchIndex {
    root: PathBuf,
    body_policy: DocumentBodyPolicy,
}

impl DurableLocalSearchIndex {
    pub fn new(root: PathBuf, body_policy: DocumentBodyPolicy) -> Self {
        Self { root, body_policy }
    }

    fn path(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.root
            .join("search-projections")
            .join(format!("{}.snapshot", hex(workspace_id.as_str())))
    }

    fn records(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<SearchDocumentRecord>, SearchIndexError> {
        match fs::read_to_string(self.path(workspace_id)) {
            Ok(content) => decode(&content, self.body_policy),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Vec::new()),
            Err(_) => Err(SearchIndexError::StorageUnavailable),
        }
    }

    fn replace(
        &self,
        workspace_id: &WorkspaceId,
        records: &[SearchDocumentRecord],
    ) -> Result<(), SearchIndexError> {
        write_text_atomically(&self.path(workspace_id), encode(records))
            .map(|_| ())
            .map_err(|_| SearchIndexError::StorageUnavailable)
    }
}

impl SearchIndex for DurableLocalSearchIndex {
    fn upsert_document(
        &mut self,
        workspace_id: &WorkspaceId,
        record: SearchDocumentRecord,
    ) -> Result<(), SearchIndexError> {
        let mut records = self.records(workspace_id)?;
        records.retain(|current| current.document_id() != record.document_id());
        records.push(record);
        records.sort_by(|left, right| {
            left.document_id()
                .as_str()
                .cmp(right.document_id().as_str())
        });
        self.replace(workspace_id, &records)
    }

    fn delete_document(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<(), SearchIndexError> {
        let mut records = self.records(workspace_id)?;
        records.retain(|record| record.document_id() != document_id);
        self.replace(workspace_id, &records)
    }

    fn search(
        &self,
        workspace_id: &WorkspaceId,
        query: SearchQuery,
    ) -> Result<SearchPage, SearchIndexError> {
        let query_text = query.text().to_lowercase();
        let mut scored = self
            .records(workspace_id)?
            .into_iter()
            .filter_map(|record| {
                let score = score_record(&record, &query_text);
                (score > 0).then_some((score, record))
            })
            .collect::<Vec<_>>();
        scored.sort_by_key(|(score, record)| {
            (
                Reverse(*score),
                record.title().as_str().to_lowercase(),
                record.document_id().as_str().to_string(),
            )
        });
        let results = scored
            .into_iter()
            .take(query.limit())
            .map(|(score, record)| {
                SearchResult::new(
                    record.document_id().clone(),
                    record.title().clone(),
                    record.path().clone(),
                    &snippet(&record, &query_text),
                    score,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SearchPage::new(results))
    }
}

fn encode(records: &[SearchDocumentRecord]) -> String {
    let payload = records
        .iter()
        .map(|record| {
            format!(
                "record\t{}\t{}\t{}\t{}",
                hex(record.document_id().as_str()),
                hex(record.title().as_str()),
                hex(record.path().as_str()),
                hex(record.body().as_str())
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

fn decode(
    content: &str,
    body_policy: DocumentBodyPolicy,
) -> Result<Vec<SearchDocumentRecord>, SearchIndexError> {
    let mut lines = content.lines();
    if lines.next() != Some(HEADER) {
        return Err(SearchIndexError::CorruptedIndex);
    }
    let expected = lines
        .next()
        .and_then(|line| line.strip_prefix("checksum\t"))
        .and_then(|value| u64::from_str_radix(value, 16).ok())
        .ok_or(SearchIndexError::CorruptedIndex)?;
    let remaining = lines.collect::<Vec<_>>();
    let payload = if remaining.is_empty() {
        String::new()
    } else {
        format!("{}\n", remaining.join("\n"))
    };
    if checksum(payload.as_bytes()) != expected {
        return Err(SearchIndexError::CorruptedIndex);
    }
    remaining
        .into_iter()
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            let ["record", id, title, path, body] = fields.as_slice() else {
                return Err(SearchIndexError::CorruptedIndex);
            };
            Ok(SearchDocumentRecord::new(
                DocumentId::new(&unhex(id)?).map_err(|_| SearchIndexError::CorruptedIndex)?,
                DocumentTitle::new(&unhex(title)?).map_err(|_| SearchIndexError::CorruptedIndex)?,
                DocumentPath::new(&unhex(path)?).map_err(|_| SearchIndexError::CorruptedIndex)?,
                DocumentBody::new(&unhex(body)?, body_policy)
                    .map_err(|_| SearchIndexError::CorruptedIndex)?,
            ))
        })
        .collect()
}

fn score_record(record: &SearchDocumentRecord, query: &str) -> u32 {
    let title = record.title().as_str().to_lowercase();
    let path = record.path().as_str().to_lowercase();
    let body = record.body().as_str().to_lowercase();
    (count_matches(&title, query) * 3
        + count_matches(&path, query) * 2
        + count_matches(&body, query)) as u32
}

fn count_matches(value: &str, query: &str) -> usize {
    value.matches(query).count()
}

fn snippet(record: &SearchDocumentRecord, query: &str) -> String {
    record
        .body()
        .as_str()
        .lines()
        .find(|line| line.to_lowercase().contains(query))
        .or_else(|| {
            if record.title().as_str().to_lowercase().contains(query) {
                Some(record.title().as_str())
            } else if record.path().as_str().to_lowercase().contains(query) {
                Some(record.path().as_str())
            } else {
                None
            }
        })
        .unwrap_or_else(|| record.body().as_str())
        .to_string()
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

fn unhex(value: &str) -> Result<String, SearchIndexError> {
    if value.len() % 2 != 0 {
        return Err(SearchIndexError::CorruptedIndex);
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            std::str::from_utf8(pair)
                .ok()
                .and_then(|text| u8::from_str_radix(text, 16).ok())
                .ok_or(SearchIndexError::CorruptedIndex)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| SearchIndexError::CorruptedIndex)
}
