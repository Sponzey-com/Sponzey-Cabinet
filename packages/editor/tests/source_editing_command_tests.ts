import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  applyMarkdownFormattingCommand,
  createInsertMarkdownTableOperation,
} from "../src/index.ts";

test("editor creates markdown table insertion source command without html grid", () => {
  const operation = createInsertMarkdownTableOperation({
    headers: ["항목", "내용", "상태"],
    alignments: ["left", "center", "right"],
    rowCount: 2,
  });

  assert.equal(operation.kind, "insert-markdown-table");
  assert.equal(
    operation.value,
    [
      "| 항목 | 내용 | 상태 |",
      "| :--- | :---: | ---: |",
      "|  |  |  |",
      "|  |  |  |",
    ].join("\n"),
  );
  assert.equal(operation.value.includes("<table"), false);
});

test("editor applies basic Markdown formatting commands with deterministic snippets", () => {
  assert.equal(applyMarkdownFormattingCommand("", "heading"), "# 제목");
  assert.equal(applyMarkdownFormattingCommand("본문", "bold"), "본문\n\n**굵은 텍스트**");
  assert.equal(applyMarkdownFormattingCommand("본문\n", "italic"), "본문\n_기울임 텍스트_");
  assert.equal(applyMarkdownFormattingCommand("본문", "link"), "본문\n\n[링크 텍스트](https://example.com)");
  assert.equal(applyMarkdownFormattingCommand("본문", "list"), "본문\n\n- 목록 항목");
  assert.equal(applyMarkdownFormattingCommand("본문", "checklist"), "본문\n\n- [ ] 할 일");
  assert.equal(
    applyMarkdownFormattingCommand("본문", "table"),
    [
      "본문",
      "",
      "| 항목 | 내용 | 상태 |",
      "| :--- | :---: | ---: |",
      "|  |  |  |",
      "|  |  |  |",
    ].join("\n"),
  );
});

test("editor source command contract does not import codemirror runtime types", async () => {
  const source = await readFile(new URL("../src/index.ts", import.meta.url), "utf8");

  assert.doesNotMatch(source, /@codemirror\/state|@codemirror\/view/);
  assert.doesNotMatch(source, /import\s+.*EditorView/);
});
