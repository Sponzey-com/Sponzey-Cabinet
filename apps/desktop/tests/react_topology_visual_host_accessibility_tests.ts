import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";

import {
  ReactTopologyVisualHost,
  nextTopologySemanticFocus,
} from "../src/react_topology_visual_host.ts";

test("topology semantic focus policy wraps and handles Home End without DOM state", () => {
  const keys = ["one", "two", "three"];

  assert.equal(nextTopologySemanticFocus(keys, "one", "ArrowDown"), "two");
  assert.equal(nextTopologySemanticFocus(keys, "three", "ArrowDown"), "one");
  assert.equal(nextTopologySemanticFocus(keys, "one", "ArrowUp"), "three");
  assert.equal(nextTopologySemanticFocus(keys, "unknown", "ArrowDown"), "one");
  assert.equal(nextTopologySemanticFocus(keys, "two", "Home"), "one");
  assert.equal(nextTopologySemanticFocus(keys, "two", "End"), "three");
  assert.equal(nextTopologySemanticFocus([], undefined, "ArrowDown"), undefined);
});

test("topology semantic list exposes one roving tab stop and a safe selected summary", () => {
  const markup = renderToStaticMarkup(React.createElement(ReactTopologyVisualHost, {
    model: {
      nodes: [
        { key: "opaque-one", title: "Cabinet 개요", kind: "document", selected: false, center: true, canNavigate: true },
        { key: "opaque-two", title: "설계 결정", kind: "document", selected: true, center: false, canNavigate: true },
      ],
      edges: [{ key: "edge-secret", sourceKey: "opaque-one", targetKey: "opaque-two", kind: "document_link" }],
    },
    semanticNodes: [
      { identity: "opaque-one", label: "Cabinet 개요", kind: "document", kindLabel: "문서", canNavigate: true },
      { identity: "opaque-two", label: "설계 결정", kind: "document", kindLabel: "문서", canNavigate: true },
    ],
    onNodeSelected() {},
    onNodeActivated() {},
  }));

  assert.equal((markup.match(/data-action="select-graph-node"[^>]*tabindex="0"/g) ?? []).length, 1);
  assert.equal((markup.match(/data-action="select-graph-node"[^>]*tabindex="-1"/g) ?? []).length, 1);
  assert.equal((markup.match(/data-action="open-graph-document"[^>]*tabindex="-1"/g) ?? []).length, 2);
  assert.match(markup, /data-topology-semantic-edges="available"/);
  assert.match(markup, /data-edge-kind="document_link"/);
  assert.match(markup, /data-edge-source-id="opaque-one"/);
  assert.match(markup, /data-edge-target-id="opaque-two"/);
  assert.match(markup, />Cabinet 개요 → 설계 결정</);
  assert.match(markup, /class="topology-accessibility-summary"[^>]*>노드 2개, 연결 1개, 선택: 설계 결정</);
  assert.doesNotMatch(markup, />opaque-(?:one|two)</);
  assert.doesNotMatch(markup, />edge-secret</);
});

test("topology semantic controls preserve focus and selection in forced colors", async () => {
  const css = await readFile(new URL("../public/styles.css", import.meta.url), "utf8");
  assert.match(css, /@media\s*\(forced-colors:\s*active\)/);
  assert.match(css, /\.topology-semantic-list button:focus-visible[^{]*\{[^}]*outline:/s);
  assert.match(css, /\.topology-semantic-list button\[aria-current="true"\][^{]*\{[^}]*border:/s);
  assert.match(css, /\.topology-semantic-edge-list[^{]*\{[^}]*clip-path:\s*inset\(50%\)/s);
});
