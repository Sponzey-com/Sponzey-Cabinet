import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  applyPlainTextEditorChangeToSyncSession,
  applyWysiwygMarkdownChecklistItemToggle,
  applyWysiwygMarkdownBlockTextEdit,
  applyWysiwygMarkdownTableCellEdit,
  applyWysiwygPatchToSyncSession,
  createWysiwygPlainTextSyncSession,
  createWysiwygMarkdownPresentationModel,
} from "../src/index.ts";

const sample = [
  "# 첫번째 문서",
  "",
  "본문 첫 줄",
  "본문 둘째 줄",
  "",
  "- [ ] 할 일",
  "- [x] 완료",
  "",
  "| 항목 | 내용 | 상태 |",
  "| :--- | :---: | ---: |",
  "| 1번 | 가운데 | 완료 |",
].join("\n");

test("editor parses Markdown source into WYSIWYG blocks with stable source ranges", () => {
  const model = createWysiwygMarkdownPresentationModel({ source: sample });

  assert.equal(model.mode, "wysiwyg-markdown-presentation");
  assert.equal(model.state, "Parsed");
  assert.deepEqual(model.blocks.map((block) => block.blockType), [
    "heading",
    "paragraph",
    "checklist",
    "table",
  ]);

  const heading = model.blocks[0];
  assert.equal(heading.blockId, "block-1");
  assert.equal(heading.displayText, "첫번째 문서");
  assert.equal(sample.slice(heading.sourceRange.start, heading.sourceRange.end), "# 첫번째 문서");
  assert.equal(heading.editable, true);

  const paragraph = model.blocks[1];
  assert.equal(paragraph.displayText, "본문 첫 줄\n본문 둘째 줄");
  assert.equal(sample.slice(paragraph.sourceRange.start, paragraph.sourceRange.end), "본문 첫 줄\n본문 둘째 줄");

  const checklist = model.blocks[2];
  assert.equal(checklist.blockType, "checklist");
  assert.deepEqual(checklist.items, [
    { checked: false, text: "할 일" },
    { checked: true, text: "완료" },
  ]);

  const table = model.blocks[3];
  assert.equal(table.blockType, "table");
  assert.deepEqual(table.headers, ["항목", "내용", "상태"]);
  assert.deepEqual(table.alignments, ["left", "center", "right"]);
  assert.deepEqual(table.rows, [["1번", "가운데", "완료"]]);
  assert.equal(sample.slice(table.sourceRange.start, table.sourceRange.end), [
    "| 항목 | 내용 | 상태 |",
    "| :--- | :---: | ---: |",
    "| 1번 | 가운데 | 완료 |",
  ].join("\n"));
});

test("editor parses WYSIWYG inline references without losing source ranges", () => {
  const source = [
    "# 문서 [[Target|표시 제목]]",
    "",
    "참조: [[Target]] and [외부 링크](https://example.com/private) and ![[asset:asset-private|설계 파일]]",
  ].join("\n");
  const model = createWysiwygMarkdownPresentationModel({ source });
  const heading = model.blocks[0];
  const paragraph = model.blocks[1];

  assert.equal(heading.blockType, "heading");
  assert.deepEqual(heading.inlines.map((inline) => inline.inlineType), ["text", "wikilink"]);
  assert.deepEqual(heading.inlines[1], {
    inlineType: "wikilink",
    text: "표시 제목",
    target: "Target",
    label: "표시 제목",
    sourceRange: {
      start: source.indexOf("[[Target|표시 제목]]"),
      end: source.indexOf("[[Target|표시 제목]]") + "[[Target|표시 제목]]".length,
    },
  });

  assert.equal(paragraph.blockType, "paragraph");
  assert.deepEqual(paragraph.inlines.map((inline) => inline.inlineType), [
    "text",
    "wikilink",
    "text",
    "markdown_link",
    "text",
    "asset_reference",
  ]);
  const wikilink = paragraph.inlines[1];
  assert.equal(wikilink.text, "Target");
  assert.equal(wikilink.sourceRange.start, source.indexOf("[[Target]]"));
  const markdownLink = paragraph.inlines[3];
  assert.equal(markdownLink.text, "외부 링크");
  assert.equal(markdownLink.sourceRange.start, source.indexOf("[외부 링크]"));
  const assetReference = paragraph.inlines[5];
  assert.equal(assetReference.text, "설계 파일");
  assert.equal(assetReference.sourceRange.start, source.indexOf("![[asset:asset-private|설계 파일]]"));
});

