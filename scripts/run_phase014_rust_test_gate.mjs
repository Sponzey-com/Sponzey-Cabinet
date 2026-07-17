import { runPhase014CommandGate } from "./phase014_command_gate.mjs";

const root = process.cwd();
const receipt = await runPhase014CommandGate({
  root,
  commandId: "rust-workspace-tests",
  executable: "cargo",
  args: ["test", "--workspace", "--all-targets"],
  receiptName: "rust-test-gate-phase014.json",
});
console.log(receipt.marker);
console.log(`command_id=${receipt.commandId}`);
console.log(`source_fingerprint=${receipt.sourceFingerprint}`);
