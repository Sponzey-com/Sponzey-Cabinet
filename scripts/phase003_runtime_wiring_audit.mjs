import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const RuntimeWiringAuditState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingSource: "ReadingSource",
  Auditing: "Auditing",
  Reported: "Reported",
  Failed: "Failed",
});

export const RuntimeWiringAuditEvent = Object.freeze({
  Start: "Start",
  SourceLoaded: "SourceLoaded",
  AuditComplete: "AuditComplete",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const RuntimeWiringAuditErrorCode = Object.freeze({
  InvalidTransition: "PHASE003_RUNTIME_WIRING_INVALID_TRANSITION",
  RouteRegistryEmpty: "PHASE003_ROUTE_REGISTRY_EMPTY",
  SourceReadFailed: "PHASE003_RUNTIME_WIRING_SOURCE_READ_FAILED",
  ReportWriteFailed: "PHASE003_RUNTIME_WIRING_REPORT_WRITE_FAILED",
  MissingRuntimeHandlers: "PHASE003_RUNTIME_WIRING_MISSING_HANDLERS",
});

const STATUS = Object.freeze({
  RuntimeWired: "runtime wired",
  DedicatedTarget: "runtime wired through dedicated target",
  ProductSmokeStubOnly: "product smoke stub only",
  ContractCompleteOnly: "contract complete only",
});

const NEXT_TARGET_PRIORITY = [
  "auth.login",
  "auth.validate_session",
  "user.list",
  "group.list",
  "group.add_member",
  "group.remove_member",
  "role.list_assignments",
  "role.assign",
  "role.revoke",
  "document.save_remote_current",
  "sharing.get_document",
  "sharing.update_document",
  "comment.list",
  "comment.add",
  "comment.add_inline",
  "comment.resolve",
  "comment.reopen",
  "review.request_document",
  "review.approve_document",
  "review.reject_document",
  "review.publish_document",
  "review.list_requests",
  "document_lock.lock",
  "document_lock.unlock",
  "document_lock.get",
  "audit.list_events",
  "backup.create",
  "backup.get_status",
  "backup.restore",
  "export.create_workspace",
  "export.get_status",
];

class RuntimeWiringAuditError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "RuntimeWiringAuditError";
    this.code = code;
  }
}

