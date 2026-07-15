use std::collections::{BTreeMap, HashSet};

use cabinet_domain::document::DocumentId;
use cabinet_domain::link::{Backlink, DocumentLink};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::link_index::{
    BacklinkPage, BacklinkPageReader, BacklinkPageRequest, LinkIndex, LinkIndexError,
    LinkProjectionRecord,
};

#[derive(Debug, Default)]
pub struct LocalLinkIndex {
    records: BTreeMap<(String, String), LinkProjectionRecord>,
}

impl BacklinkPageReader for LocalLinkIndex {
    fn list_backlinks_page(
        &self,
        workspace_id: &WorkspaceId,
        target_document_id: &DocumentId,
        request: BacklinkPageRequest,
    ) -> Result<BacklinkPage, LinkIndexError> {
        let workspace = workspace_id.as_str();
        let mut matched = 0;
        let mut records = Vec::with_capacity(request.limit() + 1);
        'sources: for ((record_workspace, _), record) in &self.records {
            if record_workspace != workspace {
                continue;
            }
            for backlink in record.backlinks() {
                if backlink.target_document_id() != target_document_id {
                    continue;
                }
                if matched < request.offset() {
                    matched += 1;
                    continue;
                }
                records.push(backlink.clone());
                matched += 1;
                if records.len() > request.limit() {
                    break 'sources;
                }
            }
        }
        let has_more = records.len() > request.limit();
        records.truncate(request.limit());
        let next_offset = has_more.then_some(request.offset() + records.len());
        Ok(BacklinkPage::new(records, next_offset))
    }
}

impl LinkIndex for LocalLinkIndex {
    fn replace_document_links(
        &mut self,
        workspace_id: &WorkspaceId,
        record: LinkProjectionRecord,
    ) -> Result<(), LinkIndexError> {
        self.records.insert(
            (
                workspace_id.as_str().to_string(),
                record.source_document_id().as_str().to_string(),
            ),
            record,
        );
        Ok(())
    }

    fn delete_document_links(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<(), LinkIndexError> {
        self.records.remove(&(
            workspace_id.as_str().to_string(),
            document_id.as_str().to_string(),
        ));
        Ok(())
    }

    fn get_document_links(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<LinkProjectionRecord>, LinkIndexError> {
        Ok(self
            .records
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
            ))
            .cloned())
    }

    fn list_backlinks(
        &self,
        workspace_id: &WorkspaceId,
        target_document_id: &DocumentId,
    ) -> Result<Vec<Backlink>, LinkIndexError> {
        let workspace = workspace_id.as_str();
        let backlinks = self
            .records
            .iter()
            .filter(|((record_workspace, _), _)| record_workspace == workspace)
            .flat_map(|(_, record)| record.backlinks().iter())
            .filter(|backlink| backlink.target_document_id() == target_document_id)
            .cloned()
            .collect();
        Ok(backlinks)
    }

    fn list_unresolved_links(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<DocumentLink>, LinkIndexError> {
        let workspace = workspace_id.as_str();
        let links = self
            .records
            .iter()
            .filter(|((record_workspace, _), _)| record_workspace == workspace)
            .flat_map(|(_, record)| record.unresolved_links().iter())
            .cloned()
            .collect();
        Ok(links)
    }

    fn list_orphan_documents(
        &self,
        workspace_id: &WorkspaceId,
        document_ids: &[DocumentId],
    ) -> Result<Vec<DocumentId>, LinkIndexError> {
        let workspace = workspace_id.as_str();
        let incoming_targets: HashSet<String> = self
            .records
            .iter()
            .filter(|((record_workspace, _), _)| record_workspace == workspace)
            .flat_map(|(_, record)| record.backlinks().iter())
            .map(|backlink| backlink.target_document_id().as_str().to_string())
            .collect();
        let orphans = document_ids
            .iter()
            .filter(|document_id| !incoming_targets.contains(document_id.as_str()))
            .cloned()
            .collect();
        Ok(orphans)
    }
}
