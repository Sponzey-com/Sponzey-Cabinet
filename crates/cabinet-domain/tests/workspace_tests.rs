use cabinet_domain::workspace::{
    Workspace, WorkspaceError, WorkspaceId, WorkspaceName, WorkspacePath,
};

#[test]
fn workspace_id_trims_and_rejects_empty_value() {
    let id = WorkspaceId::new(" workspace-1 ").expect("id should be valid");

    assert_eq!(id.as_str(), "workspace-1");
    assert_eq!(
        WorkspaceId::new("   ").expect_err("empty id must fail"),
        WorkspaceError::EmptyId
    );
}

#[test]
fn workspace_name_trims_rejects_empty_too_long_and_control_characters() {
    let name = WorkspaceName::new(" Cabinet ").expect("name should be valid");

    assert_eq!(name.as_str(), "Cabinet");
    assert_eq!(
        WorkspaceName::new("").expect_err("empty name must fail"),
        WorkspaceError::EmptyName
    );
    assert_eq!(
        WorkspaceName::new(&"a".repeat(81)).expect_err("too long name must fail"),
        WorkspaceError::NameTooLong { max: 80 }
    );
    assert_eq!(
        WorkspaceName::new("bad\nname").expect_err("control character must fail"),
        WorkspaceError::InvalidNameCharacter
    );
}

#[test]
fn workspace_path_is_logical_and_rejects_filesystem_or_traversal_paths() {
    let path = WorkspacePath::new("team/knowledge-base").expect("path should be valid");

    assert_eq!(path.as_str(), "team/knowledge-base");
    assert_eq!(
        WorkspacePath::new("/Users/example/workspace").expect_err("absolute path must fail"),
        WorkspaceError::AbsoluteWorkspacePath
    );
    assert_eq!(
        WorkspacePath::new("team//workspace").expect_err("empty segment must fail"),
        WorkspaceError::EmptyPathSegment
    );
    assert_eq!(
        WorkspacePath::new("../workspace").expect_err("traversal must fail"),
        WorkspaceError::TraversalPathSegment
    );
}

#[test]
fn workspace_aggregate_uses_explicit_id_name_and_logical_path() {
    let workspace = Workspace::new(
        WorkspaceId::new("workspace-1").expect("id"),
        WorkspaceName::new("Cabinet").expect("name"),
        WorkspacePath::new("team/cabinet").expect("path"),
    );

    assert_eq!(workspace.id().as_str(), "workspace-1");
    assert_eq!(workspace.name().as_str(), "Cabinet");
    assert_eq!(workspace.path().as_str(), "team/cabinet");
}
