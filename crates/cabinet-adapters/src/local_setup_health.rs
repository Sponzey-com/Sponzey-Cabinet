use std::path::Path;

use cabinet_core::config::LocalPathsConfig;

use crate::local_first_run::FIRST_RUN_MARKER_FILE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSetupHealthChecker {
    local_paths: LocalPathsConfig,
}

impl LocalSetupHealthChecker {
    pub fn new(local_paths: LocalPathsConfig) -> Self {
        Self { local_paths }
    }

    pub fn check(&self) -> LocalSetupHealthReport {
        let mut issues = Vec::new();

        check_directory(
            LocalSetupHealthRole::Metadata,
            &self.local_paths.metadata_dir,
            &mut issues,
        );
        check_directory(
            LocalSetupHealthRole::VersionStore,
            &self.local_paths.version_store_dir,
            &mut issues,
        );
        check_directory(
            LocalSetupHealthRole::AssetStore,
            &self.local_paths.asset_store_dir,
            &mut issues,
        );
        check_directory(
            LocalSetupHealthRole::SearchIndex,
            &self.local_paths.search_index_dir,
            &mut issues,
        );
        check_directory(
            LocalSetupHealthRole::WorkspaceRoot,
            &self.local_paths.workspace_root,
            &mut issues,
        );

        if self.local_paths.metadata_dir.is_dir()
            && !self
                .local_paths
                .metadata_dir
                .join(FIRST_RUN_MARKER_FILE)
                .is_file()
        {
            issues.push(LocalSetupHealthIssue::new(
                LocalSetupHealthRole::Metadata,
                LocalSetupHealthIssueKind::MissingFirstRunMarker,
            ));
        }

        LocalSetupHealthReport { issues }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSetupHealthReport {
    issues: Vec<LocalSetupHealthIssue>,
}

impl LocalSetupHealthReport {
    pub fn status(&self) -> LocalSetupHealthStatus {
        if self.issues.is_empty() {
            LocalSetupHealthStatus::Healthy
        } else {
            LocalSetupHealthStatus::Unhealthy
        }
    }

    pub fn issues(&self) -> &[LocalSetupHealthIssue] {
        &self.issues
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalSetupHealthStatus {
    Healthy,
    Unhealthy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalSetupHealthIssue {
    role: LocalSetupHealthRole,
    kind: LocalSetupHealthIssueKind,
}

impl LocalSetupHealthIssue {
    pub fn new(role: LocalSetupHealthRole, kind: LocalSetupHealthIssueKind) -> Self {
        Self { role, kind }
    }

    pub fn role(self) -> LocalSetupHealthRole {
        self.role
    }

    pub fn kind(self) -> LocalSetupHealthIssueKind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalSetupHealthRole {
    Metadata,
    VersionStore,
    AssetStore,
    SearchIndex,
    WorkspaceRoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalSetupHealthIssueKind {
    MissingDirectory,
    PathIsNotDirectory,
    MissingFirstRunMarker,
}

fn check_directory(
    role: LocalSetupHealthRole,
    path: &Path,
    issues: &mut Vec<LocalSetupHealthIssue>,
) {
    if !path.exists() {
        issues.push(LocalSetupHealthIssue::new(
            role,
            LocalSetupHealthIssueKind::MissingDirectory,
        ));
        return;
    }
    if !path.is_dir() {
        issues.push(LocalSetupHealthIssue::new(
            role,
            LocalSetupHealthIssueKind::PathIsNotDirectory,
        ));
    }
}
