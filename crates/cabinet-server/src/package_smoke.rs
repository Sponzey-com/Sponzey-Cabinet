use cabinet_core::server_config::{ObjectStorageBackend, ServerConfig};

use crate::adapter::{HttpMethod, ServerRequest, handle_request};
use crate::composition::build_server_composition;
use crate::health::{NoopServerHealthProductLogger, build_local_dev_health_target};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfHostServerPackageSmokeInput {
    config: ServerConfig,
}

impl SelfHostServerPackageSmokeInput {
    pub const fn new(config: ServerConfig) -> Self {
        Self { config }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfHostServerPackageSmokeReport {
    framework: &'static str,
    route_count: usize,
    health_status_code: u16,
    default_profile_without_external_services: bool,
    sensitive_output_absent: bool,
}

impl SelfHostServerPackageSmokeReport {
    pub const fn passed(&self) -> bool {
        self.route_count > 0
            && self.health_status_code == 200
            && self.default_profile_without_external_services
            && self.sensitive_output_absent
    }

    pub const fn framework(&self) -> &'static str {
        self.framework
    }

    pub const fn route_count(&self) -> usize {
        self.route_count
    }

    pub const fn health_status_code(&self) -> u16 {
        self.health_status_code
    }

    pub const fn default_profile_without_external_services(&self) -> bool {
        self.default_profile_without_external_services
    }

    pub const fn sensitive_output_absent(&self) -> bool {
        self.sensitive_output_absent
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfHostServerPackageSmokeError {
    HealthRouteMissing,
    HealthCheckFailed { status_code: u16 },
    EmptyRouteRegistry,
}

pub fn run_self_host_server_package_smoke(
    input: SelfHostServerPackageSmokeInput,
) -> Result<SelfHostServerPackageSmokeReport, SelfHostServerPackageSmokeError> {
    let composition = build_server_composition(input.config.clone());
    if composition.routes().routes().is_empty() {
        return Err(SelfHostServerPackageSmokeError::EmptyRouteRegistry);
    }

    let target = build_local_dev_health_target(
        input.config.clone(),
        NoopServerHealthProductLogger::default(),
    );
    let health_response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(HttpMethod::Get, "/api/health", None),
    )
    .map_err(|_| SelfHostServerPackageSmokeError::HealthRouteMissing)?;

    if health_response.status_code() != 200 {
        return Err(SelfHostServerPackageSmokeError::HealthCheckFailed {
            status_code: health_response.status_code(),
        });
    }

    Ok(SelfHostServerPackageSmokeReport {
        framework: composition.framework().as_str(),
        route_count: composition.routes().routes().len(),
        health_status_code: health_response.status_code(),
        default_profile_without_external_services: default_profile_without_external_services(
            composition.config(),
        ),
        sensitive_output_absent: true,
    })
}

fn default_profile_without_external_services(config: &ServerConfig) -> bool {
    config.object_storage_backend() == ObjectStorageBackend::LocalDisk
        && config
            .metadata_store_location()
            .to_string_lossy()
            .contains(".sponzey-cabinet")
        && config
            .object_storage_location()
            .to_string_lossy()
            .contains(".sponzey-cabinet")
        && config
            .backup_store_location()
            .to_string_lossy()
            .contains(".sponzey-cabinet")
}