export function transitionRuntimeWiringAuditState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [
      `${RuntimeWiringAuditState.NotStarted}:${RuntimeWiringAuditEvent.Start}`,
      RuntimeWiringAuditState.ReadingSource,
    ],
    [
      `${RuntimeWiringAuditState.ReadingSource}:${RuntimeWiringAuditEvent.SourceLoaded}`,
      RuntimeWiringAuditState.Auditing,
    ],
    [
      `${RuntimeWiringAuditState.Auditing}:${RuntimeWiringAuditEvent.AuditComplete}`,
      RuntimeWiringAuditState.Reported,
    ],
    [
      `${RuntimeWiringAuditState.ReadingSource}:${RuntimeWiringAuditEvent.Fail}`,
      RuntimeWiringAuditState.Failed,
    ],
    [
      `${RuntimeWiringAuditState.Auditing}:${RuntimeWiringAuditEvent.Fail}`,
      RuntimeWiringAuditState.Failed,
    ],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new RuntimeWiringAuditError(
      RuntimeWiringAuditErrorCode.InvalidTransition,
      `invalid runtime wiring audit transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeRuntimeWiringSources({
  compositionSource,
  runtimeSource,
  healthSource,
  e2eHttpSource,
}) {
  const routes = parseRoutes(compositionSource);
  if (routes.length === 0) {
    throw new RuntimeWiringAuditError(
      RuntimeWiringAuditErrorCode.RouteRegistryEmpty,
      "server composition route registry contains no routes",
    );
  }

  const runtimeHandlers = parseRuntimeHandlers(runtimeSource);
  const healthDedicatedRouteIds = parseDedicatedHealthRouteIds(healthSource);

  const auditedRoutes = routes.map((route) => {
    const handlerKind = runtimeHandlers.get(route.routeId) ?? "NotImplemented";
    const status = classifyRouteStatus({
      route,
      handlerKind,
      healthDedicatedRouteIds,
      e2eHttpSource,
    });
    return {
      ...route,
      handlerKind,
      status,
      needsRuntimeWiring:
        status === STATUS.ProductSmokeStubOnly || status === STATUS.ContractCompleteOnly,
    };
  });

  const productSmokeStubOnlyRoutes = auditedRoutes.filter(
    (route) => route.status === STATUS.ProductSmokeStubOnly,
  );
  const missingRuntimeRoutes = auditedRoutes.filter(
    (route) => route.status === STATUS.ContractCompleteOnly,
  );
  const runtimeWiredRoutes = auditedRoutes.filter(
    (route) => route.status === STATUS.RuntimeWired,
  );
  const dedicatedTargetRoutes = auditedRoutes.filter(
    (route) => route.status === STATUS.DedicatedTarget,
  );
  const routesNeedingRuntimeWiring = auditedRoutes.filter(
    (route) => route.needsRuntimeWiring,
  );

  return {
    phase: "Phase 003",
    sourceFiles: [
      "crates/cabinet-server/src/composition.rs",
      "crates/cabinet-server/src/runtime.rs",
      "crates/cabinet-server/src/health.rs",
      "crates/cabinet-server/src/e2e_http.rs",
    ],
    summary: {
      totalRoutes: auditedRoutes.length,
      runtimeWiredRoutes: runtimeWiredRoutes.length,
      dedicatedTargetRoutes: dedicatedTargetRoutes.length,
      productSmokeStubOnlyRoutes: productSmokeStubOnlyRoutes.length,
      missingRuntimeRoutes: missingRuntimeRoutes.length,
      routesNeedingRuntimeWiring: routesNeedingRuntimeWiring.length,
    },
    findings:
      routesNeedingRuntimeWiring.length > 0
        ? [
            {
              id: "PHASE003_RUNTIME_WIRING_GAP",
              errorCode: RuntimeWiringAuditErrorCode.MissingRuntimeHandlers,
              message:
                "Some contract complete or product smoke stub only routes are not runtime wired to usecases.",
              routeIds: routesNeedingRuntimeWiring.map((route) => route.routeId),
            },
          ]
        : [],
    nextImplementationTarget: pickNextImplementationTarget(routesNeedingRuntimeWiring),
    routes: auditedRoutes,
  };
}

export function renderRuntimeWiringAuditMarkdown(audit) {
  const lines = [
    "# Phase 003 Runtime Wiring Audit",
    "",
    "현재 단계: Phase 003 - Self-host Runtime and Product Hardening",
    "",
    "## Purpose",
    "",
    "- 이 문서는 self-host server route registry와 runtime handler 연결 상태를 코드 기준으로 고정한다.",
    "- `contract complete`, `runtime wired`, `product smoke passed`, `production hardening complete`를 구분한다.",
    "- e2e HTTP stub 응답은 product smoke 보조 수단이며 runtime wired로 간주하지 않는다.",
    "- `SERVER_HANDLER_NOT_IMPLEMENTED` 경로에 남아 있는 route는 후속 구현 task에서 닫아야 한다.",
    "",
    "## Source Files",
    "",
    ...audit.sourceFiles.map((sourceFile) => `- ${sourceFile}`),
    "",
    "## Summary",
    "",
    "| Metric | Count |",
    "| --- | ---: |",
    `| total routes | ${audit.summary.totalRoutes} |`,
    `| runtime wired | ${audit.summary.runtimeWiredRoutes} |`,
    `| runtime wired through dedicated target | ${audit.summary.dedicatedTargetRoutes} |`,
    `| product smoke stub only | ${audit.summary.productSmokeStubOnlyRoutes} |`,
    `| contract complete only | ${audit.summary.missingRuntimeRoutes} |`,
    `| routes needing runtime wiring | ${audit.summary.routesNeedingRuntimeWiring} |`,
    "",
    "## Route Status",
    "",
    "| Method | Path | Route ID | Handler Kind | Status |",
    "| --- | --- | --- | --- | --- |",
    ...audit.routes.map(
      (route) =>
        `| ${route.method} | \`${route.path}\` | \`${route.routeId}\` | ${route.handlerKind} | ${route.status} |`,
    ),
    "",
    "## Findings",
    "",
  ];

  if (audit.findings.length === 0) {
    lines.push("- No runtime wiring gap was detected.");
  } else {
    for (const finding of audit.findings) {
      lines.push(
        `- ${finding.id}: ${finding.errorCode} - ${finding.message}`,
        `- affected route count: ${finding.routeIds.length}`,
      );
    }
  }

  lines.push("", "## Next implementation target", "");
  if (audit.nextImplementationTarget) {
    lines.push(
      `- route id: \`${audit.nextImplementationTarget.routeId}\``,
      `- method/path: ${audit.nextImplementationTarget.method} \`${audit.nextImplementationTarget.path}\``,
      `- current status: ${audit.nextImplementationTarget.status}`,
      `- selected reason: ${nextImplementationReason(audit.nextImplementationTarget.routeId)}`,
      "- next task should implement this selected runtime handler set, not all remaining handlers.",
    );
  } else {
    lines.push("- No next runtime wiring target remains.");
  }

  lines.push(
    "",
    "## Review Notes",
    "",
    "- Handler code must map HTTP/request DTOs to usecase input DTOs only.",
    "- Domain and usecase code must not receive framework request objects.",
    "- Runtime config must be read once at bootstrap and injected through composition root.",
    "- Product Log must not contain document body, comment body, token, secret, credential, or raw request/response body.",
    "- Field Debug routes require explicit scope, TTL, approval, expiration, and revoke behavior before production hardening complete.",
    "- Development Log style validator output must not be part of production default behavior.",
    "- User-facing reads and searches must preserve the p95 300ms target under indexed/projection state.",
    "",
  );

  return `${lines.join("\n")}\n`;
}

