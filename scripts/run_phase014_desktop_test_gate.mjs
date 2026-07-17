import { readdir } from "node:fs/promises";
import { join } from "node:path";

import { runPhase014CommandGate } from "./phase014_command_gate.mjs";

const root = process.cwd();
const directory = join(root, "apps", "desktop", "tests");
const tests = (await readdir(directory))
  .filter((name) => name.endsWith(".ts") && name !== "desktop_remote_product_smoke.ts")
  .sort()
  .map((name) => join("apps", "desktop", "tests", name));
const receipt = await runPhase014CommandGate({
  root,
  commandId: "desktop-current-scope-tests",
  executable: process.execPath,
  args: ["--test", ...tests],
  receiptName: "desktop-test-gate-phase014.json",
});
console.log(receipt.marker);
console.log(`command_id=${receipt.commandId}`);
console.log(`source_fingerprint=${receipt.sourceFingerprint}`);
