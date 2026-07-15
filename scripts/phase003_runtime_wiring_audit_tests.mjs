import assert from "node:assert/strict";
import test from "node:test";

import {
  RuntimeWiringAuditErrorCode,
  RuntimeWiringAuditEvent,
  RuntimeWiringAuditState,
  analyzeRuntimeWiringSources,
  renderRuntimeWiringAuditMarkdown,
  transitionRuntimeWiringAuditState,
} from "./phase003_runtime_wiring_audit.mjs";

const completeFixture = {
  compositionSource: `
fn phase002_routes() -> RouteRegistry {
  RouteRegistry::new()
    .with_route(HttpMethod::Get, "/api/health", "health.check")
    .with_route(HttpMethod::Get, "/api/workspaces/{workspaceId}/documents/{documentId}/current", "document.get_accessible_current")
    .with_route(HttpMethod::Post, "/api/auth/login", "auth.login")
    .with_route(HttpMethod::Post, "/api/comments/{commentId}/resolve", "comment.resolve")
}
`,
  runtimeSource: `
impl HandlerKind {
  fn for_route_id(route_id: &str) -> Self {
    match route_id {
      "health.check" => Self::Health,
      "document.get_accessible_current" => Self::CurrentDocument,
      _ => Self::NotImplemented,
    }
  }
}
impl Target {
  fn handle(&self, input: UsecaseInputDto) -> Result<UsecaseOutputDto, ServerBoundaryError> {
    match HandlerKind::for_route_id(input.route_id()) {
      HandlerKind::CurrentDocument => Ok(self.handle_current_document(&input)),
      HandlerKind::Health | HandlerKind::NotImplemented => Ok(error_output(501, "SERVER_HANDLER_NOT_IMPLEMENTED"))
    }
  }
}
`,
  healthSource: `
impl ServerUsecaseTarget for HealthRouteTarget {
  fn handle(&self, input: UsecaseInputDto) -> Result<UsecaseOutputDto, ServerBoundaryError> {
    if input.route_id() != "health.check" { return Err(err); }
  }
}
`,
  e2eHttpSource: `
if request.method == "POST" && request.path() == "/api/auth/login" {
  return E2eRouteResult::json(200, "{}");
}
`,
};

test("runtime wiring audit classifies runtime, dedicated target, stub-only, and missing routes", () => {
  const audit = analyzeRuntimeWiringSources(completeFixture);

  assert.equal(audit.summary.totalRoutes, 4);
  assert.equal(audit.summary.runtimeWiredRoutes, 1);
  assert.equal(audit.summary.dedicatedTargetRoutes, 1);
  assert.equal(audit.summary.productSmokeStubOnlyRoutes, 1);
  assert.equal(audit.summary.missingRuntimeRoutes, 1);
  assert.equal(audit.findings[0].id, "PHASE003_RUNTIME_WIRING_GAP");
  assert.equal(audit.findings[0].errorCode, RuntimeWiringAuditErrorCode.MissingRuntimeHandlers);
  assert.equal(audit.nextImplementationTarget.routeId, "auth.login");
  assert.equal(
    audit.routes.find((route) => route.routeId === "document.get_accessible_current").status,
    "runtime wired",
  );
  assert.equal(
    audit.routes.find((route) => route.routeId === "health.check").status,
    "runtime wired through dedicated target",
  );
  assert.equal(
    audit.routes.find((route) => route.routeId === "auth.login").status,
    "product smoke stub only",
  );
  assert.equal(
    audit.routes.find((route) => route.routeId === "comment.resolve").status,
    "contract complete only",
  );
});

test("runtime wiring audit fails with stable error code when route registry is empty", () => {
  assert.throws(
    () =>
      analyzeRuntimeWiringSources({
        ...completeFixture,
        compositionSource: "fn phase002_routes() -> RouteRegistry { RouteRegistry::new() }",
      }),
    (error) => error.code === RuntimeWiringAuditErrorCode.RouteRegistryEmpty,
  );
});

test("runtime wiring audit state machine rejects invalid transitions", () => {
  assert.equal(
    transitionRuntimeWiringAuditState(
      RuntimeWiringAuditState.NotStarted,
      RuntimeWiringAuditEvent.Start,
    ),
    RuntimeWiringAuditState.ReadingSource,
  );
  assert.equal(
    transitionRuntimeWiringAuditState(
      RuntimeWiringAuditState.ReadingSource,
      RuntimeWiringAuditEvent.SourceLoaded,
    ),
    RuntimeWiringAuditState.Auditing,
  );
  assert.equal(
    transitionRuntimeWiringAuditState(
      RuntimeWiringAuditState.Auditing,
      RuntimeWiringAuditEvent.AuditComplete,
    ),
    RuntimeWiringAuditState.Reported,
  );
  assert.throws(
    () =>
      transitionRuntimeWiringAuditState(
        RuntimeWiringAuditState.NotStarted,
        RuntimeWiringAuditEvent.ReportWritten,
      ),
    (error) => error.code === RuntimeWiringAuditErrorCode.InvalidTransition,
  );
});

test("runtime wiring audit markdown keeps phase terms and next task target explicit", () => {
  const audit = analyzeRuntimeWiringSources(completeFixture);
  const markdown = renderRuntimeWiringAuditMarkdown(audit);

  assert.match(markdown, /# Phase 003 Runtime Wiring Audit/);
  assert.match(markdown, /contract complete/);
  assert.match(markdown, /runtime wired/);
  assert.match(markdown, /product smoke passed/);
  assert.match(markdown, /SERVER_HANDLER_NOT_IMPLEMENTED/);
  assert.match(markdown, /Next implementation target/);
  assert.match(markdown, /auth\.login/);
});
