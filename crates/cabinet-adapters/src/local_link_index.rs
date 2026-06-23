use std::collections::{HashMap, HashSet};

use cabinet_domain::document::DocumentId;
use cabinet_domain::link::{Backlink, DocumentLink};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::link_index::{LinkIndex, LinkIndexError, LinkProjectionRecord};

#[derive(Debug, Default)]
pub struct LocalLinkIndex {
    records: HashMap<(String, String), LinkProjectionRecord>,
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
