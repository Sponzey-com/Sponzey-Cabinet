use cabinet_core::server_config::ServerConfigInput;
use cabinet_server::adapter::HttpMethod;
use cabinet_server::composition::build_server_composition;
use cabinet_server::runtime::HandlerKind;

#[test]
fn canvas_runtime_registers_command_routes_and_handler_kinds() {
    let composition = build_server_composition(
        ServerConfigInput::local_dev_defaults()
            .validate()
            .expect("default config"),
    );
    let routes = composition.routes();
    let handlers = composition.handlers();

    assert!(routes.contains(HttpMethod::Post, "/api/workspaces/workspace-1/canvases"));
    assert!(routes.contains(
        HttpMethod::Post,
        "/api/workspaces/workspace-1/canvases/canvas-1/nodes"
    ));
    assert!(routes.contains(
        HttpMethod::Post,
        "/api/workspaces/workspace-1/documents/doc-1/canvas-embeds"
    ));
    assert_eq!(
        handlers.kind("canvas.create"),
        Some(HandlerKind::CanvasCreate)
    );
    assert_eq!(
        handlers.kind("canvas.add_node"),
        Some(HandlerKind::CanvasAddNode)
    );
    assert_eq!(
        handlers.kind("canvas.embed"),
        Some(HandlerKind::CanvasEmbed)
    );
}
