use cabinet_server::adapter::{HttpMethod, ServerRequest, handle_request};
use cabinet_server::bootstrap::{ProcessEnvironmentSource, ServerBootstrapReader};
use cabinet_server::composition::build_server_composition;
use cabinet_server::e2e_http::run_self_host_e2e_http_server;
use cabinet_server::health::{NoopServerHealthProductLogger, build_local_dev_health_target};
use cabinet_server::package_smoke::{
    SelfHostServerPackageSmokeInput, run_self_host_server_package_smoke,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let run_e2e_http_server = args.iter().any(|argument| argument == "--e2e-http-server");
    let run_package_smoke = args
        .iter()
        .any(|argument| argument == "--self-host-package-smoke");
    let environment_source = ProcessEnvironmentSource;
    let config_input = ServerBootstrapReader::new(&environment_source)
        .read_once()
        .into_config_input();
    let config = match config_input.validate() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("product_log_event=server.config.validation_failed");
            eprintln!("error_code={}", error.code_str());
            eprintln!("message={}", error.public_message());
            std::process::exit(2);
        }
    };
    if run_e2e_http_server {
        if let Err(error) = run_self_host_e2e_http_server(config) {
            eprintln!("product_log_event=server.failed");
            eprintln!("error_code=SERVER_E2E_HTTP_FAILED");
            eprintln!("message={error}");
            std::process::exit(3);
        }
        return;
    }

    if run_package_smoke {
        match run_self_host_server_package_smoke(SelfHostServerPackageSmokeInput::new(config)) {
            Ok(report) => {
                println!("server_package_smoke=passed");
                println!("framework={}", report.framework());
                println!("route_count={}", report.route_count());
                println!("health_status_code={}", report.health_status_code());
                println!(
                    "default_profile_without_external_services={}",
                    report.default_profile_without_external_services()
                );
                println!(
                    "sensitive_output_absent={}",
                    report.sensitive_output_absent()
                );
            }
            Err(error) => {
                eprintln!("server_package_smoke=failed");
                eprintln!("error_code=SERVER_PACKAGE_SMOKE_FAILED");
                eprintln!("message={error:?}");
                std::process::exit(4);
            }
        }
        return;
    }

    let composition = build_server_composition(config.clone());
    let target =
        build_local_dev_health_target(config.clone(), NoopServerHealthProductLogger::default());
    let health_response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(HttpMethod::Get, "/api/health", None),
    )
    .expect("local health route is registered");

    println!("Sponzey Cabinet self-host server boundary");
    println!("product_log_event=server.started");
    println!("framework={}", composition.framework().as_str());
    println!("bind_address={}", composition.config().bind_address());
    println!("public_url={}", composition.config().public_url());
    println!("route_count={}", composition.routes().routes().len());

    for route in composition.routes().routes() {
        println!(
            "route={} {} handler={}",
            route.method().as_str(),
            route.path(),
            route.route_id()
        );
    }

    println!("health_status_code={}", health_response.status_code());
    println!("health_body={}", health_response.body());
}
