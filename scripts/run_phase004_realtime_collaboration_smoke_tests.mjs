import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const packageJson = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));

test("package scripts expose phase004 realtime collaboration smoke runner", () => {
  assert.equal(
    packageJson.scripts["run:phase004-realtime-collaboration-smoke-tests"],
    "node scripts/run_phase004_realtime_collaboration_smoke_tests.mjs",
  );
  assert.equal(
    packageJson.scripts["run:phase004-realtime-collaboration-smoke"],
    "sh scripts/run_phase004_realtime_collaboration_smoke.sh",
  );
});

test("phase004 realtime collaboration smoke runner executes required test commands", async () => {
  const runner = await readFile(
    new URL("./run_phase004_realtime_collaboration_smoke.sh", import.meta.url),
    "utf8",
  );
  const requiredCommands = [
    "node --test packages/client-core/tests/realtime_client_tests.ts",
    "cargo test -p cabinet-domain --test realtime_tests",
    "cargo test -p cabinet-ports --test realtime_contract_tests",
    "cargo test -p cabinet-adapters --test local_realtime_adapter_tests",
    "cargo test -p cabinet-server --test collaboration_realtime_command_mapper_tests",
    "cargo test -p cabinet-server --test collaboration_realtime_executor_tests",
    "cargo test -p cabinet-server --test collaboration_realtime_runtime_target_tests",
    "cargo test -p cabinet-server --test split_realtime_server_target_tests",
  ];

  for (const command of requiredCommands) {
    assert.match(runner, new RegExp(escapeRegExp(command)));
  }
  assert.match(runner, /phase004_realtime_collaboration_smoke=passed/);
  assert.match(runner, /\.tasks\/realtime-collaboration-smoke-result\.md/);
});

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
