use std::cmp::Reverse;
use std::collections::HashMap;

use cabinet_domain::asset::{AssetCatalogRecord, AssetId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_search_index::{
    AssetSearchError, AssetSearchIndex, AssetSearchPage, AssetSearchQuery, AssetSearchResult,
};

#[derive(Debug, Default)]
pub struct LocalAssetSearchIndex {
    records: HashMap<(String, String), AssetCatalogRecord>,
}

impl LocalAssetSearchIndex {
    pub fn upsert_asset(&mut self, workspace_id: &WorkspaceId, record: AssetCatalogRecord) {
        self.records.insert(
            (
                workspace_id.as_str().to_string(),
                record.metadata().id().as_str().to_string(),
            ),
            record,
        );
    }

    pub fn delete_asset(&mut self, workspace_id: &WorkspaceId, asset_id: &AssetId) {
        self.records.remove(&(
            workspace_id.as_str().to_string(),
            asset_id.as_str().to_string(),
        ));
    }
}

impl AssetSearchIndex for LocalAssetSearchIndex {
    fn search_assets(
        &self,
        workspace_id: &WorkspaceId,
        query: AssetSearchQuery,
    ) -> Result<AssetSearchPage, AssetSearchError> {
        let query_text = query.text().to_ascii_lowercase();
        let workspace = workspace_id.as_str();
        let mut scored = self
            .records
            .iter()
            .filter(|((record_workspace, _), _)| record_workspace == workspace)
            .filter_map(|(_, record)| {
                let score = score_record(record, &query_text);
                (score > 0).then_some((score, record))
            })
            .collect::<Vec<_>>();

        scored.sort_by_key(|(score, record)| {
            (
                Reverse(*score),
                record.metadata().file_name().as_str().to_ascii_lowercase(),
                record.metadata().id().as_str().to_string(),
            )
        });

        let results = scored
            .into_iter()
            .take(query.limit())
            .map(|(score, record)| {
                AssetSearchResult::new(
                    record.metadata().id().clone(),
                    record.metadata().file_name().clone(),
                    record.metadata().media_type().clone(),
                    record.metadata().byte_size(),
                    score,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(AssetSearchPage::new(results))
    }
}

fn score_record(record: &AssetCatalogRecord, query: &str) -> u32 {
    let file_name = record.metadata().file_name().as_str().to_ascii_lowercase();
    let media_type = record.metadata().media_type().as_str().to_ascii_lowercase();
    (count_matches(&file_name, query) * 3 + count_matches(&media_type, query)) as u32
}

fn count_matches(value: &str, query: &str) -> usize {
    value.matches(query).count()
}
