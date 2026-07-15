import assert from "node:assert/strict";
import test from "node:test";

import { mapUserFacingError } from "../src/user_facing_error_presenter.ts";

test("known errors map to Korean copy and typed recovery actions", () => {
  assert.deepEqual(mapUserFacingError({
    stableCode: "WORKSPACE_HOME_PROJECTION_UNAVAILABLE",
    retryable: true,
    operationContext: "workspace_home",
    correlationReference: "request-42",
  }), {
    title: "작업 공간을 열 수 없습니다",
    message: "로컬 작업 공간 정보를 불러오지 못했습니다.",
    recoveryAction: "retry",
    recoveryLabel: "다시 시도",
    diagnosticReference: "request-42",
    mapping: "known",
  });

  assert.equal(mapUserFacingError({
    stableCode: "CANVAS_RECOVERY_REQUIRED",
    retryable: false,
    operationContext: "canvas",
  }).recoveryAction, "recover");
});

test("unknown errors never reflect raw values and sanitize diagnostic references", () => {
  const result = mapUserFacingError({
    stableCode: "SECRET_INTERNAL_ERROR",
    retryable: false,
    operationContext: "navigator",
    correlationReference: "/Users/person/private\nsecret-token",
  });

  assert.equal(result.mapping, "unknown");
  assert.equal(result.recoveryAction, "none");
  assert.doesNotMatch(`${result.title} ${result.message} ${result.diagnosticReference}`, /SECRET|private|token|\/Users/);
  assert.match(result.diagnosticReference, /^ref-[a-f0-9]{8}$/);
});
