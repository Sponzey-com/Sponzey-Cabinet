import assert from "node:assert/strict";
import test from "node:test";

import type { LocalDesktopCommandEnvelope } from "@sponzey-cabinet/client-core";

import { createTauriDocumentDiffTransport } from "../src/tauri_document_diff_transport.ts";
import { createTauriDesktopTransport } from "../src/tauri_desktop_transport.ts";

test("diff transport maps current target and preserves semantic hunks", async () => {
  let nativeRequest: unknown;
  const transport = createTauriDocumentDiffTransport(async (command, args) => {
    assert.equal(command, "execute_desktop_document_diff");
    nativeRequest = args?.request;
    return {
      ok: true,
      retryable: false,
      repairRequired: false,
      data: {
        kind: "complete",
        leftVersionToken: "current-secret",
        rightVersionToken: "history-secret",
        addedCount: 1,
        removedCount: 1,
        attachmentDiff: {
          kind: "known",
          added: [{ label: "새 설계서", availability: "available" }],
          removed: [{ label: "이전 설계서", availability: "missing" }],
          relabeled: [{ beforeLabel: "초안", afterLabel: "최종안", availability: "available" }],
          unchangedCount: 2,
        },
        titleDelta: { kind: "changed", before: "현재 제목", after: "이전 제목" },
        hunks: [{
          oldStartLine: 1,
          newStartLine: 1,
          addedCount: 1,
          removedCount: 1,
          lines: [
            { kind: "removed", text: "현재 본문", oldLineNumber: 2 },
            { kind: "added", text: "이전 본문", newLineNumber: 2 },
          ],
        }],
      },
    };
  });

  const result = await transport(currentDiffEnvelope());

  assert.deepEqual(nativeRequest, {
    kind: "current_to_version",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    versionToken: "history-secret",
  });
  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.deepEqual(result.data, {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    status: "Complete",
    leftVersionId: "current-secret",
    rightVersionId: "history-secret",
    addedCount: 1,
    removedCount: 1,
    attachmentDiff: {
      status: "Known",
      added: [{ label: "새 설계서", availability: "Available" }],
      removed: [{ label: "이전 설계서", availability: "Missing" }],
      relabeled: [{ beforeLabel: "초안", afterLabel: "최종안", availability: "Available" }],
      unchangedCount: 2,
    },
    titleDelta: { kind: "Changed", before: "현재 제목", after: "이전 제목" },
    hunks: [{
      oldStartLine: 1,
      newStartLine: 1,
      addedCount: 1,
      removedCount: 1,
      lines: [
        { kind: "Removed", text: "현재 본문", oldLineNumber: 2 },
        { kind: "Added", text: "이전 본문", newLineNumber: 2 },
      ],
    }],
  });
});

test("diff transport maps too-large and rejects malformed body-bearing data", async () => {
  const tooLarge = createTauriDocumentDiffTransport(async () => ({
    ok: true,
    retryable: false,
    repairRequired: false,
    data: {
      kind: "too_large",
      leftVersionToken: "v2",
      rightVersionToken: "v1",
      limitReason: "bytes",
      addedCount: 0,
      removedCount: 0,
      attachmentDiff: {
        kind: "legacy_unknown",
        added: [],
        removed: [],
        relabeled: [],
        unchangedCount: 0,
      },
      hunks: [],
    },
  }));
  const malformed = createTauriDocumentDiffTransport(async () => ({
    ok: true,
    data: { kind: "complete", body: "must not be accepted" },
  }));
  const invalidAvailability = createTauriDocumentDiffTransport(async () => ({
    ok: true,
    data: {
      kind: "complete",
      leftVersionToken: "v2",
      rightVersionToken: "v1",
      addedCount: 0,
      removedCount: 0,
      attachmentDiff: {
        kind: "known",
        added: [{ label: "자료", availability: "unknown" }],
        removed: [],
        relabeled: [],
        unchangedCount: 0,
      },
      titleDelta: { kind: "unchanged" },
      hunks: [],
    },
  }));

  const bounded = await tooLarge(currentDiffEnvelope());
  assert.equal(bounded.ok && (bounded.data as { status: string }).status, "TooLarge");
  assert.equal(bounded.ok && (bounded.data as { limitReason: string }).limitReason, "bytes");
  assert.deepEqual(bounded.ok && (bounded.data as { attachmentDiff: unknown }).attachmentDiff, {
    status: "LegacyUnknown",
  });
  assert.deepEqual(await malformed(currentDiffEnvelope()), {
    ok: false,
    errorCode: "COMMAND_BRIDGE_FAILED",
    retryable: false,
    repairRequired: false,
  });
  assert.deepEqual(await invalidAvailability(currentDiffEnvelope()), {
    ok: false,
    errorCode: "COMMAND_BRIDGE_FAILED",
    retryable: false,
    repairRequired: false,
  });
});

test("desktop composite routes compare command through the diff boundary", async () => {
  const commands: string[] = [];
  const transport = createTauriDesktopTransport(async (command) => {
    commands.push(command);
    return {
      ok: true,
      retryable: false,
      repairRequired: false,
      data: {
        kind: "too_large",
        leftVersionToken: "v2",
        rightVersionToken: "v1",
        limitReason: "lines",
        addedCount: 0,
        removedCount: 0,
        attachmentDiff: {
          kind: "known",
          added: [],
          removed: [],
          relabeled: [],
          unchangedCount: 0,
        },
        hunks: [],
      },
    };
  });

  const result = await transport(currentDiffEnvelope());

  assert.equal(result.ok, true);
  assert.deepEqual(commands, ["execute_desktop_document_diff"]);
});

function currentDiffEnvelope(): LocalDesktopCommandEnvelope {
  return {
    commandName: "compare_document_versions",
    payload: {
      queryName: "compare-current-document-to-version",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      targetVersionId: "history-secret",
    },
  };
}