test("editor parses WYSIWYG code and quote blocks without exposing Markdown markers", () => {
  const source = [
    "# 개발 노트",
    "",
    "```rust",
    "fn main() {",
    "  println!(\"cabinet\");",
    "}",
    "```",
    "",
    "> [!NOTE] 참고",
    "> 인용 내용",
  ].join("\n");
  const model = createWysiwygMarkdownPresentationModel({ source });

  assert.deepEqual(model.blocks.map((block) => block.blockType), ["heading", "code_block", "blockquote"]);
  const code = model.blocks[1];
  assert.equal(code.blockType, "code_block");
  assert.equal(code.language, "rust");
  assert.equal(code.displayText, "fn main() {\n  println!(\"cabinet\");\n}");
  assert.equal(source.slice(code.sourceRange.start, code.sourceRange.end), [
    "```rust",
    "fn main() {",
    "  println!(\"cabinet\");",
    "}",
    "```",
  ].join("\n"));
  assert.doesNotMatch(code.displayText, /```/);

  const quote = model.blocks[2];
  assert.equal(quote.blockType, "blockquote");
  assert.equal(quote.calloutKind, "NOTE");
  assert.equal(quote.displayText, "참고\n인용 내용");
  assert.doesNotMatch(quote.displayText, /^>|!\[NOTE]/);
});

test("editor parses an unclosed WYSIWYG code block through EOF without throwing", () => {
  const source = "# 개발 노트\n\n```ts\nconst value = 1;";
  const model = createWysiwygMarkdownPresentationModel({ source });
  const code = model.blocks[1];

  assert.equal(code.blockType, "code_block");
  assert.equal(code.language, "ts");
  assert.equal(code.displayText, "const value = 1;");
  assert.equal(source.slice(code.sourceRange.start, code.sourceRange.end), "```ts\nconst value = 1;");
});

test("editor applies WYSIWYG block text edits only when the source range is fresh", () => {
  const model = createWysiwygMarkdownPresentationModel({ source: sample });
  const paragraph = model.blocks[1];
  const applied = applyWysiwygMarkdownBlockTextEdit({
    body: sample,
    sourceRange: paragraph.sourceRange,
    expectedSourceText: "본문 첫 줄\n본문 둘째 줄",
    replacementSourceText: "수정한 본문",
  });

  assert.equal(applied.status, "Applied");
  assert.equal(applied.nextBody, sample.replace("본문 첫 줄\n본문 둘째 줄", "수정한 본문"));
  assert.deepEqual(applied.changedRange, {
    start: paragraph.sourceRange.start,
    end: paragraph.sourceRange.start + "수정한 본문".length,
  });

  const stale = applyWysiwygMarkdownBlockTextEdit({
    body: sample.replace("본문 둘째 줄", "이미 바뀐 본문"),
    sourceRange: paragraph.sourceRange,
    expectedSourceText: "본문 첫 줄\n본문 둘째 줄",
    replacementSourceText: "덮어쓰기 금지",
  });

  assert.equal(stale.status, "Rejected");
  assert.equal(stale.errorCode, "WYSIWYG_MARKDOWN_STALE_RANGE");
  assert.equal(stale.nextBody, undefined);
});

test("editor toggles WYSIWYG checklist markers without rewriting item text", () => {
  const model = createWysiwygMarkdownPresentationModel({ source: sample });
  const checklist = model.blocks[2];
  assert.equal(checklist.blockType, "checklist");
  const expectedSourceText = "- [ ] 할 일\n- [x] 완료";

  const first = applyWysiwygMarkdownChecklistItemToggle({
    body: sample,
    sourceRange: checklist.sourceRange,
    expectedSourceText,
    itemIndex: 0,
  });
  assert.equal(first.status, "Applied");
  assert.equal(first.nextBody, sample.replace(expectedSourceText, "- [x] 할 일\n- [x] 완료"));

  const second = applyWysiwygMarkdownChecklistItemToggle({
    body: sample,
    sourceRange: checklist.sourceRange,
    expectedSourceText,
    itemIndex: 1,
  });
  assert.equal(second.status, "Applied");
  assert.equal(second.nextBody, sample.replace(expectedSourceText, "- [ ] 할 일\n- [ ] 완료"));
});

test("editor rejects stale or invalid WYSIWYG checklist toggles without changing body", () => {
  const model = createWysiwygMarkdownPresentationModel({ source: sample });
  const checklist = model.blocks[2];
  assert.equal(checklist.blockType, "checklist");
  const stale = applyWysiwygMarkdownChecklistItemToggle({
    body: sample.replace("- [ ] 할 일", "- [x] 이미 변경됨"),
    sourceRange: checklist.sourceRange,
    expectedSourceText: "- [ ] 할 일\n- [x] 완료",
    itemIndex: 0,
  });

  assert.equal(stale.status, "Rejected");
  assert.equal(stale.errorCode, "WYSIWYG_MARKDOWN_STALE_RANGE");
  assert.equal(stale.nextBody, undefined);

  const invalid = applyWysiwygMarkdownChecklistItemToggle({
    body: sample,
    sourceRange: checklist.sourceRange,
    expectedSourceText: "- [ ] 할 일\n- [x] 완료",
    itemIndex: 5,
  });
  assert.equal(invalid.status, "Rejected");
  assert.equal(invalid.errorCode, "WYSIWYG_MARKDOWN_INVALID_RANGE");
});

test("editor edits WYSIWYG table body cells while preserving header and alignment rows", () => {
  const model = createWysiwygMarkdownPresentationModel({ source: sample });
  const table = model.blocks[3];
  assert.equal(table.blockType, "table");
  const expectedSourceText = [
    "| 항목 | 내용 | 상태 |",
    "| :--- | :---: | ---: |",
    "| 1번 | 가운데 | 완료 |",
  ].join("\n");

  const result = applyWysiwygMarkdownTableCellEdit({
    body: sample,
    sourceRange: table.sourceRange,
    expectedSourceText,
    rowIndex: 0,
    cellIndex: 1,
    replacementText: "수정 | 값",
  });

  assert.equal(result.status, "Applied");
  assert.equal(result.nextBody, sample.replace(expectedSourceText, [
    "| 항목 | 내용 | 상태 |",
    "| :--- | :---: | ---: |",
    "| 1번 | 수정 \\| 값 | 완료 |",
  ].join("\n")));
});

test("editor rejects stale or invalid WYSIWYG table cell edits without changing body", () => {
  const model = createWysiwygMarkdownPresentationModel({ source: sample });
  const table = model.blocks[3];
  assert.equal(table.blockType, "table");
  const expectedSourceText = [
    "| 항목 | 내용 | 상태 |",
    "| :--- | :---: | ---: |",
    "| 1번 | 가운데 | 완료 |",
  ].join("\n");

  const stale = applyWysiwygMarkdownTableCellEdit({
    body: sample.replace("| 1번 | 가운데 | 완료 |", "| 1번 | 이미 변경 | 완료 |"),
    sourceRange: table.sourceRange,
    expectedSourceText,
    rowIndex: 0,
    cellIndex: 1,
    replacementText: "덮어쓰기 금지",
  });
  assert.equal(stale.status, "Rejected");
  assert.equal(stale.errorCode, "WYSIWYG_MARKDOWN_STALE_RANGE");
  assert.equal(stale.nextBody, undefined);

  const invalid = applyWysiwygMarkdownTableCellEdit({
    body: sample,
    sourceRange: table.sourceRange,
    expectedSourceText,
    rowIndex: 3,
    cellIndex: 1,
    replacementText: "없음",
  });
  assert.equal(invalid.status, "Rejected");
  assert.equal(invalid.errorCode, "WYSIWYG_MARKDOWN_INVALID_RANGE");
});

test("editor synchronization session rejects stale WYSIWYG patches after plain text edits", () => {
  const session = createWysiwygPlainTextSyncSession({
    documentId: "doc-1",
    body: sample,
  });
  const model = createWysiwygMarkdownPresentationModel({ source: sample });
  const paragraph = model.blocks[1];
  const plainChanged = applyPlainTextEditorChangeToSyncSession(
    session,
    sample.replace("본문 둘째 줄", "원문에서 먼저 변경"),
  );

  const stale = applyWysiwygPatchToSyncSession(plainChanged.session, {
    baseRevision: 0,
    apply() {
      return applyWysiwygMarkdownBlockTextEdit({
        body: plainChanged.session.body,
        sourceRange: paragraph.sourceRange,
        expectedSourceText: "본문 첫 줄\n본문 둘째 줄",
        replacementSourceText: "WYSIWYG 오래된 변경",
      });
    },
  });

  assert.equal(plainChanged.session.revision, 1);
  assert.equal(plainChanged.session.editorState, "PlainTextEditing");
  assert.equal(stale.status, "Rejected");
  assert.equal(stale.errorCode, "EDITOR_PATCH_STALE");
  assert.equal(stale.session.body, plainChanged.session.body);
  assert.equal(stale.session.revision, 1);
  assert.equal(stale.session.editorState, "PatchRejected");
});

test("editor synchronization session applies fresh WYSIWYG patches and preserves invalid patch errors", () => {
  const session = createWysiwygPlainTextSyncSession({
    documentId: "doc-1",
    body: sample,
  });
  const model = createWysiwygMarkdownPresentationModel({ source: sample });
  const paragraph = model.blocks[1];
  const fresh = applyWysiwygPatchToSyncSession(session, {
    baseRevision: 0,
    apply() {
      return applyWysiwygMarkdownBlockTextEdit({
        body: session.body,
        sourceRange: paragraph.sourceRange,
        expectedSourceText: "본문 첫 줄\n본문 둘째 줄",
        replacementSourceText: "WYSIWYG 변경",
      });
    },
  });

  assert.equal(fresh.status, "Applied");
  assert.equal(fresh.session.body, sample.replace("본문 첫 줄\n본문 둘째 줄", "WYSIWYG 변경"));
  assert.equal(fresh.session.revision, 1);
  assert.equal(fresh.session.editorState, "WysiwygEditing");

  const invalid = applyWysiwygPatchToSyncSession(session, {
    baseRevision: 0,
    apply() {
      return applyWysiwygMarkdownBlockTextEdit({
        body: session.body,
        sourceRange: { start: sample.length + 1, end: sample.length + 2 },
        expectedSourceText: "missing",
        replacementSourceText: "invalid",
      });
    },
  });

  assert.equal(invalid.status, "Rejected");
  assert.equal(invalid.errorCode, "WYSIWYG_MARKDOWN_INVALID_RANGE");
  assert.equal(invalid.session.body, session.body);
  assert.equal(invalid.session.revision, 0);
  assert.equal(invalid.session.editorState, "PatchRejected");
});

test("editor treats raw HTML as a non-editable WYSIWYG fallback without exposing the source text", () => {
  const source = "# Safe\n\n<script>globalThis.compromised=true</script>";
  const model = createWysiwygMarkdownPresentationModel({ source });
  const fallback = model.blocks[1];

  assert.equal(fallback.blockType, "fallback");
  assert.equal(fallback.editable, false);
  assert.equal(fallback.fallbackReason, "raw_html");
  assert.equal(fallback.displayText, "원문 편집에서 확인할 수 있는 HTML 블록");
  assert.doesNotMatch(fallback.displayText, /globalThis\.compromised|<script>/);
});

test("editor WYSIWYG parser has no React DOM or CodeMirror runtime dependency", async () => {
  const source = await readFile(new URL("../src/index.ts", import.meta.url), "utf8");

  assert.doesNotMatch(source, /react|react-dom|@codemirror\/state|@codemirror\/view/);
  assert.doesNotMatch(source, /document\.createElement|HTMLElement|EditorView/);
});
