import assert from "node:assert/strict";
import test from "node:test";
import { presentAssetMetadata, presentLinkedDocuments } from "../src/asset_display_presenter.ts";

test("asset metadata presenter maps raw values to stable Korean labels", () => {
  assert.deepEqual(presentAssetMetadata({ mediaType: "application/pdf", byteSize: 1536, status: "metadata_only", previewCapability: "unsupported", extractionStatus: "not_requested" }), {
    mediaTypeLabel: "PDF 문서", sizeLabel: "1.5 KB", statusLabel: "메타데이터만 있음", previewLabel: "미리보기 미지원", extractionLabel: "텍스트 추출 안 함",
  });
  const unknown = presentAssetMetadata({ mediaType: "application/x-private", byteSize: 1, status: "secret_status", previewCapability: "secret_preview", extractionStatus: "secret_extract" });
  assert.deepEqual(unknown, { mediaTypeLabel: "파일", sizeLabel: "1 B", statusLabel: "상태 확인 필요", previewLabel: "미리보기 확인 필요", extractionLabel: "추출 상태 확인 필요" });
  assert.equal(JSON.stringify(unknown).includes("secret"), false);
});

test("linked document presenter preserves callback identity without displaying it", () => {
  const result = presentLinkedDocuments(["doc-secret-known", "doc-secret-missing"], [{ category: "document", identity: "doc-secret-known", label: "설계 문서", breadcrumbLabel: "프로젝트", statusLabel: "", state: "resolved" }]);
  assert.deepEqual(result.map((item) => [item.identity, item.label]), [["doc-secret-known", "설계 문서"], ["doc-secret-missing", "찾을 수 없는 문서"]]);
  assert.equal(result.map((item) => item.label).join(" ").includes("doc-secret"), false);
});
