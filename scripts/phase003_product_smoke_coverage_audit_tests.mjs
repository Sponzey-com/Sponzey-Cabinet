import assert from "node:assert/strict";
import test from "node:test";

import {
  ProductSmokeCoverageAuditErrorCode,
  ProductSmokeCoverageAuditEvent,
  ProductSmokeCoverageAuditState,
  analyzeProductSmokeCoverageSources,
  renderProductSmokeCoverageAuditMarkdown,
  transitionProductSmokeCoverageAuditState,
} from "./phase003_product_smoke_coverage_audit.mjs";

const completeSources = {
  "scripts/run_browser_smoke.sh": "node scripts/build_web_app.mjs node scripts/run_browser_smoke.mjs",
  "scripts/run_browser_smoke.mjs":
    "run_web_app.mjs browser_smoke=passed CodeMirror editor mounted Markdown preview table rendered New document created Restore flow completed",
  "scripts/build_web_app.mjs": "apps/web/public/app.bundle.js esbuild",
  "scripts/run_self_host_e2e_smoke.sh": "node scripts/run_self_host_e2e_smoke.mjs",
  "scripts/run_self_host_e2e_smoke.mjs":
    "scripts/run_self_host_server.sh waitForServer /api/auth/login search_under_300ms_target product_log_sensitive_exclusion self_host_e2e_smoke=passed",
  "scripts/run_desktop_remote_product_smoke.sh": "node scripts/run_desktop_remote_product_smoke.mjs",
  "scripts/run_desktop_remote_product_smoke.mjs":
    "scripts/run_self_host_server.sh runDesktopSmoke desktop_remote_product_smoke=passed desktop product smoke must not save remote document locally assertSensitiveOutputClean",
  "apps/desktop/tests/desktop_remote_product_smoke.ts":
    "desktop_remote_product_smoke=passed remote document save local repository",
  "scripts/run_mobile_read_contract_tests.sh": "node scripts/run_mobile_read_contract_tests.mjs",
  "scripts/run_mobile_read_contract_tests.mjs":
    "mobile_read_boundary_scan=passed createMobileReadSelfHostApiClient ios android",
  "scripts/run_mobile_read_product_smoke.sh": "node scripts/run_mobile_read_product_smoke.mjs",
  "scripts/run_mobile_read_product_smoke.mjs":
    "mobile_read_product_smoke=passed scripts/run_self_host_server.sh",
  "apps/mobile/tests/mobile_read_product_smoke.ts":
    "mobile_read_product_smoke=passed mobile_read_product_platform_ios=passed mobile_read_product_platform_android=passed createMobileReadSelfHostApiClient",
  "apps/mobile/tests/mobile_read_skeleton_tests.ts":
    "platform: \"ios\" platform: \"android\" createMobileReadSelfHostApiClient MOBILE_UNSUPPORTED_EDIT",
  "packages/client-core/tests/mobile_read_contract_tests.ts":
    "createMobileReadApiContract supportsMobileReadApi supportsRemoteEdit",
};

test("product smoke coverage audit marks complete fixture as product smoke wired", () => {
  const audit = analyzeProductSmokeCoverageSources({ sources: completeSources });

  assert.equal(audit.summary.totalTargets, 4);
  assert.equal(audit.summary.productSmokeWired, 4);
  assert.equal(audit.summary.targetsNeedingWork, 0);
  assert.equal(audit.nextImplementationTarget, null);
});

test("product smoke coverage audit classifies mobile contract-only smoke as next target", () => {
  const {
    "scripts/run_mobile_read_product_smoke.sh": _runner,
    "scripts/run_mobile_read_product_smoke.mjs": _script,
    "apps/mobile/tests/mobile_read_product_smoke.ts": _test,
    ...sources
  } = completeSources;

  const audit = analyzeProductSmokeCoverageSources({ sources });
  const mobile = audit.targets.find((target) => target.id === "mobile_read_skeleton_smoke");

  assert.equal(mobile.status, "contract smoke only");
  assert.equal(audit.summary.contractSmokeOnly, 1);
  assert.equal(audit.nextImplementationTarget.id, "mobile_read_skeleton_smoke");
});

test("product smoke coverage audit reports missing target when runner source is absent", () => {
  const {
    "scripts/run_browser_smoke.sh": _runner,
    "scripts/run_browser_smoke.mjs": _source,
    ...sources
  } = completeSources;

  const audit = analyzeProductSmokeCoverageSources({ sources });
  const web = audit.targets.find((target) => target.id === "web_browser_product_smoke");

  assert.equal(web.status, "missing");
  assert.equal(web.missingFiles.includes("scripts/run_browser_smoke.mjs"), true);
});

test("product smoke coverage audit fails with stable code when source set is empty", () => {
  assert.throws(
    () => analyzeProductSmokeCoverageSources({ sources: {} }),
    (error) => error.code === ProductSmokeCoverageAuditErrorCode.SourceSetEmpty,
  );
});

test("product smoke coverage audit state machine rejects invalid transitions", () => {
  assert.equal(
    transitionProductSmokeCoverageAuditState(
      ProductSmokeCoverageAuditState.NotStarted,
      ProductSmokeCoverageAuditEvent.Start,
    ),
    ProductSmokeCoverageAuditState.ReadingSource,
  );
  assert.equal(
    transitionProductSmokeCoverageAuditState(
      ProductSmokeCoverageAuditState.ReadingSource,
      ProductSmokeCoverageAuditEvent.SourceLoaded,
    ),
    ProductSmokeCoverageAuditState.Auditing,
  );
  assert.equal(
    transitionProductSmokeCoverageAuditState(
      ProductSmokeCoverageAuditState.Auditing,
      ProductSmokeCoverageAuditEvent.AuditComplete,
    ),
    ProductSmokeCoverageAuditState.Reported,
  );
  assert.throws(
    () =>
      transitionProductSmokeCoverageAuditState(
        ProductSmokeCoverageAuditState.NotStarted,
        ProductSmokeCoverageAuditEvent.ReportWritten,
      ),
    (error) => error.code === ProductSmokeCoverageAuditErrorCode.InvalidTransition,
  );
});

test("product smoke coverage markdown records status and next target", () => {
  const sources = {
    ...completeSources,
    "scripts/run_mobile_read_contract_tests.mjs":
      "mobile_read_boundary_scan=passed createMobileReadSelfHostApiClient ios android",
  };
  const audit = analyzeProductSmokeCoverageSources({ sources });
  const markdown = renderProductSmokeCoverageAuditMarkdown(audit);

  assert.match(markdown, /# Phase 003 Product Smoke Coverage Audit/);
  assert.match(markdown, /Phase 003\.3/);
  assert.match(markdown, /contract smoke only/);
  assert.match(markdown, /mobile_read_skeleton_smoke/);
});
