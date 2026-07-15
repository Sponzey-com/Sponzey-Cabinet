use std::fs;
use std::path::PathBuf;

use cabinet_adapters::local_asset_store::LocalAssetStore;
use cabinet_adapters::local_document_asset_repository::LocalDocumentAssetRepository;
use cabinet_adapters::local_document_repository::LocalDocumentRepository;
use cabinet_adapters::local_first_run::LocalFirstRunStore;
use cabinet_adapters::local_link_index::LocalLinkIndex;
use cabinet_adapters::local_markdown_parser::LocalMarkdownParser;
use cabinet_adapters::local_migration::LocalMigrationStore;
use cabinet_adapters::local_phase002_migration_fixture::{
    LocalPhase002FixtureFailure, LocalPhase002MigrationFixtureStore,
};
use cabinet_adapters::local_search_index::LocalSearchIndex;
use cabinet_adapters::local_setup_health::{
    LocalSetupHealthChecker, LocalSetupHealthIssueKind, LocalSetupHealthRole,
    LocalSetupHealthStatus,
};
use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_core::config::{AppConfig, ConfigError, ExternalEnvironmentSnapshot};
use cabinet_core::first_run::{FirstRunInitializationOutcome, FirstRunInitializer, FirstRunState};
use cabinet_core::migration::{
    MigrationOutcome, MigrationPlan, MigrationProductEvent, MigrationRunner, MigrationState,
    Phase002FixtureRecord, Phase002FixtureRecordKind, Phase002MigrationFixture,
};
use cabinet_domain::asset::AssetId;
use cabinet_domain::document::{DocumentBodyPolicy, DocumentId};
use cabinet_domain::link::Backlink;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_store::{AssetStore, AssetStoreError};
use cabinet_ports::document_repository::DocumentRepository;
use cabinet_ports::link_index::{LinkIndex, LinkProjectionRecord};
use cabinet_ports::markdown_parser::{MarkdownParser, ParsedMarkdown};
use cabinet_ports::search_index::{SearchDocumentRecord, SearchIndex};
use cabinet_usecases::document::{
    AttachFileToDocumentInput, AttachFileToDocumentUsecase, CreateDocumentInput,
    CreateDocumentProductEvent, CreateDocumentUsecase, DocumentChangeEvent,
    DocumentChangeEventPublisher, DocumentProductLogger, GetCurrentDocumentInput,
    GetCurrentDocumentUsecase, GetDocumentHistoryInput, GetDocumentHistoryUsecase,
    GetDocumentVersionInput, GetDocumentVersionUsecase, ListDocumentAssetsInput,
    ListDocumentAssetsUsecase, PreviewDocumentRestoreInput, PreviewDocumentRestoreUsecase,
    RestoreDocumentVersionInput, RestoreDocumentVersionState, RestoreDocumentVersionUsecase,
    UpdateDocumentInput, UpdateDocumentUsecase,
};
use cabinet_usecases::graph::{GraphLiteProjectionInput, GraphLiteProjectionUsecase};
use cabinet_usecases::search::{SearchDocumentsInput, SearchDocumentsUsecase};

const FIXTURE_WORKSPACE_ID: &str = "workspace-1";
const FIXTURE_DOCUMENT_ID: &str = "doc-0001";
const FIXTURE_DOCUMENT_PATH: &str = "docs/release-smoke.md";
const FIXTURE_INITIAL_BODY: &str = "# Release Smoke Document\ninitial release smoke body";
const FIXTURE_UPDATED_BODY: &str = "updated release smoke body with attachment reference";
const FIXTURE_INITIAL_VERSION_ID: &str = "v-0001";
const FIXTURE_UPDATED_VERSION_ID: &str = "v-0002";
const FIXTURE_INITIAL_SNAPSHOT_REF: &str = "snapshot-0001";
const FIXTURE_UPDATED_SNAPSHOT_REF: &str = "snapshot-0002";
const FIXTURE_AUTHOR: &str = "system";
const FIXTURE_ASSET_ID: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const FIXTURE_ASSET_FILE_NAME: &str = "release-smoke.txt";
const FIXTURE_ASSET_MEDIA_TYPE: &str = "text/plain";
const FIXTURE_ASSET_LABEL: &str = "Release Smoke Asset";
const FIXTURE_ASSET_BYTES: &[u8] = b"cabinet fixture asset";
const E2E_WORKSPACE_ID: &str = "workspace-e2e";
const E2E_SOURCE_DOCUMENT_ID: &str = "doc-source";
const E2E_TARGET_DOCUMENT_ID: &str = "doc-target";
const E2E_TARGET_TITLE: &str = "Target Document";
const E2E_SOURCE_PATH: &str = "docs/source.md";
const E2E_TARGET_PATH: &str = "docs/target.md";
const E2E_INITIAL_BODY: &str = "# Source Document\ninitial body before restore\n";
const E2E_TARGET_BODY: &str = "# Target Document\nlinked target body\n";
const E2E_ASSET_ID: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const E2E_ASSET_FILE_NAME: &str = "mvp-e2e.txt";
const E2E_ASSET_MEDIA_TYPE: &str = "text/plain";
const E2E_ASSET_LABEL: &str = "MVP Asset";
const E2E_ASSET_BYTES: &[u8] = b"mvp end to end asset";
const E2E_SOURCE_VERSION_1: &str = "source-v-0001";
const E2E_SOURCE_VERSION_2: &str = "source-v-0002";
const E2E_SOURCE_VERSION_3: &str = "source-v-0003";
const E2E_TARGET_VERSION_1: &str = "target-v-0001";
const E2E_SOURCE_SNAPSHOT_1: &str = "source-snapshot-0001";
const E2E_SOURCE_SNAPSHOT_2: &str = "source-snapshot-0002";
const E2E_SOURCE_SNAPSHOT_3: &str = "source-snapshot-0003";
const E2E_TARGET_SNAPSHOT_1: &str = "target-snapshot-0001";
const E2E_UPDATED_BODY: &str = "# Source\nsearchneedle linked body\nSee [[Target Document]].\nAsset ![[asset:2222222222222222222222222222222222222222222222222222222222222222|MVP Asset]]\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanInstallSmokeInput {
    app_data_dir: PathBuf,
}