function nextImplementationReason(routeId) {
  if (routeId === "auth.login" || routeId === "auth.validate_session") {
    return "auth/session runtime wiring is the first Phase 003.1 dependency for user, group, role, document, comment, audit, backup, and Field Debug flows.";
  }
  if (routeId.startsWith("user.")) {
    return "user runtime wiring is the next Phase 003.1 dependency after auth/session and before group/RBAC administration flows.";
  }
  if (routeId.startsWith("group.") || routeId.startsWith("role.")) {
    return "group and RBAC runtime wiring closes the collaboration administration boundary before document collaboration handlers.";
  }
  return "this route is the next item in the Phase 003.1 runtime wiring priority order.";
}

export async function runRuntimeWiringAudit({
  rootDir,
  reportPath,
  writeReport = false,
}) {
  let state = RuntimeWiringAuditState.NotStarted;
  state = transitionRuntimeWiringAuditState(state, RuntimeWiringAuditEvent.Start);

  let sources;
  try {
    sources = await readRuntimeWiringSources(rootDir);
  } catch (error) {
    transitionRuntimeWiringAuditState(state, RuntimeWiringAuditEvent.Fail);
    throw new RuntimeWiringAuditError(
      RuntimeWiringAuditErrorCode.SourceReadFailed,
      error.message,
    );
  }
  state = transitionRuntimeWiringAuditState(
    state,
    RuntimeWiringAuditEvent.SourceLoaded,
  );

  let audit;
  try {
    audit = analyzeRuntimeWiringSources(sources);
  } catch (error) {
    transitionRuntimeWiringAuditState(state, RuntimeWiringAuditEvent.Fail);
    throw error;
  }
  state = transitionRuntimeWiringAuditState(
    state,
    RuntimeWiringAuditEvent.AuditComplete,
  );

  const markdown = renderRuntimeWiringAuditMarkdown(audit);
  if (writeReport) {
    try {
      await writeFile(reportPath, markdown);
    } catch (error) {
      throw new RuntimeWiringAuditError(
        RuntimeWiringAuditErrorCode.ReportWriteFailed,
        error.message,
      );
    }
  }

  return {
    state,
    audit,
    markdown,
  };
}

async function readRuntimeWiringSources(rootDir) {
  const sourcePaths = {
    compositionSource: "crates/cabinet-server/src/composition.rs",
    runtimeSource: "crates/cabinet-server/src/runtime.rs",
    healthSource: "crates/cabinet-server/src/health.rs",
    e2eHttpSource: "crates/cabinet-server/src/e2e_http.rs",
  };
  const entries = await Promise.all(
    Object.entries(sourcePaths).map(async ([key, relativePath]) => [
      key,
      await readFile(path.join(rootDir, relativePath), "utf8"),
    ]),
  );
  return Object.fromEntries(entries);
}

