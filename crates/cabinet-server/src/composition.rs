use cabinet_core::server_config::ServerConfig;

use crate::adapter::{HttpMethod, RouteRegistry};
use crate::runtime::{HandlerRegistry, RuntimeDependencyManifest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerFramework {
    AxumTokio,
}

impl ServerFramework {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AxumTokio => "axum-tokio",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerComposition {
    framework: ServerFramework,
    config: ServerConfig,
    routes: RouteRegistry,
    handlers: HandlerRegistry,
    runtime_dependencies: RuntimeDependencyManifest,
}

impl ServerComposition {
    pub const fn framework(&self) -> ServerFramework {
        self.framework
    }

    pub const fn routes(&self) -> &RouteRegistry {
        &self.routes
    }

    pub const fn config(&self) -> &ServerConfig {
        &self.config
    }

    pub const fn handlers(&self) -> &HandlerRegistry {
        &self.handlers
    }

    pub const fn runtime_dependencies(&self) -> &RuntimeDependencyManifest {
        &self.runtime_dependencies
    }
}

pub fn build_server_composition(config: ServerConfig) -> ServerComposition {
    let routes = phase002_routes();
    let handlers = HandlerRegistry::from_routes(&routes);
    ServerComposition {
        framework: ServerFramework::AxumTokio,
        config,
        routes,
        handlers,
        runtime_dependencies: RuntimeDependencyManifest::phase003_self_host(),
    }
}

fn phase002_routes() -> RouteRegistry {
    RouteRegistry::new()
        .with_route(HttpMethod::Get, "/api/health", "health.check")
        .with_route(
            HttpMethod::Post,
            "/api/documents/{documentId}/review-requests",
            "review.request_document",
        )
        .with_route(
            HttpMethod::Post,
            "/api/review-requests/{reviewRequestId}/approve",
            "review.approve_document",
        )
        .with_route(
            HttpMethod::Post,
            "/api/review-requests/{reviewRequestId}/reject",
            "review.reject_document",
        )
        .with_route(
            HttpMethod::Post,
            "/api/documents/{documentId}/publish",
            "review.publish_document",
        )
        .with_route(
            HttpMethod::Get,
            "/api/review-requests",
            "review.list_requests",
        )
        .with_route(
            HttpMethod::Post,
            "/api/documents/{documentId}/locks",
            "document_lock.lock",
        )
        .with_route(
            HttpMethod::Delete,
            "/api/documents/{documentId}/locks/current",
            "document_lock.unlock",
        )
        .with_route(
            HttpMethod::Get,
            "/api/documents/{documentId}/locks/current",
            "document_lock.get",
        )
        .with_route(HttpMethod::Get, "/api/audit-events", "audit.list_events")
        .with_route(
            HttpMethod::Post,
            "/api/field-debug-sessions",
            "field_debug.request_session",
        )
        .with_route(
            HttpMethod::Post,
            "/api/field-debug-sessions/{sessionId}/approve",
            "field_debug.approve_session",
        )
        .with_route(
            HttpMethod::Post,
            "/api/field-debug-sessions/{sessionId}/expire",
            "field_debug.expire_session",
        )
        .with_route(HttpMethod::Post, "/api/backups", "backup.create")
        .with_route(HttpMethod::Get, "/api/backups/{jobId}", "backup.get_status")
        .with_route(
            HttpMethod::Post,
            "/api/backups/{jobId}/restore",
            "backup.restore",
        )
        .with_route(HttpMethod::Post, "/api/exports", "export.create_workspace")
        .with_route(HttpMethod::Get, "/api/exports/{jobId}", "export.get_status")
        .with_route(HttpMethod::Post, "/api/auth/login", "auth.login")
        .with_route(
            HttpMethod::Post,
            "/api/auth/session/validate",
            "auth.validate_session",
        )
        .with_route(HttpMethod::Get, "/api/users", "user.list")
        .with_route(
            HttpMethod::Get,
            "/api/workspaces/{workspaceId}/groups",
            "group.list",
        )
        .with_route(
            HttpMethod::Post,
            "/api/workspaces/{workspaceId}/groups/{groupId}/members",
            "group.add_member",
        )
        .with_route(
            HttpMethod::Delete,
            "/api/workspaces/{workspaceId}/groups/{groupId}/members/{userId}",
            "group.remove_member",
        )
        .with_route(
            HttpMethod::Get,
            "/api/workspaces/{workspaceId}/roles",
            "role.list_assignments",
        )
        .with_route(
            HttpMethod::Post,
            "/api/workspaces/{workspaceId}/roles",
            "role.assign",
        )
        .with_route(
            HttpMethod::Delete,
            "/api/workspaces/{workspaceId}/roles/{assignmentId}",
            "role.revoke",
        )
        .with_route(
            HttpMethod::Get,
            "/api/workspaces/{workspaceId}/documents/{documentId}/current",
            "document.get_accessible_current",
        )
        .with_route(
            HttpMethod::Put,
            "/api/workspaces/{workspaceId}/documents/{documentId}/current",
            "document.save_remote_current",
        )
        .with_route(
            HttpMethod::Get,
            "/api/workspaces/{workspaceId}/documents/{documentId}/history",
            "document.get_accessible_history",
        )
        .with_route(
            HttpMethod::Get,
            "/api/workspaces/{workspaceId}/search",
            "search.accessible",
        )
        .with_route(
            HttpMethod::Get,
            "/api/workspaces/{workspaceId}/documents/{documentId}/graph",
            "graph.get_local",
        )
        .with_route(
            HttpMethod::Post,
            "/api/workspaces/{workspaceId}/canvases",
            "canvas.create",
        )
        .with_route(
            HttpMethod::Post,
            "/api/workspaces/{workspaceId}/canvases/{canvasId}/nodes",
            "canvas.add_node",
        )
        .with_route(
            HttpMethod::Post,
            "/api/workspaces/{workspaceId}/documents/{documentId}/canvas-embeds",
            "canvas.embed",
        )
        .with_route(
            HttpMethod::Post,
            "/api/workspaces/{workspaceId}/documents/{documentId}/collaboration/join",
            "collaboration.join_document_room",
        )
        .with_route(
            HttpMethod::Post,
            "/api/workspaces/{workspaceId}/documents/{documentId}/collaboration/operations",
            "collaboration.broadcast_operation",
        )
        .with_route(
            HttpMethod::Post,
            "/api/workspaces/{workspaceId}/documents/{documentId}/collaboration/presence",
            "collaboration.broadcast_presence",
        )
        .with_route(
            HttpMethod::Post,
            "/api/workspaces/{workspaceId}/documents/{documentId}/collaboration/replay",
            "collaboration.request_replay",
        )
        .with_route(
            HttpMethod::Get,
            "/api/documents/{documentId}/sharing",
            "sharing.get_document",
        )
        .with_route(
            HttpMethod::Put,
            "/api/documents/{documentId}/sharing",
            "sharing.update_document",
        )
        .with_route(
            HttpMethod::Get,
            "/api/documents/{documentId}/comments",
            "comment.list",
        )
        .with_route(
            HttpMethod::Post,
            "/api/documents/{documentId}/comments",
            "comment.add",
        )
        .with_route(
            HttpMethod::Post,
            "/api/documents/{documentId}/inline-comments",
            "comment.add_inline",
        )
        .with_route(
            HttpMethod::Post,
            "/api/comments/{commentId}/resolve",
            "comment.resolve",
        )
        .with_route(
            HttpMethod::Post,
            "/api/comments/{commentId}/reopen",
            "comment.reopen",
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition_registers_only_boundary_routes() {
        let config = cabinet_core::server_config::ServerConfigInput::local_dev_defaults()
            .validate()
            .expect("valid default config");
        let composition = build_server_composition(config);

        assert_eq!(composition.framework(), ServerFramework::AxumTokio);
        assert_eq!(composition.routes().routes().len(), 46);
        assert!(
            composition
                .routes()
                .contains(HttpMethod::Get, "/api/health")
        );
    }
}
