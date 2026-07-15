import assert from "node:assert/strict";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase003PlanErrorCode,
  Phase003PlanEvent,
  Phase003PlanState,
  renderPhase003PlanValidationResult,
  runPhase003PlanValidation,
  transitionPhase003PlanState,
  validatePhase003PlanText,
} from "./phase003_plan_validator.mjs";

test("phase003 plan validator rejects missing required planning term", async () => {
  const root = await createFixtureRoot({
    planText: completePlanText().replace("- `runtime wired`", "- runtime connection"),
  });

  const result = await runPhase003PlanValidation({ root, planPath: ".tasks/plan.md" });
  const rendered = renderPhase003PlanValidationResult(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase003PlanErrorCode.RequiredTermMissing);
  assert.equal(result.findingId, "runtime wired");
  assert.match(rendered, /error_code=PHASE003_PLAN_REQUIRED_TERM_MISSING/);
});

test("phase003 plan validator rejects active phase002 plan state", () => {
  const findings = validatePhase003PlanText(
    completePlanText().replace("현재 단계: Phase 003", "현재 단계: Phase 002"),
  );

  assert.equal(findings[0].errorCode, Phase003PlanErrorCode.ActivePhaseMismatch);
  assert.equal(findings[0].findingId, "current_phase");
});

test("phase003 plan validator rejects missing implementation phase section", () => {
  const findings = validatePhase003PlanText(
    completePlanText().replace("## 5. Implementation Phases", "## 5. Work"),
  );

  assert.equal(findings[0].errorCode, Phase003PlanErrorCode.RequiredSectionMissing);
  assert.equal(findings[0].findingId, "## 5. Implementation Phases");
});

test("phase003 plan validator passes complete plan fixture", async () => {
  const root = await createFixtureRoot({ planText: completePlanText() });

  const result = await runPhase003PlanValidation({ root, planPath: ".tasks/plan.md" });

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase003PlanState.Passed);
  assert.equal(result.requiredSectionCount > 10, true);
});

test("phase003 plan state machine exposes explicit terminal states", () => {
  const reading = transitionPhase003PlanState(Phase003PlanState.NotStarted, Phase003PlanEvent.Start);
  const validating = transitionPhase003PlanState(reading.state, Phase003PlanEvent.PlanLoaded);
  const passed = transitionPhase003PlanState(validating.state, Phase003PlanEvent.Complete);
  const failed = transitionPhase003PlanState(validating.state, Phase003PlanEvent.Fail, {
    errorCode: Phase003PlanErrorCode.RequiredSectionMissing,
    findingId: "## 5. Implementation Phases",
  });

  assert.equal(reading.state, Phase003PlanState.ReadingPlan);
  assert.equal(validating.state, Phase003PlanState.Validating);
  assert.equal(passed.state, Phase003PlanState.Passed);
  assert.equal(failed.state, Phase003PlanState.Failed);
  assert.equal(failed.findingId, "## 5. Implementation Phases");
});

async function createFixtureRoot({ planText }) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase003-plan-"));
  await mkdir(join(root, ".tasks"), { recursive: true });
  await writeFile(join(root, ".tasks", "plan.md"), planText);
  return root;
}

function completePlanText() {
  return [
    "# Phase 003 Development Plan",
    "",
    "현재 단계: Phase 003 - Self-host Runtime and Product Hardening",
    "",
    "## 1. Project Goal",
    "Self-host product hardening.",
    "## 2. Current State Assessment",
    "Phase 002 archive is reference only.",
    "## 3. Architecture Direction",
    "Layered Architecture and Clean Architecture.",
    "## 4. Development Principles",
    "Tidy First and TDD.",
    "## 5. Implementation Phases",
    "Phase 003.1 runtime wiring.",
    "## 6. TDD Strategy",
    "Failing tests first.",
    "## 7. Configuration and Runtime Environment Policy",
    "Runtime config is read once at bootstrap.",
    "## 8. Logging Strategy",
    "Product Log, Field Debug Log, Development Log.",
    "## 9. State Machine Strategy",
    "Explicit state machine transitions.",
    "## 10. Dependency and Boundary Rules",
    "External I/O stays at adapters.",
    "## 11. Performance Strategy",
    "p95 300ms.",
    "## 12. Release and Validation Gates",
    "Product smoke and release gate.",
    "## 13. Risk and Mitigation",
    "Runtime gaps.",
    "## 14. Review Checklist",
    "Layer boundaries.",
    "## 15. Definition of Done",
    "Validated.",
    "## 16. Prohibited Implementation Patterns",
    "No hidden env reads.",
    "## 17. Next Task Decision",
    "Create task002.",
    "",
    "- `contract complete`",
    "- `runtime wired`",
    "- `product smoke passed`",
    "- `production hardening complete`",
    "- .tasks/phase002/archive-manifest.json",
    "- Web, iOS, Android, Windows, macOS, Linux",
  ].join("\n");
}