function parseRoutes(source) {
  const routeRegex =
    /\.with_route\s*\(\s*HttpMethod::(\w+)\s*,\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,?\s*\)/gms;
  return [...source.matchAll(routeRegex)].map((match) => ({
    method: match[1].toUpperCase(),
    path: match[2],
    routeId: match[3],
  }));
}

function parseRuntimeHandlers(source) {
  const handlerRegex = /"([a-z0-9_.]+)"\s*=>\s*Self::(\w+)/g;
  return new Map([...source.matchAll(handlerRegex)].map((match) => [match[1], match[2]]));
}

function parseDedicatedHealthRouteIds(source) {
  if (source.includes('input.route_id() != "health.check"')) {
    return new Set(["health.check"]);
  }
  return new Set();
}

function classifyRouteStatus({ route, handlerKind, healthDedicatedRouteIds, e2eHttpSource }) {
  if (healthDedicatedRouteIds.has(route.routeId)) {
    return STATUS.DedicatedTarget;
  }
  if (handlerKind !== "NotImplemented" && handlerKind !== "Health") {
    return STATUS.RuntimeWired;
  }
  if (routeHasE2eStub(route, e2eHttpSource)) {
    return STATUS.ProductSmokeStubOnly;
  }
  return STATUS.ContractCompleteOnly;
}

function routeHasE2eStub(route, source) {
  if (source.includes(`request.path() == "${route.path}"`)) {
    return true;
  }
  const staticSegments = route.path
    .trim()
    .split("/")
    .filter(Boolean)
    .filter((segment) => !segment.startsWith("{") && !segment.endsWith("}"));
  if (staticSegments.length === 0) {
    return false;
  }
  return staticSegments.every((segment) => source.includes(`"${segment}"`));
}

function pickNextImplementationTarget(routesNeedingRuntimeWiring) {
  if (routesNeedingRuntimeWiring.length === 0) {
    return null;
  }
  const byRouteId = new Map(routesNeedingRuntimeWiring.map((route) => [route.routeId, route]));
  for (const routeId of NEXT_TARGET_PRIORITY) {
    if (byRouteId.has(routeId)) {
      return byRouteId.get(routeId);
    }
  }
  return routesNeedingRuntimeWiring[0];
}

function formatCliResult(result, reportPath) {
  const { audit } = result;
  return [
    "phase003_runtime_wiring_audit=passed",
    `total_routes=${audit.summary.totalRoutes}`,
    `runtime_wired_routes=${audit.summary.runtimeWiredRoutes}`,
    `dedicated_target_routes=${audit.summary.dedicatedTargetRoutes}`,
    `product_smoke_stub_only_routes=${audit.summary.productSmokeStubOnlyRoutes}`,
    `contract_complete_only_routes=${audit.summary.missingRuntimeRoutes}`,
    `routes_needing_runtime_wiring=${audit.summary.routesNeedingRuntimeWiring}`,
    `next_route_id=${audit.nextImplementationTarget?.routeId ?? "none"}`,
    `report_path=${reportPath}`,
  ].join("\n");
}

async function main(argv) {
  const currentFile = fileURLToPath(import.meta.url);
  const invokedFile = path.resolve(argv[1] ?? "");
  if (currentFile !== invokedFile) {
    return;
  }

  const rootDir = path.resolve(argv[2] ?? ".");
  const writeReport = argv.includes("--write");
  const reportPath = path.join(rootDir, ".tasks/phase003/runtime-wiring-audit.md");
  try {
    const result = await runRuntimeWiringAudit({
      rootDir,
      reportPath,
      writeReport,
    });
    process.stdout.write(`${formatCliResult(result, reportPath)}\n`);
  } catch (error) {
    process.stderr.write(`phase003_runtime_wiring_audit=failed\n`);
    process.stderr.write(`error_code=${error.code ?? "PHASE003_RUNTIME_WIRING_UNKNOWN"}\n`);
    process.stderr.write(`message=${error.message}\n`);
    process.exitCode = 1;
  }
}

await main(process.argv);
