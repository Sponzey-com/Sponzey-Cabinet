import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { createInsertMarkdownTableOperation } from "../src/index.ts";

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

test("editor source command contract does not import codemirror runtime types", async () => {
  const source = await readFile(new URL("../src/index.ts", import.meta.url), "utf8");

  assert.doesNotMatch(source, /@codemirror\/state|@codemirror\/view/);
  assert.doesNotMatch(source, /import\s+.*EditorView/);
});
