import assert from "node:assert/strict";
import test from "node:test";

import {
  PackagedAccessibilityMeasurementError,
  measurePackagedAccessibilityRoute,
  packagedUnnamedControlDiagnostic,
  type PackagedAccessibilityDocument,
  type PackagedAccessibilityElement,
} from "../src/packaged_accessibility_measurement_port.ts";

test("packaged DOM measurement returns only safe route counts and focus results", () => {
  const fixture = documentFixture("Home", [
    element({ textContent: "새 문서", tabIndex: 0 }),
    element({ ariaLabel: "검색", tabIndex: 0 }),
    element({ textContent: "비활성", tabIndex: -1, disabled: true }),
  ]);
  const result = measurePackagedAccessibilityRoute(fixture.document, "Home");

  assert.deepEqual(result, {
    route: "Home",
    visibleControlCount: 3,
    namedControlCount: 3,
    mainFocusReached: true,
    keyboardJourneyPassed: true,
    focusRestorationCount: 1,
    keyboardErrorCount: 0,
    focusErrorCount: 0,
    internalExposureCount: 0,
  });
  assert.deepEqual(Object.keys(result).sort(), [
    "focusErrorCount", "focusRestorationCount", "internalExposureCount",
    "keyboardErrorCount", "keyboardJourneyPassed", "mainFocusReached",
    "namedControlCount", "route", "visibleControlCount",
  ]);
});

test("packaged DOM measurement counts unnamed controls without retaining labels", () => {
  const fixture = documentFixture("Assets", [
    element({ textContent: "첨부", tabIndex: 0 }),
    element({ textContent: "", tabIndex: 0 }),
  ]);
  const result = measurePackagedAccessibilityRoute(fixture.document, "Assets");
  assert.equal(result.visibleControlCount, 2);
  assert.equal(result.namedControlCount, 1);
  assert.equal("labels" in result, false);
});

test("unnamed control diagnostic returns only a stable action identity", () => {
  const fixture = documentFixture("Document", [
    element({ textContent: "저장", tabIndex: 0, action: "save-document" }),
    element({ textContent: "", tabIndex: 0, action: "workspace-search-input" }),
  ]);
  assert.equal(
    packagedUnnamedControlDiagnostic(fixture.document),
    "workspace-search-input",
  );
});

test("packaged DOM measurement reports focus failures and main restoration failure", () => {
  const fixture = documentFixture("Canvas", [
    element({ textContent: "선택", tabIndex: 0 }),
    element({ textContent: "연결", tabIndex: 0, focusFails: true }),
  ], true);
  const result = measurePackagedAccessibilityRoute(fixture.document, "Canvas");
  assert.equal(result.keyboardJourneyPassed, false);
  assert.equal(result.keyboardErrorCount, 1);
  assert.equal(result.mainFocusReached, false);
  assert.equal(result.focusErrorCount, 1);
  assert.equal(result.focusRestorationCount, 0);
});

test("packaged DOM measurement rejects route mismatch and missing shell", () => {
  const fixture = documentFixture("Graph", [element({ textContent: "확대", tabIndex: 0 })]);
  assert.throws(() => measurePackagedAccessibilityRoute(fixture.document, "Home"),
    measurementError("PACKAGED_ACCESSIBILITY_ROUTE_MISMATCH"));
  assert.throws(() => measurePackagedAccessibilityRoute({
    activeElement: null,
    querySelector: () => null,
    getElementById: () => null,
  }, "Home"), measurementError("PACKAGED_ACCESSIBILITY_SHELL_MISSING"));
});

test("packaged DOM measurement converts DOM focus exceptions to stable error codes", () => {
  const fixture = documentFixture("Document", [
    element({ textContent: "저장", tabIndex: 0, focusThrows: true }),
  ]);

  assert.throws(() => measurePackagedAccessibilityRoute(fixture.document, "Document"),
    measurementError("PACKAGED_ACCESSIBILITY_CONTROL_FOCUS_FAILED"));
});

type ElementOptions = Readonly<{
  textContent?: string;
  ariaLabel?: string;
  tabIndex: number;
  disabled?: boolean;
  focusFails?: boolean;
  focusThrows?: boolean;
  action?: string;
}>; 

function element(options: ElementOptions): PackagedAccessibilityElement & { focusFails: boolean; focusThrows: boolean } {
  const attributes = new Map<string, string>();
  if (options.ariaLabel) attributes.set("aria-label", options.ariaLabel);
  if (options.disabled) attributes.set("disabled", "");
  if (options.action) attributes.set("data-action", options.action);
  return {
    textContent: options.textContent ?? "",
    tabIndex: options.tabIndex,
    disabled: options.disabled ?? false,
    focusFails: options.focusFails ?? false,
    focusThrows: options.focusThrows ?? false,
    outerHTML: `<button${options.ariaLabel ? ` aria-label="${options.ariaLabel}"` : ""}>${options.textContent ?? ""}</button>`,
    getBoundingClientRect: () => ({ width: 100, height: 30 }),
    getAttribute: (name) => attributes.get(name) ?? null,
    hasAttribute: (name) => attributes.has(name),
    focus() {},
    querySelector: () => null,
    querySelectorAll: () => [],
  };
}

function documentFixture(
  route: string,
  controls: ReturnType<typeof element>[],
  mainFocusFails = false,
): { document: PackagedAccessibilityDocument } {
  let activeElement: PackagedAccessibilityElement | null = null;
  for (const control of controls) {
    control.focus = () => {
      if (control.focusThrows) throw new Error("raw focus failure");
      if (!control.focusFails) activeElement = control;
    };
  }
  const main = element({ textContent: "본문", tabIndex: -1, focusFails: mainFocusFails });
  main.focus = () => { if (!main.focusFails) activeElement = main; };
  const shell: PackagedAccessibilityElement = {
    ...element({ textContent: "", tabIndex: -1 }),
    outerHTML: `<div data-shell-route="${route}"><main>안전한 화면</main></div>`,
    getAttribute: (name) => name === "data-shell-route" ? route : null,
    querySelector: (selector) => selector === "[data-workspace-route-main]" ? main : null,
    querySelectorAll: () => controls,
  };
  return {
    document: {
      get activeElement() { return activeElement; },
      querySelector: (selector) => selector === "[data-shell-route]" ? shell : null,
      getElementById: () => null,
    },
  };
}

function measurementError(code: string): (error: unknown) => boolean {
  return (error) => error instanceof PackagedAccessibilityMeasurementError && error.code === code;
}
