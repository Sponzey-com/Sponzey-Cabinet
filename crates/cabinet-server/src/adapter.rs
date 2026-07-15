use crate::errors::{ServerBoundaryError, ServerErrorCode};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Delete,
    Get,
    Post,
    Put,
}

impl HttpMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Delete => "DELETE",
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteSpec {
    method: HttpMethod,
    path: &'static str,
    route_id: &'static str,
}

impl RouteSpec {
    pub const fn new(method: HttpMethod, path: &'static str, route_id: &'static str) -> Self {
        Self {
            method,
            path,
            route_id,
        }
    }

    pub const fn method(&self) -> HttpMethod {
        self.method
    }

    pub const fn path(&self) -> &'static str {
        self.path
    }

    pub const fn route_id(&self) -> &'static str {
        self.route_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RouteRegistry {
    routes: Vec<RouteSpec>,
}

impl RouteRegistry {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    pub fn with_route(
        mut self,
        method: HttpMethod,
        path: &'static str,
        route_id: &'static str,
    ) -> Self {
        self.routes.push(RouteSpec::new(method, path, route_id));
        self
    }

    pub fn contains(&self, method: HttpMethod, path: &str) -> bool {
        self.find(method, path).is_some()
    }

    pub fn routes(&self) -> &[RouteSpec] {
        &self.routes
    }

    fn find(&self, method: HttpMethod, path: &str) -> Option<&RouteSpec> {
        self.routes
            .iter()
            .find(|route| route.method == method && route_matches(route.path, path).is_some())
    }

