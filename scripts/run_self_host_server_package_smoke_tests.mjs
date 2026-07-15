import assert from "node:assert/strict";
import test from "node:test";

import {
  assertPackageSmokeOutput,
  assertSensitiveOutputClean,
  buildSelfHostServerPackageSmokePlan,
} from "./run_self_host_server_package_smoke.mjs";

test("server package smoke plan builds cabinet-server and runs packaged smoke flag", () => {
  const plan = buildSelfHostServerPackageSmokePlan();

  assert.deepEqual(plan.buildCommand, ["cargo", "build", "-p", "cabinet-server"]);
  assert.deepEqual(plan.smokeCommand, [
    "target/debug/cabinet-server",
    "--self-host-package-smoke",
  ]);
});

test("server package smoke output validator requires pass marker and route count", () => {
  assertPackageSmokeOutput(
    [
      "server_package_smoke=passed",
      "route_count=37",
      "health_status_code=200",
      "default_profile_without_external_services=true",
    ].join("\n"),
  );

  assert.throws(
    () => assertPackageSmokeOutput("route_count=0\nhealth_status_code=500"),
    /server_package_smoke marker was not found/,
  );
});

test("server package smoke sensitive output scanner rejects secrets and document bodies", () => {
  assertSensitiveOutputClean("server_package_smoke=passed\nroute_count=37");

  assert.throws(
    () => assertSensitiveOutputClean("server_package_smoke=passed\ntoken=abc"),
    /sensitive output detected/,
  );
  assert.throws(
    () => assertSensitiveOutputClean("server_package_smoke=passed\nsecret-access-key"),
    /sensitive output detected/,
  );
  assert.throws(
    () =>
      assertSensitiveOutputClean(
        "server_package_smoke=passed\nfixture document body should not be logged",
      ),
    /sensitive output detected/,
  );
});
