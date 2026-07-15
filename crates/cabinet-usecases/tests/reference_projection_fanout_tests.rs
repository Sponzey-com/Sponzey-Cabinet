use cabinet_domain::document::{DocumentId, DocumentSlug, DocumentTitle};
use cabinet_domain::link::{Backlink, DocumentLink, LinkTarget, SourceRange};
use cabinet_domain::projection_work::{ProjectionWork, ProjectionWorkIdentity};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::link_index::{LinkIndex, LinkIndexError, LinkProjectionRecord};
use cabinet_ports::projection_work::{
    ProjectionEnqueueOutcome, ProjectionWorkRepository, ProjectionWorkRepositoryError,
};
use cabinet_usecases::document::DocumentChangeEvent;
use cabinet_usecases::reference_projection_fanout::{
    ReferenceFanoutOutcome, ReindexReferenceDependentsUsecase,
};

#[test]
fn rename_reindexes_union_of_resolved_and_newly_resolvable_sources_once() {
    let links = Links {
        backlinks: vec![backlink("source-1"), backlink("source-1")],
        unresolved: vec![
            unresolved("source-1", "Renamed"),
            unresolved("source-2", "Renamed"),
            unresolved("unrelated", "Other"),
        ],
    };
    let mut work = Work::default();
    let output = ReindexReferenceDependentsUsecase::new()
        .execute(&renamed(), &links, &Pointer, &mut work)
        .unwrap();

    assert_eq!(
        output,
        ReferenceFanoutOutcome::Applied {
            affected_documents: 2,
            enqueued: 6,
            reset: 0,
            already_active: 0,
        }
    );
    let mut documents = work
        .enqueued
        .iter()
        .map(|item| item.identity().document_id().as_str())
        .collect::<Vec<_>>();
    documents.sort();
    assert_eq!(
        documents,
        [
            "source-1", "source-1", "source-1", "source-2", "source-2", "source-2"
        ]
    );
}

#[test]
fn create_uses_matching_unresolved_and_delete_uses_resolved_backlinks() {
    let links = Links {
        backlinks: vec![backlink("resolved-source")],
        unresolved: vec![unresolved("unresolved-source", "Target")],
    };
    let mut created_work = Work::default();
    let created = ReindexReferenceDependentsUsecase::new()
        .execute(&created(), &links, &Pointer, &mut created_work)
        .unwrap();
    assert_eq!(created.affected_documents(), 1);
    assert!(
        created_work
            .enqueued
            .iter()
            .all(|work| { work.identity().document_id().as_str() == "unresolved-source" })
    );

    let mut deleted_work = Work::default();
    let deleted = ReindexReferenceDependentsUsecase::new()
        .execute(&deleted(), &links, &Pointer, &mut deleted_work)
        .unwrap();
    assert_eq!(deleted.affected_documents(), 1);
    assert!(
        deleted_work
            .enqueued
            .iter()
            .all(|work| { work.identity().document_id().as_str() == "resolved-source" })
    );
}

struct Links {
    backlinks: Vec<Backlink>,
    unresolved: Vec<DocumentLink>,
}

impl LinkIndex for Links {
    fn replace_document_links(
        &mut self,
        _: &WorkspaceId,
        _: LinkProjectionRecord,
    ) -> Result<(), LinkIndexError> {
        unreachable!()
    }
    fn get_document_links(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<Option<LinkProjectionRecord>, LinkIndexError> {
        unreachable!()
    }
    fn list_backlinks(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<Vec<Backlink>, LinkIndexError> {
        Ok(self.backlinks.clone())
    }
    fn list_unresolved_links(&self, _: &WorkspaceId) -> Result<Vec<DocumentLink>, LinkIndexError> {
        Ok(self.unresolved.clone())
    }
    fn list_orphan_documents(
        &self,
        _: &WorkspaceId,
        _: &[DocumentId],
    ) -> Result<Vec<DocumentId>, LinkIndexError> {
        unreachable!()
    }
}

struct Pointer;
impl CurrentDocumentVersionPointerPort for Pointer {
    fn load_current_version(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<Option<VersionId>, CurrentDocumentVersionPointerError> {
        Ok(Some(VersionId::new("source-version").unwrap()))
    }
    fn compare_and_set_current_version(
        &mut self,
        _: &WorkspaceId,
        _: &DocumentId,
        _: Option<&VersionId>,
        _: VersionId,
    ) -> Result<(), CurrentDocumentVersionPointerError> {
        unreachable!()
    }
}

#[derive(Default)]
struct Work {
    enqueued: Vec<ProjectionWork>,
}
impl ProjectionWorkRepository for Work {
    fn enqueue(
        &mut self,
        work: ProjectionWork,
    ) -> Result<ProjectionEnqueueOutcome, ProjectionWorkRepositoryError> {
        self.enqueued.push(work);
        Ok(ProjectionEnqueueOutcome::Enqueued)
    }
    fn get(
        &self,
        _: &ProjectionWorkIdentity,
    ) -> Result<Option<ProjectionWork>, ProjectionWorkRepositoryError> {
        Ok(None)
    }
    fn replace(
        &mut self,
        _: ProjectionWork,
        _: cabinet_domain::projection_work::ProjectionWorkState,
    ) -> Result<(), ProjectionWorkRepositoryError> {
        unreachable!()
    }
    fn list_resumable(
        &self,
        _: usize,
    ) -> Result<Vec<ProjectionWork>, ProjectionWorkRepositoryError> {
        unreachable!()
    }
}

fn backlink(source: &str) -> Backlink {
    Backlink::new(
        DocumentId::new(source).unwrap(),
        DocumentId::new("target").unwrap(),
        SourceRange::new(1, 2).unwrap(),
    )
}
fn unresolved(source: &str, target: &str) -> DocumentLink {
    let slug = DocumentSlug::from_title(&DocumentTitle::new(target).unwrap()).unwrap();
    DocumentLink::new(
        DocumentId::new(source).unwrap(),
        LinkTarget::unresolved(slug),
        SourceRange::new(1, 2).unwrap(),
    )
}
fn renamed() -> DocumentChangeEvent {
    DocumentChangeEvent::DocumentRenamed {
        workspace_id: "workspace-1".into(),
        document_id: "target".into(),
        version_id: "target-version".into(),
        title: "Renamed".into(),
        old_path: "old.md".into(),
        new_path: "new.md".into(),
    }
}
fn created() -> DocumentChangeEvent {
    DocumentChangeEvent::DocumentCreated {
        workspace_id: "workspace-1".into(),
        document_id: "target".into(),
        version_id: "target-version".into(),
        title: "Target".into(),
        path: "target.md".into(),
    }
}
fn deleted() -> DocumentChangeEvent {
    DocumentChangeEvent::DocumentDeleted {
        workspace_id: "workspace-1".into(),
        document_id: "target".into(),
        version_id: "target-version".into(),
    }
}