    fn has_path(&self, path: &str) -> bool {
        self.routes
            .iter()
            .any(|route| route_matches(route.path, path).is_some())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerRequest {
    method: HttpMethod,
    path: String,
    body: Option<String>,
}

impl ServerRequest {
    pub fn new(method: HttpMethod, path: &str, body: Option<&str>) -> Self {
        Self {
            method,
            path: path.to_string(),
            body: body.map(str::to_string),
        }
    }

    pub const fn method(&self) -> HttpMethod {
        self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    fn route_path(&self) -> &str {
        self.path
            .split_once('?')
            .map_or(self.path.as_str(), |(path, _)| path)
    }

    fn query_string(&self) -> Option<&str> {
        self.path
            .split_once('?')
            .map(|(_, query_string)| query_string)
            .filter(|query_string| !query_string.is_empty())
    }

    pub fn body(&self) -> Option<&str> {
        self.body.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsecaseInputDto {
    route_id: String,
    body: Option<String>,
    path_params: BTreeMap<String, String>,
    query_params: BTreeMap<String, String>,
}

impl UsecaseInputDto {
    pub fn new(route_id: &str, body: Option<&str>) -> Self {
        Self::new_with_path_params(route_id, body, BTreeMap::new())
    }

    pub fn new_with_path_params(
        route_id: &str,
        body: Option<&str>,
        path_params: BTreeMap<String, String>,
    ) -> Self {
        Self::new_with_params(route_id, body, path_params, BTreeMap::new())
    }

    pub fn new_with_params(
        route_id: &str,
        body: Option<&str>,
        path_params: BTreeMap<String, String>,
        query_params: BTreeMap<String, String>,
    ) -> Self {
        Self {
            route_id: route_id.to_string(),
            body: body.map(str::to_string),
            path_params,
            query_params,
        }
    }

    pub fn route_id(&self) -> &str {
        &self.route_id
    }

    pub fn body(&self) -> Option<&str> {
        self.body.as_deref()
    }

    pub fn path_param(&self, name: &str) -> Option<&str> {
        self.path_params.get(name).map(String::as_str)
    }

    pub fn path_params(&self) -> &BTreeMap<String, String> {
        &self.path_params
    }

    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.query_params.get(name).map(String::as_str)
    }

    pub fn query_params(&self) -> &BTreeMap<String, String> {
        &self.query_params
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsecaseOutputDto {
    status_code: u16,
    body: String,
}

impl UsecaseOutputDto {
    pub fn new(status_code: u16, body: &str) -> Self {
        Self {
            status_code,
            body: body.to_string(),
        }
    }

    pub const fn status_code(&self) -> u16 {
        self.status_code
    }

    pub fn body(&self) -> &str {
        &self.body
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerResponse {
    status_code: u16,
    body: String,
}

impl ServerResponse {
    pub const fn status_code(&self) -> u16 {
        self.status_code
    }

    pub fn body(&self) -> &str {
        &self.body
    }
}

pub struct BoundaryMapper<'routes> {
    routes: &'routes RouteRegistry,
}

impl<'routes> BoundaryMapper<'routes> {
    pub const fn new(routes: &'routes RouteRegistry) -> Self {
        Self { routes }
    }

    pub fn request_to_usecase(
        &self,
        request: ServerRequest,
    ) -> Result<UsecaseInputDto, ServerBoundaryError> {
        let route_path = request.route_path();
        if let Some(route) = self.routes.find(request.method(), route_path) {
            let path_params = route_matches(route.path(), route_path).unwrap_or_default();
            let query_params = parse_query_params(request.query_string());
            return Ok(UsecaseInputDto::new_with_params(
                route.route_id(),
                request.body(),
                path_params,
                query_params,
            ));
        }

        if self.routes.has_path(route_path) {
            return Err(ServerBoundaryError::new(
                ServerErrorCode::MethodNotAllowed,
                format!(
                    "{} is not allowed for {}",
                    request.method().as_str(),
                    route_path
                ),
            ));
        }

        Err(ServerBoundaryError::new(
            ServerErrorCode::RouteNotFound,
            format!("route not found: {}", request.path()),
        ))
    }

    pub fn usecase_to_response(&self, output: UsecaseOutputDto) -> ServerResponse {
        ServerResponse {
            status_code: output.status_code(),
            body: output.body().to_string(),
        }
    }
}

fn route_matches(template: &str, path: &str) -> Option<BTreeMap<String, String>> {
    let template_segments = path_segments(template);
    let path_segments = path_segments(path);
    if template_segments.len() != path_segments.len() {
        return None;
    }

    let mut params = BTreeMap::new();
    for (template_segment, path_segment) in template_segments.iter().zip(path_segments.iter()) {
        if let Some(param_name) = path_param_name(template_segment) {
            if path_segment.is_empty() {
                return None;
            }
            params.insert(param_name.to_string(), (*path_segment).to_string());
        } else if template_segment != path_segment {
            return None;
        }
    }

    Some(params)
}

fn path_segments(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn path_param_name(segment: &str) -> Option<&str> {
    segment
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .filter(|name| !name.is_empty())
}

fn parse_query_params(query_string: Option<&str>) -> BTreeMap<String, String> {
    let mut params = BTreeMap::new();
    let Some(query_string) = query_string else {
        return params;
    };

    for pair in query_string.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair
            .split_once('=')
            .map_or((pair, ""), |(key, value)| (key, value));
        if key.is_empty() {
            continue;
        }
        params.insert(key.to_string(), value.to_string());
    }

    params
}

pub trait ServerUsecaseTarget {
    fn handle(&self, input: UsecaseInputDto) -> Result<UsecaseOutputDto, ServerBoundaryError>;
}

pub fn handle_request(
    routes: &RouteRegistry,
    target: &impl ServerUsecaseTarget,
    request: ServerRequest,
) -> Result<ServerResponse, ServerBoundaryError> {
    let mapper = BoundaryMapper::new(routes);
    let input = mapper.request_to_usecase(request)?;
    let output = target.handle(input)?;
    Ok(mapper.usecase_to_response(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_registry_finds_method_and_path_together() {
        let routes = RouteRegistry::new().with_route(HttpMethod::Get, "/api/health", "health");

        assert!(routes.contains(HttpMethod::Get, "/api/health"));
        assert!(!routes.contains(HttpMethod::Post, "/api/health"));
    }
}
