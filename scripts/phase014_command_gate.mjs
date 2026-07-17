import { spawn } from "node:child_process";
import { mkdir, rename, writeFile } from "node:fs/promises";
import { join } from "node:path";

import { fingerprintPhase014CurrentSource } from "./phase014_source_fingerprint.mjs";

const SHA256 = /^[0-9a-f]{64}$/;

export function validatePhase014CommandReceipt(receipt, expectedFingerprint, expectedCommandId) {
  const findingIds = [];
  if (receipt?.marker !== "phase014_command_gate=passed") findingIds.push("marker");
  if (receipt?.state !== "Passed") findingIds.push("state");
  if (receipt?.commandId !== expectedCommandId) findingIds.push("command_id");
  if (!SHA256.test(receipt?.sourceFingerprint ?? "")) findingIds.push("source_fingerprint");
  if (expectedFingerprint && receipt?.sourceFingerprint !== expectedFingerprint) findingIds.push("stale_source_fingerprint");
  if (receipt?.diagnostics !== "sanitized") findingIds.push("diagnostics");
  return Object.freeze({ passed: findingIds.length === 0, findingIds: Object.freeze(findingIds) });
}

export async function runPhase014CommandGate({ root, commandId, executable, args, receiptName, execute = executeCommand }) {
  const before = await fingerprintPhase014CurrentSource(root);
  const exitCode = await execute(executable, args, root);
  if (exitCode !== 0) throw new Error(`PHASE014_COMMAND_FAILED:${commandId}`);
  const after = await fingerprintPhase014CurrentSource(root);
  if (before !== after) throw new Error(`PHASE014_SOURCE_CHANGED_DURING_COMMAND:${commandId}`);
  const receipt = Object.freeze({
    marker: "phase014_command_gate=passed",
    state: "Passed",
    commandId,
    sourceFingerprint: after,
    diagnostics: "sanitized",
  });
  const validation = validatePhase014CommandReceipt(receipt, after, commandId);
  if (!validation.passed) throw new Error(`PHASE014_COMMAND_RECEIPT_INVALID:${validation.findingIds.join(",")}`);
  const release = join(root, ".tasks", "release");
  await mkdir(release, { recursive: true });
  await writeAtomic(join(release, receiptName), `${JSON.stringify(receipt, null, 2)}\n`);
  return receipt;
}

function executeCommand(executable, args, cwd) {
  return new Promise((resolve, reject) => {
    const child = spawn(executable, args, { cwd, stdio: "inherit" });
    child.on("error", reject);
    child.on("close", (code) => resolve(code ?? 1));
  });
}

async function writeAtomic(path, content) {
  const temporary = `${path}.tmp`;
  await writeFile(temporary, content, "utf8");
  await rename(temporary, path);
}
