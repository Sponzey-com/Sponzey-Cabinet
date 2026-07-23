import assert from "node:assert/strict";
import test from "node:test";

import { auditUserExposedMarkup } from "../src/ui_exposure_audit.ts";

test("audit rejects internal identity, stable error, absolute path, and banned English copy", () => {
  const issues = auditUserExposedMarkup('<main><p>doc-secret-42</p><div role="alert">COMMAND_BRIDGE_FAILED</div><span>/Users/person/data.md</span><button aria-label="Retry save">다시</button></main>');
  assert.deepEqual(issues.map((issue) => issue.code), ["IDENTITY_EXPOSED", "ERROR_CODE_EXPOSED", "ABSOLUTE_PATH_EXPOSED", "MARKDOWN_FILENAME_EXPOSED", "ENGLISH_COPY_EXPOSED"]);
});

test("audit accepts Korean presentation while ignoring callback-only data identities", () => {
  assert.deepEqual(auditUserExposedMarkup('<button data-document-id="doc-secret-42" aria-label="설계 문서 열기">설계 문서</button>'), []);
});

test("audit returns deterministic de-duplicated issues", () => {
  const issues = auditUserExposedMarkup('<p>asset-secret-1 asset-secret-2</p>');
  assert.equal(issues.length, 1);
  assert.equal(issues[0]?.code, "IDENTITY_EXPOSED");
});

test("audit rejects markdown filenames and internal versioning terms in visible and accessible copy", () => {
  const issues = auditUserExposedMarkup(`
    <main>
      <p>notes/source.md</p>
      <button aria-label="Git commit 기록 보기">이력</button>
      <span title="snapshot path 확인">복원 정보</span>
    </main>
  `);
  assert.deepEqual(issues.map((issue) => issue.code), [
    "MARKDOWN_FILENAME_EXPOSED",
    "SNAPSHOT_TERM_EXPOSED",
    "GIT_TERM_EXPOSED",
  ]);
});
