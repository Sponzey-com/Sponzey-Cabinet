use cabinet_domain::document_diff_query::{DocumentDiffQueryTarget, DocumentDiffQueryTargetError};

#[test]
fn current_to_version_target_validates_ids_and_exposes_typed_values() {
    let target =
        DocumentDiffQueryTarget::current_to_version(" workspace-1 ", " doc-1 ", " version-1 ")
            .unwrap();

    assert_eq!(target.workspace_id().as_str(), "workspace-1");
    assert_eq!(target.document_id().as_str(), "doc-1");
    assert_eq!(target.current_version_id().unwrap().as_str(), "version-1");
    assert_eq!(target.version_pair(), None);

    for (workspace, document, version) in [
        ("", "doc-1", "version-1"),
        ("workspace-1", "", "version-1"),
        ("workspace-1", "doc-1", ""),
    ] {
        assert_eq!(
            DocumentDiffQueryTarget::current_to_version(workspace, document, version).unwrap_err(),
            DocumentDiffQueryTargetError::InvalidTarget
        );
    }
}

#[test]
fn version_pair_target_validates_both_versions_and_is_deterministic() {
    let first = DocumentDiffQueryTarget::versions("workspace-1", "doc-1", "version-1", "version-2")
        .unwrap();
    let second =
        DocumentDiffQueryTarget::versions("workspace-1", "doc-1", "version-1", "version-2")
            .unwrap();

    assert_eq!(first, second);
    assert_eq!(first.current_version_id(), None);
    let (left, right) = first.version_pair().unwrap();
    assert_eq!(left.as_str(), "version-1");
    assert_eq!(right.as_str(), "version-2");

    for (left, right) in [("", "version-2"), ("version-1", "")] {
        assert_eq!(
            DocumentDiffQueryTarget::versions("workspace-1", "doc-1", left, right).unwrap_err(),
            DocumentDiffQueryTargetError::InvalidTarget
        );
    }
}

#[test]
fn target_error_has_a_stable_code() {
    assert_eq!(
        DocumentDiffQueryTargetError::InvalidTarget.code(),
        "document_diff_query.invalid_target"
    );
}
