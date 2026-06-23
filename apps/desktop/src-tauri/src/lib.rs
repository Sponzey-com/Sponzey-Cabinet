//! Desktop shell boundary.
//!
//! This crate is intentionally thin. Future Tauri commands should map request
//! DTOs into platform boundary calls without embedding business rules.

/// Request DTO accepted at the desktop shell boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopShellRequest {
    pub command: String,
}

/// Response DTO returned from the desktop shell boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopShellResponse {
    pub boundary: &'static str,
    pub command: String,
}

/// Routes a command through the desktop shell boundary.
pub fn route_desktop_command(request: DesktopShellRequest) -> DesktopShellResponse {
    DesktopShellResponse {
        boundary: cabinet_platform::layer_name(),
        command: request.command,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_shell_routes_to_platform_boundary() {
        let response = route_desktop_command(DesktopShellRequest {
            command: "open_workspace".to_string(),
        });

        assert_eq!(response.boundary, "platform");
        assert_eq!(response.command, "open_workspace");
    }
}
