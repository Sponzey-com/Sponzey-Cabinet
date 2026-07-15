use std::collections::BTreeMap;
use std::path::PathBuf;

const APP_DATA_DIR_KEY: &str = "SPONZEY_CABINET_APP_DATA_DIR";
const WORKSPACE_ROOT_KEY: &str = "SPONZEY_CABINET_WORKSPACE_ROOT";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub local_paths: LocalPathsConfig,
    pub logging: LoggingConfig,
    pub storage: StorageConfig,
    pub search: SearchConfig,
}

impl AppConfig {
    pub fn from_environment_snapshot(
        snapshot: ExternalEnvironmentSnapshot,
    ) -> Result<Self, ConfigError> {
        let local_paths = LocalPathsConfig::from_snapshot(&snapshot)?;
        Ok(Self {
            storage: StorageConfig {
                metadata_dir: local_paths.metadata_dir.clone(),
                version_store_dir: local_paths.version_store_dir.clone(),
                asset_store_dir: local_paths.asset_store_dir.clone(),
            },
            search: SearchConfig {
                index_dir: local_paths.search_index_dir.clone(),
            },
            local_paths,
            logging: LoggingConfig::default(),
        })
    }
}

pub type LocalDesktopConfig = AppConfig;

pub trait ExternalEnvironmentReader {
    fn read_environment_snapshot(&mut self) -> ExternalEnvironmentSnapshot;
}

pub fn bootstrap_local_desktop_config_from_reader<R: ExternalEnvironmentReader>(
    reader: &mut R,
) -> Result<LocalDesktopConfig, ConfigError> {
    let snapshot = reader.read_environment_snapshot();
    LocalDesktopConfig::from_environment_snapshot(snapshot)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapConfigInput {
    environment: ExternalEnvironmentSnapshot,
}

impl BootstrapConfigInput {
    pub fn new(environment: ExternalEnvironmentSnapshot) -> Self {
        Self { environment }
    }

    pub fn into_app_config(self) -> Result<AppConfig, ConfigError> {
        AppConfig::from_environment_snapshot(self.environment)
    }

    pub fn into_local_desktop_config(self) -> Result<LocalDesktopConfig, ConfigError> {
        LocalDesktopConfig::from_environment_snapshot(self.environment)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPathsConfig {
    pub app_data_dir: PathBuf,
    pub workspace_root: PathBuf,
    pub metadata_dir: PathBuf,
    pub version_store_dir: PathBuf,
    pub asset_store_dir: PathBuf,
    pub search_index_dir: PathBuf,
}

impl LocalPathsConfig {
    fn from_snapshot(snapshot: &ExternalEnvironmentSnapshot) -> Result<Self, ConfigError> {
        let app_data_dir = required_path(snapshot, APP_DATA_DIR_KEY)?;
        let workspace_root = match snapshot.get(WORKSPACE_ROOT_KEY) {
            Some(value) => validate_path(WORKSPACE_ROOT_KEY, value)?,
            None => app_data_dir.join("workspaces"),
        };

        Ok(Self {
            metadata_dir: app_data_dir.join("metadata"),
            version_store_dir: app_data_dir.join("version-store"),
            asset_store_dir: app_data_dir.join("assets"),
            search_index_dir: app_data_dir.join("search-index"),
            app_data_dir,
            workspace_root,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggingConfig {
    pub product_log_enabled: bool,
    pub field_debug_enabled: bool,
    pub development_log_enabled: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            product_log_enabled: true,
            field_debug_enabled: false,
            development_log_enabled: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageConfig {
    pub metadata_dir: PathBuf,
    pub version_store_dir: PathBuf,
    pub asset_store_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchConfig {
    pub index_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalEnvironmentSnapshot {
    values: BTreeMap<String, String>,
}

impl ExternalEnvironmentSnapshot {
    pub fn from_pairs<const N: usize>(pairs: [(&str, &str); N]) -> Self {
        let values = pairs
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        Self { values }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    MissingRequiredValue(&'static str),
    InvalidValue(&'static str),
}

fn required_path(
    snapshot: &ExternalEnvironmentSnapshot,
    key: &'static str,
) -> Result<PathBuf, ConfigError> {
    let value = snapshot
        .get(key)
        .ok_or(ConfigError::MissingRequiredValue(key))?;
    validate_path(key, value)
}

fn validate_path(key: &'static str, value: &str) -> Result<PathBuf, ConfigError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ConfigError::InvalidValue(key));
    }
    Ok(PathBuf::from(trimmed))
}
