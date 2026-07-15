import { formatBytesKoKr } from "./ko_kr_catalog.ts";
import type { DisplayReference } from "./display_reference_resolver.ts";

export interface AssetMetadataPresentationInput {
  readonly mediaType: string;
  readonly byteSize: number;
  readonly status: string;
  readonly previewCapability?: string;
  readonly extractionStatus?: string;
}

export interface AssetMetadataPresentation {
  readonly mediaTypeLabel: string;
  readonly sizeLabel: string;
  readonly statusLabel: string;
  readonly previewLabel: string;
  readonly extractionLabel: string;
}

export function presentAssetMetadata(input: AssetMetadataPresentationInput): AssetMetadataPresentation {
  return Object.freeze({
    mediaTypeLabel: mediaTypeLabel(input.mediaType),
    sizeLabel: formatBytesKoKr(input.byteSize),
    statusLabel: ({ available: "사용 가능", missing: "파일 없음", metadata_only: "메타데이터만 있음" } as Record<string, string>)[input.status] ?? "상태 확인 필요",
    previewLabel: ({ image: "이미지 미리보기", pdf: "PDF 미리보기", text: "텍스트 미리보기", unsupported: "미리보기 미지원" } as Record<string, string>)[input.previewCapability ?? ""] ?? "미리보기 확인 필요",
    extractionLabel: ({ not_requested: "텍스트 추출 안 함", pending: "텍스트 추출 중", completed: "텍스트 추출 완료", failed: "텍스트 추출 실패" } as Record<string, string>)[input.extractionStatus ?? ""] ?? "추출 상태 확인 필요",
  });
}

export function presentLinkedDocuments(
  identities: readonly string[],
  references: readonly DisplayReference[],
): readonly DisplayReference[] {
  const byIdentity = new Map(references.map((reference) => [reference.identity, reference]));
  return Object.freeze(identities.map((identity) => byIdentity.get(identity) ?? Object.freeze({
    category: "document" as const,
    identity,
    label: "찾을 수 없는 문서",
    breadcrumbLabel: "",
    statusLabel: "대상을 찾을 수 없습니다",
    state: "missing" as const,
  })));
}

function mediaTypeLabel(mediaType: string): string {
  if (mediaType.startsWith("image/")) return "이미지";
  if (mediaType === "application/pdf") return "PDF 문서";
  if (/text|word|document/.test(mediaType)) return "텍스트 문서";
  return "파일";
}
