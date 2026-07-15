import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("exploration source classifies every input and select with a stable action", async () => {
  const source = await readFile(new URL("../src/react_exploration_surfaces.ts", import.meta.url), "utf8");
  const controls = [...source.matchAll(/e\("(input|select)", \{([^}]*)\}/gs)];
  const unidentified = controls
    .filter((match) => !match[2]?.includes('"data-action"'))
    .map((match) => `${match[1]}:${match[2]?.slice(0, 50)}`);
  assert.deepEqual(unidentified, []);
});

test("exploration symbol-only buttons provide an explicit accessible name", async () => {
  const source = await readFile(new URL("../src/react_exploration_surfaces.ts", import.meta.url), "utf8");
  const symbolOnly = source.split("\n")
    .filter((line) => line.includes('e("button"') && /\}, "[+−×←→↑↓↖⌁□✎⌘✓]"\)/.test(line));
  assert.deepEqual(
    symbolOnly.filter((line) => !line.includes('"aria-label"')),
    [],
  );
});
