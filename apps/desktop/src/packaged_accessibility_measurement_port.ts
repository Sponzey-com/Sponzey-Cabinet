import type {
  AccessibilityRoute,
  AccessibilityRouteMeasurement,
} from "./accessibility_evidence_collector.ts";
import { auditUserExposedMarkup } from "./ui_exposure_audit.ts";

const INTERACTIVE_SELECTOR = [
  "button", "input", "select", "textarea", "a[href]", "[role=button]", "[tabindex]",
].join(",");

export interface PackagedAccessibilityElement {
  readonly textContent: string | null;
  readonly tabIndex: number;
  readonly disabled?: boolean;
  readonly outerHTML: string;
  getBoundingClientRect(): Readonly<{ width: number; height: number }>;
  getAttribute(name: string): string | null;
  hasAttribute(name: string): boolean;
  focus(): void;
  querySelector(selector: string): PackagedAccessibilityElement | null;
  querySelectorAll(selector: string): Iterable<PackagedAccessibilityElement> | ArrayLike<PackagedAccessibilityElement>;
}

export interface PackagedAccessibilityDocument {
  readonly activeElement: PackagedAccessibilityElement | null;
  querySelector(selector: string): PackagedAccessibilityElement | null;
  getElementById(id: string): PackagedAccessibilityElement | null;
}

export class PackagedAccessibilityMeasurementError extends Error {
  readonly code: string;

  constructor(code: string) {
    super(code);
    this.name = "PackagedAccessibilityMeasurementError";
    this.code = code;
  }
}

export function measurePackagedAccessibilityRoute(
  document: PackagedAccessibilityDocument,
  expectedRoute: AccessibilityRoute,
): AccessibilityRouteMeasurement {
  const shell = document.querySelector("[data-shell-route]");
  if (!shell) fail("PACKAGED_ACCESSIBILITY_SHELL_MISSING");
  if (shell.getAttribute("data-shell-route") !== expectedRoute) {
    fail("PACKAGED_ACCESSIBILITY_ROUTE_MISMATCH");
  }
  const main = shell.querySelector("[data-workspace-route-main]");
  if (!main) fail("PACKAGED_ACCESSIBILITY_MAIN_MISSING");

  const controls = scanVisibleControls(shell);
  const namedControlCount = countNamedControls(document, controls);
  const focusable = controls.filter((control) => !control.disabled
    && !control.hasAttribute("disabled")
    && control.tabIndex >= 0);
  let keyboardErrorCount = 0;
  for (const control of focusable) {
    focusControl(control, "PACKAGED_ACCESSIBILITY_CONTROL_FOCUS_FAILED");
    if (document.activeElement !== control) keyboardErrorCount += 1;
  }

  focusControl(main, "PACKAGED_ACCESSIBILITY_MAIN_FOCUS_FAILED");
  const mainFocusReached = document.activeElement === main;
  const internalExposureCount = countInternalExposure(shell.outerHTML);

  return Object.freeze({
    route: expectedRoute,
    visibleControlCount: controls.length,
    namedControlCount,
    mainFocusReached,
    keyboardJourneyPassed: focusable.length > 0 && keyboardErrorCount === 0,
    focusRestorationCount: mainFocusReached ? 1 : 0,
    keyboardErrorCount,
    focusErrorCount: mainFocusReached ? 0 : 1,
    internalExposureCount,
  });
}

function scanVisibleControls(shell: PackagedAccessibilityElement): PackagedAccessibilityElement[] {
  try {
    return Array.from(shell.querySelectorAll(INTERACTIVE_SELECTOR)).filter(isVisible);
  } catch {
    fail("PACKAGED_ACCESSIBILITY_CONTROL_SCAN_FAILED");
  }
}

function countNamedControls(
  document: PackagedAccessibilityDocument,
  controls: readonly PackagedAccessibilityElement[],
): number {
  try {
    return controls.filter((control) => hasAccessibleName(document, control)).length;
  } catch {
    fail("PACKAGED_ACCESSIBILITY_NAME_SCAN_FAILED");
  }
}

function focusControl(element: PackagedAccessibilityElement, errorCode: string): void {
  try {
    element.focus();
  } catch {
    fail(errorCode);
  }
}

function countInternalExposure(markup: string): number {
  try {
    return auditUserExposedMarkup(markup).length;
  } catch {
    fail("PACKAGED_ACCESSIBILITY_MARKUP_AUDIT_FAILED");
  }
}

export function packagedUnnamedControlDiagnostic(
  document: PackagedAccessibilityDocument,
): string | undefined {
  const shell = document.querySelector("[data-shell-route]");
  if (!shell) return undefined;
  const unnamed = Array.from(shell.querySelectorAll(INTERACTIVE_SELECTOR))
    .filter(isVisible)
    .find((control) => !hasAccessibleName(document, control));
  if (!unnamed) return undefined;
  const action = unnamed.getAttribute("data-action")?.trim();
  if (action && /^[a-z0-9-]{1,60}$/.test(action)) return action;
  const className = unnamed.getAttribute("class")?.trim().split(/\s+/u)[0];
  if (className && /^[a-z0-9-]{1,60}$/.test(className)) return className;
  return "unnamed-control";
}

function isVisible(element: PackagedAccessibilityElement): boolean {
  if (element.hasAttribute("hidden") || element.getAttribute("aria-hidden") === "true") return false;
  const bounds = element.getBoundingClientRect();
  return Number.isFinite(bounds.width) && Number.isFinite(bounds.height)
    && bounds.width > 0 && bounds.height > 0;
}

function hasAccessibleName(
  document: PackagedAccessibilityDocument,
  element: PackagedAccessibilityElement,
): boolean {
  if (element.getAttribute("aria-label")?.trim()) return true;
  const labelledBy = element.getAttribute("aria-labelledby")?.trim();
  if (labelledBy && labelledBy.split(/\s+/u).some((id) => document.getElementById(id)?.textContent?.trim())) {
    return true;
  }
  return Boolean(element.textContent?.trim());
}

function fail(code: string): never {
  throw new PackagedAccessibilityMeasurementError(code);
}
