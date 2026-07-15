import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase012EvidenceErrorCode,
  collectPhase012SourceFingerprint,
  phase012ReleaseCommandPlan,
  runPhase012ReleaseEvidence,
} from "./phase012_release_evidence.mjs";

test("source fingerprint is deterministic and excludes task and build output", async () => {
  const root = await sourceFixture();
  const first = await collectPhase012SourceFingerprint(root);
  await writeFile(join(root, ".tasks", "result.md"), "changed task output\n");
  await writeFile(join(root, "target", "debug.bin"), "changed build output\n");
  const second = await collectPhase012SourceFingerprint(root);
  assert.equal(first.sourceFingerprint, second.sourceFingerprint);
  assert.equal(first.sourceFileCount, second.sourceFileCount);

  await writeFile(join(root, "crates", "sample.rs"), "pub fn changed() {}\n");
  const third = await collectPhase012SourceFingerprint(root);
  assert.notEqual(first.sourceFingerprint, third.sourceFingerprint);
});

test("performance and macOS release claims require the packaged WebView workflow", () => {
  const ids = phase012ReleaseCommandPlan.map((command) => command.id);
  assert.ok(ids.includes("packaged-ui-contract"));
  assert.ok(ids.includes("macos-packaged-ui-smoke"));
});

test("fails fast and writes no passed artifacts when a command fails", async () => {
  const root = await sourceFixture();
  const calls = [];
  const result = await runPhase012ReleaseEvidence({
    root,
    commandPlan: phase012ReleaseCommandPlan,
    executeCommand: async (command) => {
      calls.push(command.id);
      return { passed: command.id !== phase012ReleaseCommandPlan[1].id, durationMs: 5 };
    },
  });
  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase012EvidenceErrorCode.CommandFailed);
  assert.equal(calls.length, 2);
  await assert.rejects(readFile(join(root, ".tasks", "phase012-release-gate-result.md"), "utf8"));
});

test("rejects an incomplete command plan before claiming requirement coverage", async () => {
  const root = await sourceFixture();
  const result = await runPhase012ReleaseEvidence({
    root,
    commandPlan: [phase012ReleaseCommandPlan[0]],
    executeCommand: async () => ({ passed: true, durationMs: 1 }),
  });
  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase012EvidenceErrorCode.RequirementCoverageMissing);
});

test("writes sanitized passed artifacts for a complete successful command plan", async () => {
  const root = await sourceFixture();
  const result = await runPhase012ReleaseEvidence({
    root,
    executeCommand: async () => ({ passed: true, durationMs: 7 }),
  });
  assert.equal(result.passed, true);
  assert.equal(result.requirementCount, 33);
  assert.equal(result.commandCount, phase012ReleaseCommandPlan.length);

  const release = await readFile(join(root, ".tasks", "phase012-release-gate-result.md"), "utf8");
  const matrix = await readFile(join(root, ".tasks", "release", "requirement-evidence-matrix-phase012.md"), "utf8");
  const commands = await readFile(join(root, ".tasks", "release", "command-summary-phase012.md"), "utf8");
  assert.match(release, /phase012_release_gate=passed/);
  assert.match(matrix, /requirement_count=33/);
  assert.match(commands, /phase012_command_summary=passed/);
  for (const text of [release, matrix, commands]) {
    assert.equal(text.includes(root), false);
    assert.doesNotMatch(text, /stdout\s*=|stderr\s*=|document_body\s*=|asset_bytes\s*=/);
  }
});

async function sourceFixture() {
  const root = await mkdtemp(join(tmpdir(), "cabinet-phase012-release-"));
  for (const directory of [".tasks", "target", "crates", "apps/desktop/src", "scripts"] ) {
    await mkdir(join(root, directory), { recursive: true });
  }
  await writeFile(join(root, "AGENTS.md"), "TDD\n");
  await writeFile(join(root, "PROJECT.md"), "local macOS desktop\n");
  await writeFile(join(root, "Cargo.toml"), "[workspace]\n");
  await writeFile(join(root, "package.json"), "{}\n");
  await writeFile(join(root, "crates", "sample.rs"), "pub fn sample() {}\n");
  await writeFile(join(root, "apps/desktop/src", "index.ts"), "export const ready = true;\n");
  await writeFile(join(root, "scripts", "sample.mjs"), "export const gate = true;\n");
  await writeFile(join(root, ".tasks", "result.md"), "ignored\n");
  await writeFile(join(root, "target", "debug.bin"), "ignored\n");
  return root;
}