impl CleanInstallSmokeInput {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanInstallSmokeReport {
    first_run: FirstRunInitializationOutcome,
    setup_health: LocalSetupHealthStatus,
}

impl CleanInstallSmokeReport {
    pub fn completed(&self) -> bool {
        self.first_run.final_state == FirstRunState::Completed
    }

    pub fn healthy(&self) -> bool {
        self.setup_health == LocalSetupHealthStatus::Healthy
    }

    pub fn created_directories(&self) -> usize {
        self.first_run.created_directories
    }

    pub fn already_present_directories(&self) -> usize {
        self.first_run.already_present_directories
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPreservationSmokeInput {
    app_data_dir: PathBuf,
}

impl DataPreservationSmokeInput {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPreservationSmokeReport {
    first_run: FirstRunInitializationOutcome,
    initial_migration: MigrationOutcome,
    idempotent_migration: MigrationOutcome,
    current_document_preserved: bool,
    version_history_preserved: bool,
    specific_version_preserved: bool,
    asset_metadata_preserved: bool,
    asset_object_preserved: bool,
    product_log_sensitive_data_absent: bool,
    history_entry_count: usize,
    asset_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phase002MigrationFixtureSmokeInput {
    app_data_dir: PathBuf,
}

impl Phase002MigrationFixtureSmokeInput {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupRepairSmokeInput {
    app_data_dir: PathBuf,
}

impl StartupRepairSmokeInput {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phase002MigrationFixtureSmokeReport {
    first_run: FirstRunInitializationOutcome,
    initial_migration: MigrationOutcome,
    idempotent_migration: MigrationOutcome,
    fixture_record_count: usize,
    required_fixture_records_preserved: bool,
    migration_failure_preserved_current_fixture: bool,
    product_log_sensitive_data_absent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupRepairSmokeReport {
    first_run: FirstRunInitializationOutcome,
    initial_migration: MigrationOutcome,
    repair_outcome: StartupRepairOutcome,
    corruption_detected_before_repair: bool,
    current_document_preserved: bool,
    search_result_found: bool,
    product_log_sensitive_data_absent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MvpEndToEndSmokeInput {
    app_data_dir: PathBuf,
}

impl MvpEndToEndSmokeInput {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MvpEndToEndSmokeReport {
    first_run: FirstRunInitializationOutcome,
    migration: MigrationOutcome,
    document_created: bool,
    document_edited: bool,
    wikilink_parsed: bool,
    asset_reference_parsed: bool,
    search_result_found: bool,
    backlink_found: bool,
    asset_metadata_listed: bool,
    restore_preview_available: bool,
    restore_completed: bool,
    restored_current_document_matches_initial_version: bool,
    product_log_sensitive_data_absent: bool,
    history_entry_count: usize,
}

impl MvpEndToEndSmokeReport {
    pub fn first_run_completed(&self) -> bool {
        self.first_run.final_state == FirstRunState::Completed
    }

    pub fn migration_completed(&self) -> bool {
        self.migration.final_state == MigrationState::Completed
    }

    pub fn document_created(&self) -> bool {
        self.document_created
    }

    pub fn document_edited(&self) -> bool {
        self.document_edited
    }

    pub fn wikilink_parsed(&self) -> bool {
        self.wikilink_parsed
    }

    pub fn asset_reference_parsed(&self) -> bool {
        self.asset_reference_parsed
    }

    pub fn search_result_found(&self) -> bool {
        self.search_result_found
    }

    pub fn backlink_found(&self) -> bool {
        self.backlink_found
    }

    pub fn asset_metadata_listed(&self) -> bool {
        self.asset_metadata_listed
    }

    pub fn restore_preview_available(&self) -> bool {
        self.restore_preview_available
    }

    pub fn restore_completed(&self) -> bool {
        self.restore_completed
    }

    pub fn restored_current_document_matches_initial_version(&self) -> bool {
        self.restored_current_document_matches_initial_version
    }

    pub fn product_log_sensitive_data_absent(&self) -> bool {
        self.product_log_sensitive_data_absent
    }

    pub fn history_entry_count(&self) -> usize {
        self.history_entry_count
    }
}

impl DataPreservationSmokeReport {
    pub fn first_run_completed(&self) -> bool {
        self.first_run.final_state == FirstRunState::Completed
    }

    pub fn initial_migration_completed(&self) -> bool {
        self.initial_migration.final_state == MigrationState::Completed
    }

    pub fn migration_idempotent(&self) -> bool {
        self.idempotent_migration.final_state == MigrationState::Completed
            && self.idempotent_migration.applied_versions.is_empty()
    }

    pub fn current_document_preserved(&self) -> bool {
        self.current_document_preserved
    }

    pub fn version_history_preserved(&self) -> bool {
        self.version_history_preserved
    }

    pub fn specific_version_preserved(&self) -> bool {
        self.specific_version_preserved
    }

    pub fn asset_metadata_preserved(&self) -> bool {
        self.asset_metadata_preserved
    }

    pub fn asset_object_preserved(&self) -> bool {
        self.asset_object_preserved
    }

    pub fn product_log_sensitive_data_absent(&self) -> bool {
        self.product_log_sensitive_data_absent
    }

    pub fn history_entry_count(&self) -> usize {
        self.history_entry_count
    }

    pub fn asset_count(&self) -> usize {
        self.asset_count
    }
}

impl Phase002MigrationFixtureSmokeReport {
    pub fn first_run_completed(&self) -> bool {
        self.first_run.final_state == FirstRunState::Completed
    }

    pub fn initial_migration_completed(&self) -> bool {
        self.initial_migration.final_state == MigrationState::Completed
    }

    pub fn migration_idempotent(&self) -> bool {
        self.idempotent_migration.final_state == MigrationState::Completed
            && self.idempotent_migration.applied_versions.is_empty()
    }

    pub fn fixture_record_count(&self) -> usize {
        self.fixture_record_count
    }

    pub fn required_fixture_records_preserved(&self) -> bool {
        self.required_fixture_records_preserved
    }

    pub fn migration_failure_preserved_current_fixture(&self) -> bool {
        self.migration_failure_preserved_current_fixture
    }

    pub fn product_log_sensitive_data_absent(&self) -> bool {
        self.product_log_sensitive_data_absent
    }
}

impl StartupRepairSmokeReport {
    pub fn first_run_completed(&self) -> bool {
        self.first_run.final_state == FirstRunState::Completed
    }

    pub fn initial_migration_completed(&self) -> bool {
        self.initial_migration.final_state == MigrationState::Completed
    }

    pub fn corruption_detected_before_repair(&self) -> bool {
        self.corruption_detected_before_repair
    }

    pub fn startup_repair_completed(&self) -> bool {
        self.repair_outcome.final_state == StartupRepairState::Completed
    }

    pub fn corrupted_index_rebuilt(&self) -> bool {
        self.repair_outcome.corrupted_index_rebuilt
    }

    pub fn current_document_preserved(&self) -> bool {
        self.current_document_preserved
    }

    pub fn search_result_found(&self) -> bool {
        self.search_result_found
    }

    pub fn product_log_sensitive_data_absent(&self) -> bool {
        self.product_log_sensitive_data_absent
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartupRepairOutcome {
    final_state: StartupRepairState,
    corrupted_index_rebuilt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupRepairState {
    NotStarted,
    Inspecting,
    RepairingIndex,
    RebuildingProjection,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupRepairEvent {
    Start,
    CorruptionDetected,
    IndexRepaired,
    ProjectionRebuilt,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartupRepairTransition {
    pub previous_state: StartupRepairState,
    pub event: StartupRepairEvent,
    pub next_state: StartupRepairState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupRepairError {
    InvalidTransition {
        state: StartupRepairState,
        event: StartupRepairEvent,
    },
}

impl StartupRepairError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidTransition { .. } => "startup_repair.invalid_transition",
        }
    }
}

pub fn transition_startup_repair(
    state: StartupRepairState,
    event: StartupRepairEvent,
) -> Result<StartupRepairTransition, StartupRepairError> {
    let next_state = match (state, event) {
        (StartupRepairState::NotStarted, StartupRepairEvent::Start) => {
            StartupRepairState::Inspecting
        }
        (StartupRepairState::Inspecting, StartupRepairEvent::CorruptionDetected) => {
            StartupRepairState::RepairingIndex
        }
        (StartupRepairState::RepairingIndex, StartupRepairEvent::IndexRepaired) => {
            StartupRepairState::RebuildingProjection
        }
        (StartupRepairState::RebuildingProjection, StartupRepairEvent::ProjectionRebuilt) => {
            StartupRepairState::Completed
        }
        (
            StartupRepairState::NotStarted
            | StartupRepairState::Inspecting
            | StartupRepairState::RepairingIndex
            | StartupRepairState::RebuildingProjection,
            StartupRepairEvent::Fail,
        ) => StartupRepairState::Failed,
        _ => return Err(StartupRepairError::InvalidTransition { state, event }),
    };

    Ok(StartupRepairTransition {
        previous_state: state,
        event,
        next_state,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseSmokeError {
    NonUtf8Path,
    InvalidConfig(ConfigError),
    FirstRunFailed,
    MigrationFailed,
    UsecaseFailed(&'static str),
    AssetStoreFailed(&'static str),
    DataMismatch(&'static str),
}

pub fn run_clean_install_smoke(
    input: CleanInstallSmokeInput,
) -> Result<CleanInstallSmokeReport, ReleaseSmokeError> {
    let config = app_config_from_app_data_dir(input.app_data_dir)?;
    let first_run = run_first_run(&config);
    let setup_health = LocalSetupHealthChecker::new(config.local_paths.clone())
        .check()
        .status();

    Ok(CleanInstallSmokeReport {
        first_run,
        setup_health,
    })
}

pub fn run_data_preservation_smoke(
    input: DataPreservationSmokeInput,
) -> Result<DataPreservationSmokeReport, ReleaseSmokeError> {
    let config = app_config_from_app_data_dir(input.app_data_dir)?;
    let first_run = run_first_run(&config);
    if first_run.final_state != FirstRunState::Completed {
        return Err(ReleaseSmokeError::FirstRunFailed);
    }

    let initial_migration = run_initial_migration(&config);
    if initial_migration.final_state != MigrationState::Completed {
        return Err(ReleaseSmokeError::MigrationFailed);
    }

    let product_log_sensitive_data_absent = seed_preservation_fixture(&config)?;

    let second_first_run = run_first_run(&config);
    if second_first_run.final_state != FirstRunState::Completed {
        return Err(ReleaseSmokeError::FirstRunFailed);
    }

    let idempotent_migration = run_initial_migration(&config);
    if idempotent_migration.final_state != MigrationState::Completed {
        return Err(ReleaseSmokeError::MigrationFailed);
    }

    let readback = read_preservation_fixture(&config)?;
    if !readback.current_document_preserved {
        return Err(ReleaseSmokeError::DataMismatch("current_document"));
    }
    if !readback.version_history_preserved {
        return Err(ReleaseSmokeError::DataMismatch("version_history"));
    }
    if !readback.specific_version_preserved {
        return Err(ReleaseSmokeError::DataMismatch("specific_version"));
    }
    if !readback.asset_metadata_preserved {
        return Err(ReleaseSmokeError::DataMismatch("asset_metadata"));
    }
    if !readback.asset_object_preserved {
        return Err(ReleaseSmokeError::DataMismatch("asset_object"));
    }
    if !product_log_sensitive_data_absent {
        return Err(ReleaseSmokeError::DataMismatch("product_log"));
    }

    Ok(DataPreservationSmokeReport {
        first_run: second_first_run,
        initial_migration,
        idempotent_migration,
        current_document_preserved: readback.current_document_preserved,
        version_history_preserved: readback.version_history_preserved,
        specific_version_preserved: readback.specific_version_preserved,
        asset_metadata_preserved: readback.asset_metadata_preserved,
        asset_object_preserved: readback.asset_object_preserved,
        product_log_sensitive_data_absent,
        history_entry_count: readback.history_entry_count,
        asset_count: readback.asset_count,
    })
}

pub fn run_phase002_migration_fixture_smoke(
    input: Phase002MigrationFixtureSmokeInput,
) -> Result<Phase002MigrationFixtureSmokeReport, ReleaseSmokeError> {
    let config = app_config_from_app_data_dir(input.app_data_dir)?;
    let first_run = run_first_run(&config);
    if first_run.final_state != FirstRunState::Completed {
        return Err(ReleaseSmokeError::FirstRunFailed);
    }

    let initial_migration = run_initial_migration(&config);
    if initial_migration.final_state != MigrationState::Completed {
        return Err(ReleaseSmokeError::MigrationFailed);
    }

    let fixture = Phase002MigrationFixture::self_host_sample();
    let fixture_store =
        LocalPhase002MigrationFixtureStore::new(config.local_paths.metadata_dir.clone());
    fixture_store
        .save_fixture(&fixture)
        .map_err(|_| ReleaseSmokeError::DataMismatch("phase002_fixture_save"))?;

    let second_first_run = run_first_run(&config);
    if second_first_run.final_state != FirstRunState::Completed {
        return Err(ReleaseSmokeError::FirstRunFailed);
    }

    let idempotent_migration = run_initial_migration(&config);
    if idempotent_migration.final_state != MigrationState::Completed {
        return Err(ReleaseSmokeError::MigrationFailed);
    }

    let loaded = fixture_store
        .load_fixture()
        .map_err(|_| ReleaseSmokeError::DataMismatch("phase002_fixture_load"))?;
    let required_fixture_records_preserved = loaded == fixture;

    let changed = changed_phase002_fixture();
    let _ = fixture_store
        .save_fixture_with_failure_for_test(&changed, LocalPhase002FixtureFailure::BeforeCommit);
    let after_failure = fixture_store
        .load_fixture()
        .map_err(|_| ReleaseSmokeError::DataMismatch("phase002_fixture_after_failure"))?;
    let migration_failure_preserved_current_fixture = after_failure == fixture;

    let product_log_sensitive_data_absent = phase002_migration_product_log_sensitive_data_absent(
        &fixture,
        &[
            &initial_migration.product_event,
            &idempotent_migration.product_event,
        ],
    );

    let report = Phase002MigrationFixtureSmokeReport {
        first_run: second_first_run,
        initial_migration,
        idempotent_migration,
        fixture_record_count: loaded.record_count(),
        required_fixture_records_preserved,
        migration_failure_preserved_current_fixture,
        product_log_sensitive_data_absent,
    };

    if !report.required_fixture_records_preserved {
        return Err(ReleaseSmokeError::DataMismatch("phase002_fixture_records"));
    }
    if !report.migration_failure_preserved_current_fixture {
        return Err(ReleaseSmokeError::DataMismatch(
            "phase002_fixture_failure_preservation",
        ));
    }
    if !report.product_log_sensitive_data_absent {
        return Err(ReleaseSmokeError::DataMismatch(
            "phase002_fixture_product_log",
        ));
    }

    Ok(report)
}

pub fn run_startup_repair_smoke(
    input: StartupRepairSmokeInput,
) -> Result<StartupRepairSmokeReport, ReleaseSmokeError> {
    let config = app_config_from_app_data_dir(input.app_data_dir)?;
    let first_run = run_first_run(&config);
    if first_run.final_state != FirstRunState::Completed {
        return Err(ReleaseSmokeError::FirstRunFailed);
    }

    let initial_migration = run_initial_migration(&config);
    if initial_migration.final_state != MigrationState::Completed {
        return Err(ReleaseSmokeError::MigrationFailed);
    }

    let product_log_sensitive_data_absent = seed_preservation_fixture(&config)?;
    corrupt_search_index_path(&config)?;

    let pre_repair_health = LocalSetupHealthChecker::new(config.local_paths.clone()).check();
    let corruption_detected_before_repair = pre_repair_health.issues().iter().any(|issue| {
        issue.role() == LocalSetupHealthRole::SearchIndex
            && issue.kind() == LocalSetupHealthIssueKind::PathIsNotDirectory
    });
    if !corruption_detected_before_repair {
        return Err(ReleaseSmokeError::DataMismatch(
            "startup_repair_corruption_detection",
        ));
    }

    let (repair_outcome, search_result_found) = run_startup_repair(&config)?;
    let post_repair_health = LocalSetupHealthChecker::new(config.local_paths.clone()).check();
    if post_repair_health.status() != LocalSetupHealthStatus::Healthy {
        return Err(ReleaseSmokeError::DataMismatch("startup_repair_health"));
    }

    let readback = read_preservation_fixture(&config)?;

    let report = StartupRepairSmokeReport {
        first_run,
        initial_migration,
        repair_outcome,
        corruption_detected_before_repair,
        current_document_preserved: readback.current_document_preserved,
        search_result_found,
        product_log_sensitive_data_absent,
    };

    if !report.startup_repair_completed() {
        return Err(ReleaseSmokeError::DataMismatch("startup_repair_state"));
    }
    if !report.corrupted_index_rebuilt() {
        return Err(ReleaseSmokeError::DataMismatch("startup_repair_index"));
    }
    if !report.current_document_preserved {
        return Err(ReleaseSmokeError::DataMismatch("startup_repair_current"));
    }
    if !report.search_result_found {
        return Err(ReleaseSmokeError::DataMismatch("startup_repair_search"));
    }
    if !report.product_log_sensitive_data_absent {
        return Err(ReleaseSmokeError::DataMismatch(
            "startup_repair_product_log",
        ));
    }

    Ok(report)
}

pub fn run_mvp_end_to_end_smoke(
    input: MvpEndToEndSmokeInput,
) -> Result<MvpEndToEndSmokeReport, ReleaseSmokeError> {
    let config = app_config_from_app_data_dir(input.app_data_dir)?;
    let first_run = run_first_run(&config);
    if first_run.final_state != FirstRunState::Completed {
        return Err(ReleaseSmokeError::FirstRunFailed);
    }

    let migration = run_initial_migration(&config);
    if migration.final_state != MigrationState::Completed {
        return Err(ReleaseSmokeError::MigrationFailed);
    }

    let body_policy = smoke_body_policy();
    let mut documents = LocalDocumentRepository::with_body_policy(
        config.local_paths.workspace_root.clone(),
        body_policy,
    );
    let mut versions = LocalVersionStore::with_body_policy(
        config.local_paths.version_store_dir.clone(),
        body_policy,
    );
    let mut assets = LocalAssetStore::new(config.local_paths.asset_store_dir.clone());
    let mut document_assets =
        LocalDocumentAssetRepository::new(config.local_paths.metadata_dir.clone());
    let mut search_index = LocalSearchIndex::default();
    let mut link_index = LocalLinkIndex::default();
    let markdown_parser = LocalMarkdownParser::new();
    let mut events = ReleaseSmokeEventPublisher::default();
    let mut product_log = ReleaseSmokeProductLogger::default();

    let target_created = CreateDocumentUsecase::new(body_policy)
        .execute(
            CreateDocumentInput::new(
                E2E_WORKSPACE_ID,
                E2E_TARGET_DOCUMENT_ID,
                E2E_TARGET_PATH,
                E2E_TARGET_BODY,
                E2E_TARGET_VERSION_1,
                E2E_TARGET_SNAPSHOT_1,
                FIXTURE_AUTHOR,
                "create target",
            ),
            &mut documents,
            &mut versions,
            &mut events,
            &mut product_log,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;

    let source_created = CreateDocumentUsecase::new(body_policy)
        .execute(
            CreateDocumentInput::new(
                E2E_WORKSPACE_ID,
                E2E_SOURCE_DOCUMENT_ID,
                E2E_SOURCE_PATH,
                E2E_INITIAL_BODY,
                E2E_SOURCE_VERSION_1,
                E2E_SOURCE_SNAPSHOT_1,
                FIXTURE_AUTHOR,
                "create source",
            ),
            &mut documents,
            &mut versions,
            &mut events,
            &mut product_log,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    let document_created = target_created.document_id().as_str() == E2E_TARGET_DOCUMENT_ID
        && source_created.document_id().as_str() == E2E_SOURCE_DOCUMENT_ID;

    let update = UpdateDocumentUsecase::new(body_policy)
        .execute(
            UpdateDocumentInput::new(
                E2E_WORKSPACE_ID,
                E2E_SOURCE_DOCUMENT_ID,
                E2E_UPDATED_BODY,
                E2E_SOURCE_VERSION_2,
                E2E_SOURCE_SNAPSHOT_2,
                FIXTURE_AUTHOR,
                "edit source",
            ),
            &mut documents,
            &mut versions,
            &mut events,
            &mut product_log,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    let document_edited = update.version_id().as_str() == E2E_SOURCE_VERSION_2;

    AttachFileToDocumentUsecase::new()
        .execute(
            AttachFileToDocumentInput::new(
                E2E_WORKSPACE_ID,
                E2E_SOURCE_DOCUMENT_ID,
                E2E_SOURCE_VERSION_2,
                E2E_ASSET_ID,
                E2E_ASSET_FILE_NAME,
                E2E_ASSET_MEDIA_TYPE,
                E2E_ASSET_BYTES.to_vec(),
                E2E_ASSET_LABEL,
            ),
            &documents,
            &mut assets,
            &mut document_assets,
            &mut events,
            &mut product_log,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;

    let source_current = GetCurrentDocumentUsecase::new()
        .execute(
            GetCurrentDocumentInput::by_id(E2E_WORKSPACE_ID, E2E_SOURCE_DOCUMENT_ID),
            &documents,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    let parsed = markdown_parser
        .parse(source_current.record().body())
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    let wikilink_parsed = parsed
        .wikilinks()
        .iter()
        .any(|link| link.target() == E2E_TARGET_TITLE);
    let asset_reference_parsed = parsed
        .asset_references()
        .iter()
        .any(|reference| reference.asset_id().as_str() == E2E_ASSET_ID);

    upsert_current_document_in_search(
        E2E_WORKSPACE_ID,
        E2E_SOURCE_DOCUMENT_ID,
        &documents,
        &mut search_index,
    )?;
    upsert_current_document_in_search(
        E2E_WORKSPACE_ID,
        E2E_TARGET_DOCUMENT_ID,
        &documents,
        &mut search_index,
    )?;
    replace_source_link_projection(E2E_WORKSPACE_ID, &parsed, &mut link_index)?;

    let search = SearchDocumentsUsecase::new()
        .execute(
            SearchDocumentsInput::new(E2E_WORKSPACE_ID, "searchneedle", 10),
            &search_index,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    let search_result_found = search
        .page()
        .results()
        .iter()
        .any(|result| result.document_id().as_str() == E2E_SOURCE_DOCUMENT_ID);

    let graph = GraphLiteProjectionUsecase::new()
        .execute(
            GraphLiteProjectionInput::new(
                E2E_WORKSPACE_ID,
                E2E_TARGET_DOCUMENT_ID,
                vec![E2E_SOURCE_DOCUMENT_ID, E2E_TARGET_DOCUMENT_ID],
            ),
            &link_index,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    let backlink_found = graph.edges().iter().any(|edge| {
        edge.source_id() == E2E_SOURCE_DOCUMENT_ID && edge.target_id() == E2E_TARGET_DOCUMENT_ID
    });

    let asset_page = ListDocumentAssetsUsecase::new()
        .execute(
            ListDocumentAssetsInput::new(E2E_WORKSPACE_ID, E2E_SOURCE_DOCUMENT_ID),
            &documents,
            &document_assets,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    let asset_metadata_listed = asset_page
        .assets()
        .iter()
        .any(|asset| asset.asset_id().as_str() == E2E_ASSET_ID);

    let restore_preview = PreviewDocumentRestoreUsecase::new()
        .execute(
            PreviewDocumentRestoreInput::new(
                E2E_WORKSPACE_ID,
                E2E_SOURCE_DOCUMENT_ID,
                E2E_SOURCE_VERSION_1,
            ),
            &documents,
            &versions,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    let restore_preview_available = restore_preview.can_restore();

    let restore = RestoreDocumentVersionUsecase::new()
        .execute(
            RestoreDocumentVersionInput::new(
                E2E_WORKSPACE_ID,
                E2E_SOURCE_DOCUMENT_ID,
                E2E_SOURCE_VERSION_1,
                E2E_SOURCE_VERSION_3,
                E2E_SOURCE_SNAPSHOT_3,
                FIXTURE_AUTHOR,
                "restore source",
            ),
            &mut documents,
            &mut versions,
            &mut events,
            &mut product_log,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    let restore_completed = restore.final_state() == RestoreDocumentVersionState::Completed;

    let restored_current = GetCurrentDocumentUsecase::new()
        .execute(
            GetCurrentDocumentInput::by_id(E2E_WORKSPACE_ID, E2E_SOURCE_DOCUMENT_ID),
            &documents,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    let restored_current_document_matches_initial_version =
        restored_current.record().body().as_str() == E2E_INITIAL_BODY;

    let history = GetDocumentHistoryUsecase::new()
        .execute(
            GetDocumentHistoryInput::new(E2E_WORKSPACE_ID, E2E_SOURCE_DOCUMENT_ID, None, 10),
            &versions,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    let history_entry_count = history.page().entries().len();

    let product_log_sensitive_data_absent = product_log.sensitive_data_absent()
        && !format!("{:?}", product_log.events).contains(E2E_UPDATED_BODY)
        && !format!("{:?}", product_log.events).contains("mvp end to end asset");

    let report = MvpEndToEndSmokeReport {
        first_run,
        migration,
        document_created,
        document_edited,
        wikilink_parsed,
        asset_reference_parsed,
        search_result_found,
        backlink_found,
        asset_metadata_listed,
        restore_preview_available,
        restore_completed,
        restored_current_document_matches_initial_version,
        product_log_sensitive_data_absent,
        history_entry_count,
    };

    if !report.document_created {
        return Err(ReleaseSmokeError::DataMismatch("e2e_document_created"));
    }
    if !report.document_edited {
        return Err(ReleaseSmokeError::DataMismatch("e2e_document_edited"));
    }
    if !report.wikilink_parsed {
        return Err(ReleaseSmokeError::DataMismatch("e2e_wikilink"));
    }
    if !report.asset_reference_parsed {
        return Err(ReleaseSmokeError::DataMismatch("e2e_asset_reference"));
    }
    if !report.search_result_found {
        return Err(ReleaseSmokeError::DataMismatch("e2e_search"));
    }
    if !report.backlink_found {
        return Err(ReleaseSmokeError::DataMismatch("e2e_backlink"));
    }
    if !report.asset_metadata_listed {
        return Err(ReleaseSmokeError::DataMismatch("e2e_asset_metadata"));
    }
    if !report.restore_preview_available {
        return Err(ReleaseSmokeError::DataMismatch("e2e_restore_preview"));
    }
    if !report.restore_completed {
        return Err(ReleaseSmokeError::DataMismatch("e2e_restore"));
    }
    if !report.restored_current_document_matches_initial_version {
        return Err(ReleaseSmokeError::DataMismatch("e2e_restored_current"));
    }
    if !report.product_log_sensitive_data_absent {
        return Err(ReleaseSmokeError::DataMismatch("e2e_product_log"));
    }
    if report.history_entry_count != 3 {
        return Err(ReleaseSmokeError::DataMismatch("e2e_history"));
    }

    Ok(report)
}

fn app_config_from_app_data_dir(app_data_dir: PathBuf) -> Result<AppConfig, ReleaseSmokeError> {
    let app_data_dir = app_data_dir
        .to_str()
        .ok_or(ReleaseSmokeError::NonUtf8Path)?;
    let snapshot =
        ExternalEnvironmentSnapshot::from_pairs([("SPONZEY_CABINET_APP_DATA_DIR", app_data_dir)]);
    AppConfig::from_environment_snapshot(snapshot).map_err(ReleaseSmokeError::InvalidConfig)
}

fn run_first_run(config: &AppConfig) -> FirstRunInitializationOutcome {
    FirstRunInitializer::new(config.clone()).initialize(&mut LocalFirstRunStore::new())
}

fn run_initial_migration(config: &AppConfig) -> MigrationOutcome {
    MigrationRunner::new(MigrationPlan::initial()).run(&mut LocalMigrationStore::new(
        config.local_paths.metadata_dir.clone(),
    ))
}

fn run_startup_repair(
    config: &AppConfig,
) -> Result<(StartupRepairOutcome, bool), ReleaseSmokeError> {
    let mut state =
        transition_startup_repair(StartupRepairState::NotStarted, StartupRepairEvent::Start)
            .map(|transition| transition.next_state)
            .map_err(|_| ReleaseSmokeError::DataMismatch("startup_repair_transition"))?;

    if config.search.index_dir.is_dir() {
        return Err(ReleaseSmokeError::DataMismatch(
            "startup_repair_no_corruption",
        ));
    }

    state = transition_startup_repair(state, StartupRepairEvent::CorruptionDetected)
        .map(|transition| transition.next_state)
        .map_err(|_| ReleaseSmokeError::DataMismatch("startup_repair_transition"))?;
    repair_search_index_path(config)?;
    state = transition_startup_repair(state, StartupRepairEvent::IndexRepaired)
        .map(|transition| transition.next_state)
        .map_err(|_| ReleaseSmokeError::DataMismatch("startup_repair_transition"))?;

    let search_result_found = rebuild_fixture_search_projection(config)?;
    state = transition_startup_repair(state, StartupRepairEvent::ProjectionRebuilt)
        .map(|transition| transition.next_state)
        .map_err(|_| ReleaseSmokeError::DataMismatch("startup_repair_transition"))?;

    Ok((
        StartupRepairOutcome {
            final_state: state,
            corrupted_index_rebuilt: config.search.index_dir.is_dir() && search_result_found,
        },
        search_result_found,
    ))
}

fn corrupt_search_index_path(config: &AppConfig) -> Result<(), ReleaseSmokeError> {
    if config.search.index_dir.is_dir() {
        fs::remove_dir_all(&config.search.index_dir)
            .map_err(|_| ReleaseSmokeError::DataMismatch("startup_repair_corrupt_index"))?;
    } else if config.search.index_dir.exists() {
        fs::remove_file(&config.search.index_dir)
            .map_err(|_| ReleaseSmokeError::DataMismatch("startup_repair_corrupt_index"))?;
    }
    fs::write(&config.search.index_dir, b"corrupted index")
        .map_err(|_| ReleaseSmokeError::DataMismatch("startup_repair_corrupt_index"))
}

fn repair_search_index_path(config: &AppConfig) -> Result<(), ReleaseSmokeError> {
    if config.search.index_dir.is_dir() {
        return Ok(());
    }
    if config.search.index_dir.exists() {
        fs::remove_file(&config.search.index_dir)
            .map_err(|_| ReleaseSmokeError::DataMismatch("startup_repair_index_remove"))?;
    }
    fs::create_dir_all(&config.search.index_dir)
        .map_err(|_| ReleaseSmokeError::DataMismatch("startup_repair_index_create"))
}

fn changed_phase002_fixture() -> Phase002MigrationFixture {
    let mut records = Phase002MigrationFixture::self_host_sample()
        .records()
        .to_vec();
    records.push(
        Phase002FixtureRecord::new(
            Phase002FixtureRecordKind::AuditEvent,
            "audit-after-failure",
            vec![("event", "fixture.changed")],
            None,
        )
        .expect("changed fixture record"),
    );
    Phase002MigrationFixture::new(records).expect("changed fixture")
}

fn phase002_migration_product_log_sensitive_data_absent(
    fixture: &Phase002MigrationFixture,
    events: &[&MigrationProductEvent],
) -> bool {
    let rendered = events
        .iter()
        .map(|event| format!("{event:?}"))
        .collect::<Vec<_>>()
        .join("\n");
    fixture
        .sensitive_values()
        .iter()
        .all(|sensitive| !rendered.contains(sensitive))
}

fn seed_preservation_fixture(config: &AppConfig) -> Result<bool, ReleaseSmokeError> {
    let body_policy = smoke_body_policy();
    let mut documents = LocalDocumentRepository::with_body_policy(
        config.local_paths.workspace_root.clone(),
        body_policy,
    );
    let mut versions = LocalVersionStore::with_body_policy(
        config.local_paths.version_store_dir.clone(),
        body_policy,
    );
    let mut assets = LocalAssetStore::new(config.local_paths.asset_store_dir.clone());
    let mut document_assets =
        LocalDocumentAssetRepository::new(config.local_paths.metadata_dir.clone());
    let mut events = ReleaseSmokeEventPublisher::default();
    let mut product_log = ReleaseSmokeProductLogger::default();

    CreateDocumentUsecase::new(body_policy)
        .execute(
            CreateDocumentInput::new(
                FIXTURE_WORKSPACE_ID,
                FIXTURE_DOCUMENT_ID,
                FIXTURE_DOCUMENT_PATH,
                FIXTURE_INITIAL_BODY,
                FIXTURE_INITIAL_VERSION_ID,
                FIXTURE_INITIAL_SNAPSHOT_REF,
                FIXTURE_AUTHOR,
                "initial fixture",
            ),
            &mut documents,
            &mut versions,
            &mut events,
            &mut product_log,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;

    UpdateDocumentUsecase::new(body_policy)
        .execute(
            UpdateDocumentInput::new(
                FIXTURE_WORKSPACE_ID,
                FIXTURE_DOCUMENT_ID,
                FIXTURE_UPDATED_BODY,
                FIXTURE_UPDATED_VERSION_ID,
                FIXTURE_UPDATED_SNAPSHOT_REF,
                FIXTURE_AUTHOR,
                "updated fixture",
            ),
            &mut documents,
            &mut versions,
            &mut events,
            &mut product_log,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;

    AttachFileToDocumentUsecase::new()
        .execute(
            AttachFileToDocumentInput::new(
                FIXTURE_WORKSPACE_ID,
                FIXTURE_DOCUMENT_ID,
                FIXTURE_UPDATED_VERSION_ID,
                FIXTURE_ASSET_ID,
                FIXTURE_ASSET_FILE_NAME,
                FIXTURE_ASSET_MEDIA_TYPE,
                FIXTURE_ASSET_BYTES.to_vec(),
                FIXTURE_ASSET_LABEL,
            ),
            &documents,
            &mut assets,
            &mut document_assets,
            &mut events,
            &mut product_log,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;

    Ok(product_log.sensitive_data_absent())
}

fn read_preservation_fixture(
    config: &AppConfig,
) -> Result<PreservationReadback, ReleaseSmokeError> {
    let body_policy = smoke_body_policy();
    let documents = LocalDocumentRepository::with_body_policy(
        config.local_paths.workspace_root.clone(),
        body_policy,
    );
    let versions = LocalVersionStore::with_body_policy(
        config.local_paths.version_store_dir.clone(),
        body_policy,
    );
    let assets = LocalAssetStore::new(config.local_paths.asset_store_dir.clone());
    let document_assets =
        LocalDocumentAssetRepository::new(config.local_paths.metadata_dir.clone());

    let current = GetCurrentDocumentUsecase::new()
        .execute(
            GetCurrentDocumentInput::by_id(FIXTURE_WORKSPACE_ID, FIXTURE_DOCUMENT_ID),
            &documents,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    let current_document_preserved = current.record().body().as_str() == FIXTURE_UPDATED_BODY;

    let history = GetDocumentHistoryUsecase::new()
        .execute(
            GetDocumentHistoryInput::new(FIXTURE_WORKSPACE_ID, FIXTURE_DOCUMENT_ID, None, 10),
            &versions,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    let history_entry_count = history.page().entries().len();
    let version_history_preserved = history_entry_count == 2;

    let specific_version = GetDocumentVersionUsecase::new()
        .execute(
            GetDocumentVersionInput::new(
                FIXTURE_WORKSPACE_ID,
                FIXTURE_DOCUMENT_ID,
                FIXTURE_INITIAL_VERSION_ID,
            ),
            &versions,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    let specific_version_preserved =
        specific_version.snapshot().body().as_str() == FIXTURE_INITIAL_BODY;

    let document_asset_page = ListDocumentAssetsUsecase::new()
        .execute(
            ListDocumentAssetsInput::new(FIXTURE_WORKSPACE_ID, FIXTURE_DOCUMENT_ID),
            &documents,
            &document_assets,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    let asset_count = document_asset_page.assets().len();

    let workspace_id = WorkspaceId::new(FIXTURE_WORKSPACE_ID)
        .map_err(|_| ReleaseSmokeError::DataMismatch("workspace_id"))?;
    let asset_id = AssetId::from_sha256_hex(FIXTURE_ASSET_ID)
        .map_err(|_| ReleaseSmokeError::DataMismatch("asset_id"))?;
    let asset_metadata = assets
        .get_metadata(&workspace_id, &asset_id)
        .map_err(asset_store_error)?
        .ok_or(ReleaseSmokeError::DataMismatch("asset_metadata_missing"))?;
    let asset_metadata_preserved = asset_count == 1
        && asset_metadata.file_name().as_str() == FIXTURE_ASSET_FILE_NAME
        && asset_metadata.media_type().as_str() == FIXTURE_ASSET_MEDIA_TYPE
        && asset_metadata.byte_size() == FIXTURE_ASSET_BYTES.len() as u64;

    let asset_object = assets
        .get_object(&workspace_id, &asset_id)
        .map_err(asset_store_error)?
        .ok_or(ReleaseSmokeError::DataMismatch("asset_object_missing"))?;
    let asset_object_preserved = asset_object.bytes() == FIXTURE_ASSET_BYTES;

    Ok(PreservationReadback {
        current_document_preserved,
        version_history_preserved,
        specific_version_preserved,
        asset_metadata_preserved,
        asset_object_preserved,
        history_entry_count,
        asset_count,
    })
}

fn rebuild_fixture_search_projection(config: &AppConfig) -> Result<bool, ReleaseSmokeError> {
    let body_policy = smoke_body_policy();
    let documents = LocalDocumentRepository::with_body_policy(
        config.local_paths.workspace_root.clone(),
        body_policy,
    );
    let mut search_index = LocalSearchIndex::default();

    upsert_current_document_in_search(
        FIXTURE_WORKSPACE_ID,
        FIXTURE_DOCUMENT_ID,
        &documents,
        &mut search_index,
    )?;

    let search = SearchDocumentsUsecase::new()
        .execute(
            SearchDocumentsInput::new(FIXTURE_WORKSPACE_ID, "attachment", 10),
            &search_index,
        )
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;

    Ok(search
        .page()
        .results()
        .iter()
        .any(|result| result.document_id().as_str() == FIXTURE_DOCUMENT_ID))
}

fn upsert_current_document_in_search(
    workspace_id: &str,
    document_id: &str,
    documents: &LocalDocumentRepository,
    search_index: &mut LocalSearchIndex,
) -> Result<(), ReleaseSmokeError> {
    let workspace_id = WorkspaceId::new(workspace_id)
        .map_err(|_| ReleaseSmokeError::DataMismatch("workspace_id"))?;
    let document_id =
        DocumentId::new(document_id).map_err(|_| ReleaseSmokeError::DataMismatch("document_id"))?;
    let current = documents
        .get_current_by_id(&workspace_id, &document_id)
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?
        .ok_or(ReleaseSmokeError::DataMismatch("current_document_missing"))?;
    let record = SearchDocumentRecord::new(
        current.document_id().clone(),
        current.metadata().title().clone(),
        current.path().clone(),
        current.body().clone(),
    );
    search_index
        .upsert_document(&workspace_id, record)
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))
}

fn replace_source_link_projection(
    workspace_id: &str,
    parsed: &ParsedMarkdown,
    link_index: &mut LocalLinkIndex,
) -> Result<(), ReleaseSmokeError> {
    let workspace_id = WorkspaceId::new(workspace_id)
        .map_err(|_| ReleaseSmokeError::DataMismatch("workspace_id"))?;
    let source_document_id = DocumentId::new(E2E_SOURCE_DOCUMENT_ID)
        .map_err(|_| ReleaseSmokeError::DataMismatch("source_document_id"))?;
    let target_document_id = DocumentId::new(E2E_TARGET_DOCUMENT_ID)
        .map_err(|_| ReleaseSmokeError::DataMismatch("target_document_id"))?;
    let backlinks = parsed
        .wikilinks()
        .iter()
        .filter(|link| link.target() == E2E_TARGET_TITLE)
        .map(|link| {
            Backlink::new(
                source_document_id.clone(),
                target_document_id.clone(),
                link.source_range(),
            )
        })
        .collect::<Vec<_>>();
    let record = LinkProjectionRecord::new(source_document_id, backlinks, Vec::new())
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))?;
    link_index
        .replace_document_links(&workspace_id, record)
        .map_err(|error| ReleaseSmokeError::UsecaseFailed(error.code()))
}

fn smoke_body_policy() -> DocumentBodyPolicy {
    DocumentBodyPolicy::new(64 * 1024).expect("release smoke body policy should be valid")
}

fn asset_store_error(error: AssetStoreError) -> ReleaseSmokeError {
    ReleaseSmokeError::AssetStoreFailed(error.code())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreservationReadback {
    current_document_preserved: bool,
    version_history_preserved: bool,
    specific_version_preserved: bool,
    asset_metadata_preserved: bool,
    asset_object_preserved: bool,
    history_entry_count: usize,
    asset_count: usize,
}

#[derive(Debug, Default)]
struct ReleaseSmokeEventPublisher {
    events: Vec<DocumentChangeEvent>,
}

impl DocumentChangeEventPublisher for ReleaseSmokeEventPublisher {
    fn publish(&mut self, event: DocumentChangeEvent) {
        self.events.push(event);
    }
}

#[derive(Debug, Default)]
struct ReleaseSmokeProductLogger {
    events: Vec<CreateDocumentProductEvent>,
}

impl ReleaseSmokeProductLogger {
    fn sensitive_data_absent(&self) -> bool {
        self.events.iter().all(|event| {
            let rendered = format!("{event:?}");
            !rendered.contains(FIXTURE_INITIAL_BODY)
                && !rendered.contains(FIXTURE_UPDATED_BODY)
                && !rendered.contains("cabinet fixture asset")
        })
    }
}

impl DocumentProductLogger for ReleaseSmokeProductLogger {
    fn write_product(&mut self, event: CreateDocumentProductEvent) {
        self.events.push(event);
    }
}
