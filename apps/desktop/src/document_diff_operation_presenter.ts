import type { DesktopDocumentDiffOperationSnapshot } from "./desktop_document_diff_operation_controller.ts";
import type { DesktopDocumentDiffWorkbenchState } from "./react_document_authoring_workbench.ts";

export interface DesktopDocumentDiffPresentationTarget {
  readonly targetVersionId: string;
  readonly targetVersionLabel: string;
}

export function presentDesktopDocumentDiffOperation(
  snapshot: DesktopDocumentDiffOperationSnapshot,
  target: DesktopDocumentDiffPresentationTarget,
): DesktopDocumentDiffWorkbenchState {
  if (snapshot.state === "Ready" && snapshot.diff?.status === "Complete") {
    return Object.freeze({
      status: "Ready",
      ...target,
      addedCount: snapshot.diff.addedCount,
      removedCount: snapshot.diff.removedCount,
      attachmentDiff: snapshot.diff.attachmentDiff,
      titleDelta: snapshot.diff.titleDelta,
      hunks: snapshot.diff.hunks,
    });
  }
  if (snapshot.state === "Accepted" || snapshot.state === "Running"
    || snapshot.state === "Cancelled" || snapshot.state === "Expired") {
    return Object.freeze({ status: snapshot.state, ...target });
  }
  return Object.freeze({
    status: "Failed",
    ...target,
    errorCode: snapshot.errorCode ?? "DOCUMENT_DIFF_OPERATION_FAILED",
    canRetry: true,
  });
}
