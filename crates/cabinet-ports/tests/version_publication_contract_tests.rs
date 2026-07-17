use cabinet_domain::document_revision::DocumentOperationId;
use cabinet_domain::version::{DocumentRevisionNumber, VersionId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::version_publication::{
    PublishedVersion, VersionPublicationError, VersionPublicationPort,
};

struct FakePublicationPort;

impl VersionPublicationPort for FakePublicationPort {
    fn publish_prepared(
        &mut self,
        _workspace_id: &WorkspaceId,
        _operation_id: &DocumentOperationId,
    ) -> Result<PublishedVersion, VersionPublicationError> {
        Ok(PublishedVersion::new(
            VersionId::new("version-1").expect("version"),
            DocumentRevisionNumber::new(1).expect("revision"),
        ))
    }
}

#[test]
fn publication_port_is_replaceable_and_returns_version_identity() {
    let mut port = FakePublicationPort;
    let published = port
        .publish_prepared(
            &WorkspaceId::new("workspace-1").expect("workspace"),
            &DocumentOperationId::new("operation-1").expect("operation"),
        )
        .expect("publish");

    assert_eq!(published.version_id().as_str(), "version-1");
    assert_eq!(published.revision_number().value(), 1);
}

#[test]
fn publication_errors_have_stable_codes() {
    assert_eq!(
        VersionPublicationError::NotPrepared.code(),
        "version_publication.not_prepared"
    );
    assert_eq!(
        VersionPublicationError::Conflict.code(),
        "version_publication.conflict"
    );
    assert_eq!(
        VersionPublicationError::StorageUnavailable.code(),
        "version_publication.storage_unavailable"
    );
    assert_eq!(
        VersionPublicationError::CorruptedPublication.code(),
        "version_publication.corrupted"
    );
}
