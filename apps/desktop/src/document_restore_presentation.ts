import type { DocumentDiffView } from "@sponzey-cabinet/client-core";

export interface RestorePreviewPresentationInput {
  readonly targetVersionId: string;
  readonly expectedCurrentVersionId: string;
  readonly targetVersionLabel: string;
  readonly changedLineCount: number;
  readonly missingAssetLabels: readonly string[];
  readonly canRestore: boolean;
  readonly diff: DocumentDiffView;
}

export interface RestoreCommandFailure {
  readonly code: string;
  readonly retryable: boolean;
  readonly repairRequired: boolean;
}

export type DocumentRestorePresentationState =
  | { readonly status: "Idle" }
  | { readonly status: "Previewing"; readonly targetVersionId: string }
  | ({ readonly status: "PreviewReady" } & RestorePreviewPresentationInput)
  | ({ readonly status: "Confirming" } & RestorePreviewPresentationInput)
  | ({ readonly status: "Applying"; readonly operationId: string } & RestorePreviewPresentationInput)
  | ({ readonly status: "Applied" } & RestorePreviewPresentationInput)
  | {
      readonly status: "Conflict";
      readonly targetVersionId: string;
      readonly targetVersionLabel: string;
    }
  | ({ readonly status: "BlockedMissingAsset" } & RestorePreviewPresentationInput)
  | ({ readonly status: "BlockedLargeDiff" } & RestorePreviewPresentationInput)
  | ({ readonly status: "RecoveryRequired"; readonly operationId: string } & RestorePreviewPresentationInput)
  | {
      readonly status: "Failed";
      readonly targetVersionId: string;
      readonly targetVersionLabel: string;
      readonly errorCategory: "invalid" | "not-found" | "storage" | "unavailable";
      readonly retryable: boolean;
    };

export function beginRestorePreview(targetVersionId: string): DocumentRestorePresentationState {
  return { status: "Previewing", targetVersionId };
}

export function completeRestorePreview(
  state: Extract<DocumentRestorePresentationState, { status: "Previewing" }>,
  input: RestorePreviewPresentationInput,
): DocumentRestorePresentationState {
  if (state.targetVersionId !== input.targetVersionId) return state;
  if (input.missingAssetLabels.length > 0) {
    return { status: "BlockedMissingAsset", ...input };
  }
  if (input.diff.status === "TooLarge") {
    return { status: "BlockedLargeDiff", ...input };
  }
  if (!input.canRestore) {
    return {
      status: "Failed",
      targetVersionId: input.targetVersionId,
      targetVersionLabel: input.targetVersionLabel,
      errorCategory: "invalid",
      retryable: false,
    };
  }
  return { status: "PreviewReady", ...input };
}

export function beginRestoreApply(
  state: DocumentRestorePresentationState,
  operationId: string,
): DocumentRestorePresentationState {
  if (state.status !== "Confirming") return state;
  return { ...state, status: "Applying", operationId };
}

export function requestRestoreConfirmation(
  state: DocumentRestorePresentationState,
): DocumentRestorePresentationState {
  return state.status === "PreviewReady" ? { ...state, status: "Confirming" } : state;
}

export function cancelRestoreConfirmation(
  state: DocumentRestorePresentationState,
): DocumentRestorePresentationState {
  return state.status === "Confirming" ? { ...state, status: "PreviewReady" } : state;
}

export function completeRestoreApply(
  state: Extract<DocumentRestorePresentationState, { status: "Applying" }>,
): Extract<DocumentRestorePresentationState, { status: "Applied" }> {
  const { operationId: _operationId, ...preview } = state;
  return { ...preview, status: "Applied" };
}

export function failRestoreApply(
  state: Extract<DocumentRestorePresentationState, { status: "Applying" }>,
  failure: RestoreCommandFailure,
): DocumentRestorePresentationState {
  if (failure.repairRequired || failure.code === "DOCUMENT_RESTORE_RECOVERY_REQUIRED") {
    return { ...state, status: "RecoveryRequired" };
  }
  if (failure.code === "DOCUMENT_RESTORE_VERSION_CONFLICT") {
    return {
      status: "Conflict",
      targetVersionId: state.targetVersionId,
      targetVersionLabel: state.targetVersionLabel,
    };
  }
  if (failure.code === "DOCUMENT_RESTORE_MISSING_DEPENDENCY") {
    return { ...state, status: "BlockedMissingAsset", missingAssetLabels: [] };
  }
  return {
    status: "Failed",
    targetVersionId: state.targetVersionId,
    targetVersionLabel: state.targetVersionLabel,
    errorCategory: classifyFailure(failure.code),
    retryable: failure.retryable,
  };
}

export function retryRestoreRecovery(
  state: Extract<DocumentRestorePresentationState, { status: "RecoveryRequired" }>,
): Extract<DocumentRestorePresentationState, { status: "Applying" }> {
  return { ...state, status: "Applying" };
}

export function refreshRestoreConflict(
  state: Extract<DocumentRestorePresentationState, { status: "Conflict" }>,
): Extract<DocumentRestorePresentationState, { status: "Previewing" }> {
  return { status: "Previewing", targetVersionId: state.targetVersionId };
}

function classifyFailure(code: string): "invalid" | "not-found" | "storage" | "unavailable" {
  if (code.endsWith("INVALID_INPUT")) return "invalid";
  if (code.endsWith("NOT_FOUND")) return "not-found";
  if (code.endsWith("STORAGE_UNAVAILABLE")) return "storage";
  return "unavailable";
}
