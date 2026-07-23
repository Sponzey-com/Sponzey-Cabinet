import type { DocumentNavigatorModel } from "@sponzey-cabinet/ui";

export type GlobalSearchOverlayState =
  | "Closed"
  | "Searching"
  | "ResultsReady"
  | "Empty"
  | "Degraded"
  | "Failed";

export interface GlobalSearchOverlayPresentation {
  readonly state: GlobalSearchOverlayState;
  readonly title: string;
  readonly description: string;
  readonly closeLabel: string;
  readonly query: string;
}

export function presentGlobalSearchOverlay(
  model: Pick<DocumentNavigatorModel, "displayState" | "filter">,
): GlobalSearchOverlayPresentation {
  return Object.freeze({
    state: mapGlobalSearchOverlayState(model.displayState),
    title: "전체 검색",
    description: "제목, 본문, 첨부 파일 이름을 한 번에 검색합니다.",
    closeLabel: "검색 닫기",
    query: model.filter ?? "",
  });
}

function mapGlobalSearchOverlayState(
  displayState: DocumentNavigatorModel["displayState"],
): GlobalSearchOverlayState {
  if (displayState === "Closed") return "Closed";
  if (displayState === "Loading" || displayState === "Filtering") return "Searching";
  if (displayState === "Ready") return "ResultsReady";
  if (displayState === "EmptyResult") return "Empty";
  if (displayState === "Degraded") return "Degraded";
  return "Failed";
}
