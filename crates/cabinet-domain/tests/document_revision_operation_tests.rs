use cabinet_domain::document::DocumentId;
use cabinet_domain::document_revision::{
    DocumentExpectedCurrentVersion, DocumentMutationFingerprint, DocumentMutationKind,
    DocumentOperationError, DocumentOperationId, DocumentOperationIdentity,
    MAX_DOCUMENT_MUTATION_FINGERPRINT_LENGTH, MAX_DOCUMENT_OPERATION_ID_LENGTH,
};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;

#[test]
fn document_operation_id_rejects_invalid_values() {
    assert_eq!(
        DocumentOperationId::new(" ").expect_err("empty must fail"),
        DocumentOperationError::InvalidOperationId
    );
    assert_eq!(
        DocumentOperationId::new("bad\nvalue").expect_err("control must fail"),
        DocumentOperationError::InvalidOperationId
    );
    assert_eq!(
        DocumentOperationId::new(&"a".repeat(MAX_DOCUMENT_OPERATION_ID_LENGTH + 1))
            .expect_err("too long must fail"),
        DocumentOperationError::OperationIdTooLong
    );
    assert_eq!(
        DocumentOperationError::OperationIdTooLong.code(),
        "document_operation.operation_id_too_long"
    );
}

#[test]
fn create_requires_absent_current_and_other_mutations_require_match() {
    let create = identity(
        DocumentMutationKind::Create,
        DocumentExpectedCurrentVersion::MustNotExist,
    );
    assert_eq!(create.kind(), DocumentMutationKind::Create);

    for kind in [
        DocumentMutationKind::Update,
        DocumentMutationKind::AttachAsset,
        DocumentMutationKind::LinkAsset,
        DocumentMutationKind::UnlinkAsset,
        DocumentMutationKind::Restore,
    ] {
        let identity = identity(
            kind,
            DocumentExpectedCurrentVersion::MustMatch(
                VersionId::new("version-1").expect("version"),
            ),
        );
        assert_eq!(identity.kind(), kind);
    }
}

#[test]
fn mutation_and_current_guard_mismatch_is_rejected() {
    let create_with_current = DocumentOperationIdentity::new(
        operation_id(),
        workspace_id(),
        document_id(),
        DocumentMutationKind::Create,
        DocumentExpectedCurrentVersion::MustMatch(VersionId::new("version-1").expect("version")),
    )
    .expect_err("create cannot match current");
    let update_without_current = DocumentOperationIdentity::new(
        operation_id(),
        workspace_id(),
        document_id(),
        DocumentMutationKind::Update,
        DocumentExpectedCurrentVersion::MustNotExist,
    )
    .expect_err("update requires current");

    assert_eq!(
        create_with_current,
        DocumentOperationError::InvalidExpectedCurrentGuard
    );
    assert_eq!(create_with_current, update_without_current);
}

#[test]
fn mutation_fingerprint_is_bounded_and_part_of_operation_identity() {
    assert_eq!(
        DocumentMutationFingerprint::new(" ").expect_err("empty fingerprint"),
        DocumentOperationError::InvalidRequestFingerprint
    );
    assert_eq!(
        DocumentMutationFingerprint::new("bad\nvalue").expect_err("control fingerprint"),
        DocumentOperationError::InvalidRequestFingerprint
    );
    assert_eq!(
        DocumentMutationFingerprint::new(&"a".repeat(MAX_DOCUMENT_MUTATION_FINGERPRINT_LENGTH + 1))
            .expect_err("long fingerprint"),
        DocumentOperationError::RequestFingerprintTooLong
    );

    let first = identity(
        DocumentMutationKind::Update,
        DocumentExpectedCurrentVersion::MustMatch(VersionId::new("version-1").expect("version")),
    )
    .with_request_fingerprint(
        DocumentMutationFingerprint::new("sha256:first").expect("fingerprint"),
    );
    let same = identity(
        DocumentMutationKind::Update,
        DocumentExpectedCurrentVersion::MustMatch(VersionId::new("version-1").expect("version")),
    )
    .with_request_fingerprint(
        DocumentMutationFingerprint::new("sha256:first").expect("fingerprint"),
    );
    let changed = identity(
        DocumentMutationKind::Update,
        DocumentExpectedCurrentVersion::MustMatch(VersionId::new("version-1").expect("version")),
    )
    .with_request_fingerprint(
        DocumentMutationFingerprint::new("sha256:changed").expect("fingerprint"),
    );

    assert_eq!(first, same);
    assert_ne!(first, changed);
    assert_eq!(
        first.request_fingerprint().expect("fingerprint").as_str(),
        "sha256:first"
    );
    assert_eq!(
        DocumentOperationError::RequestFingerprintTooLong.code(),
        "document_operation.request_fingerprint_too_long"
    );
}

fn identity(
    kind: DocumentMutationKind,
    expected: DocumentExpectedCurrentVersion,
) -> DocumentOperationIdentity {
    DocumentOperationIdentity::new(
        operation_id(),
        workspace_id(),
        document_id(),
        kind,
        expected,
    )
    .expect("valid operation identity")
}

fn operation_id() -> DocumentOperationId {
    DocumentOperationId::new("operation-1").expect("operation id")
}

fn workspace_id() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace id")
}

fn document_id() -> DocumentId {
    DocumentId::new("doc-1").expect("document id")
}
