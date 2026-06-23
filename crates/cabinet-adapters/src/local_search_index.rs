use std::cmp::Reverse;
use std::collections::HashMap;

use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::search_index::{
    SearchDocumentRecord, SearchIndex, SearchIndexError, SearchPage, SearchQuery, SearchResult,
};

#[derive(Debug, Default)]
pub struct LocalSearchIndex {
    records: HashMap<(String, String), SearchDocumentRecord>,
}

impl SearchIndex for LocalSearchIndex {
    fn upsert_document(
        &mut self,
        workspace_id: &WorkspaceId,
        record: SearchDocumentRecord,
    ) -> Result<(), SearchIndexError> {
        self.records.insert(
            (
                workspace_id.as_str().to_string(),
                record.document_id().as_str().to_string(),
            ),
            record,
        );
        Ok(())
    }

    fn delete_document(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<(), SearchIndexError> {
        self.records.remove(&(
            workspace_id.as_str().to_string(),
            document_id.as_str().to_string(),
        ));
        Ok(())
    }

    fn search(
        &self,
        workspace_id: &WorkspaceId,
        query: SearchQuery,
    ) -> Result<SearchPage, SearchIndexError> {
        let query_text = query.text().to_ascii_lowercase();
        let workspace = workspace_id.as_str();
        let mut scored = self
            .records
            .iter()
            .filter(|((record_workspace, _), _)| record_workspace == workspace)
            .filter_map(|(_, record)| {
                let score = score_record(record, &query_text);
                if score == 0 {
                    return None;
                }
                Some((score, record))
            })
            .collect::<Vec<_>>();

        scored.sort_by_key(|(score, record)| {
            (
                Reverse(*score),
                record.title().as_str().to_ascii_lowercase(),
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
                    &snippet(record, &query_text),
                    score,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(SearchPage::new(results))
    }
}

fn score_record(record: &SearchDocumentRecord, query: &str) -> u32 {
    let title = record.title().as_str().to_ascii_lowercase();
    let path = record.path().as_str().to_ascii_lowercase();
    let body = record.body().as_str().to_ascii_lowercase();
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
        .find(|line| line.to_ascii_lowercase().contains(query))
        .or_else(|| {
            if record.title().as_str().to_ascii_lowercase().contains(query) {
                Some(record.title().as_str())
            } else if record.path().as_str().to_ascii_lowercase().contains(query) {
                Some(record.path().as_str())
            } else {
                None
            }
        })
        .unwrap_or_else(|| record.body().as_str())
        .to_string()
}
