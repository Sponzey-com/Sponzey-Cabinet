import { createHash } from "node:crypto";
import { join } from "node:path";

const VALID_CHANNELS = new Set(["visible_text", "accessible_name", "tooltip"]);
const DIRECT_ID_RENDER_PATTERNS = [
  /null\s*,\s*(node\.id|selected\.id|node\.targetId|snapshot\.documentId|documentId|item\.path|source\.path)\b/,
  /aria-label[^\n]*(node\.id|selected\.id|node\.targetId|snapshot\.documentId|documentId)\b/,
  /}\s*,\s*(documentId|node\.targetId|node\.id|selected\.id)\s*[,)]/,
];
const DIRECT_ERROR_RENDER_PATTERN = /null\s*,\s*(snapshot\.errorCode|model\.error\?\.code|model\.error\.code)\b/;
const ID_DOM_ATTRIBUTE_PATTERN = /["']data-(?:document|asset|canvas|graph|linked-document|selected-asset)-(?:id|node-id)["']\s*:\s*([A-Za-z0-9_?.-]+)/;

export const DEFAULT_UI_AUDIT_POLICY = Object.freeze({
  approvedEnglishTerms: Object.freeze(["Markdown", "PDF", "Cabinet", "AI", "Cmd"]),
  approvedFileNamePattern: /^[^/\\]+\.[A-Za-z0-9]{1,8}$/,
  internalIdentityPatterns: Object.freeze([
    /\b[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\b/i,
    /^(?:workspace|document|doc|canvas|asset|version|operation)[-_:][A-Za-z0-9._:-]{3,}$/i,
  ]),
  internalErrorCodePattern: /^[A-Z][A-Z0-9]+(?:_[A-Z0-9]+)+$/,
  absolutePathPatterns: Object.freeze([
    /^\/(?:Users|home|var|tmp|private|opt)\//,
    /^[A-Za-z]:\\(?:Users|Documents and Settings|ProgramData|Windows)\\/i,
  ]),
});

export function auditSemanticTextRecords(records, policy = DEFAULT_UI_AUDIT_POLICY) {
  const findings = [];
  for (const input of records) {
    const record = normalizeRecord(input);
    if (!record || !VALID_CHANNELS.has(record.channel)) {
      findings.push(finding("invalid_record", input));
      continue;
    }
    const value = record.value.trim();
    if (!value) continue;

    const category = classifySemanticValue(value, policy);
    if (category) findings.push(finding(category, record, sanitizeEvidence(value)));
  }
  return Object.freeze({ state: findings.length === 0 ? "Passed" : "Failed", findings: Object.freeze(findings) });
}

export function buildRouteShellInventory(descriptors, sourceByPath) {
  const records = [];
  const findings = [];
  const seenRoutes = new Set();

  for (const descriptor of descriptors) {
    if (!descriptor?.route || !descriptor?.sourceFile || !descriptor?.shellMarker) {
      findings.push(finding("invalid_descriptor", descriptor));
      continue;
    }
    if (seenRoutes.has(descriptor.route)) findings.push(finding("duplicate_route", { route: descriptor.route, source: descriptor.sourceFile }));
    seenRoutes.add(descriptor.route);

    const source = sourceByPath[descriptor.sourceFile];
    if (typeof source !== "string") {
      findings.push(finding("source_missing", { route: descriptor.route, source: descriptor.sourceFile }));
      continue;
    }
    if (!source.includes(descriptor.shellMarker)) {
      findings.push(finding("shell_marker_missing", { route: descriptor.route, source: descriptor.sourceFile }, descriptor.shellMarker));
    }
    records.push(Object.freeze({
      route: descriptor.route,
      sourceFile: descriptor.sourceFile,
      shellMarker: descriptor.shellMarker,
      ownsSidebar: Boolean(descriptor.sidebarMarker && source.includes(descriptor.sidebarMarker)),
      ownsTopbar: Boolean(descriptor.topbarMarker && source.includes(descriptor.topbarMarker)),
      sourceHash: sha256(source),
    }));
  }

  return Object.freeze({ records: Object.freeze(records), findings: Object.freeze(findings) });
}

export function auditUiSource({ route, sourceFile, source, lineOffset = 0 }, policy = DEFAULT_UI_AUDIT_POLICY) {
  if (!route || !sourceFile || typeof source !== "string") {
    return Object.freeze({ state: "Failed", findings: Object.freeze([finding("invalid_source_input", { route, source: sourceFile })]) });
  }

  const findings = [];
  const lines = source.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const location = { route, source: sourceFile, line: index + 1 + lineOffset };

    for (const literal of extractUserFacingLiterals(line)) {
      const visibleLiteral = literal.value.replace(/\$\{[^}]*\}/g, "").trim();
      if (!visibleLiteral) continue;
      const category = classifySemanticValue(visibleLiteral, policy);
      if (category) findings.push(finding(category, { ...location, channel: literal.channel }, visibleLiteral));
    }

    for (const pattern of DIRECT_ID_RENDER_PATTERNS) {
      const match = line.match(pattern);
      if (match) {
        findings.push(finding("internal_identity", { ...location, channel: inferSourceChannel(line) }, match[1]));
        break;
      }
    }
    const errorMatch = line.match(DIRECT_ERROR_RENDER_PATTERN);
    if (errorMatch) findings.push(finding("internal_error_code", { ...location, channel: "visible_text" }, errorMatch[1]));

    const domIdMatch = line.match(ID_DOM_ATTRIBUTE_PATTERN);
    if (domIdMatch) findings.push(finding("internal_identity_dom_attribute", { ...location, channel: "dom_attribute" }, domIdMatch[1]));
  }
  return Object.freeze({ state: findings.length === 0 ? "Passed" : "Failed", findings: Object.freeze(findings) });
}

export function createSourceFingerprint(sourceByPath) {
  const stable = Object.entries(sourceByPath)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([path, source]) => `${path}\0${source.length}\0${source}`)
    .join("\0");
  return sha256(stable);
}

export function createFixtureHash(value) {
  return sha256(JSON.stringify(value));
}

export async function collectBaseline({ rootDir, descriptors, readText, policy = DEFAULT_UI_AUDIT_POLICY }) {
  if (typeof rootDir !== "string" || !Array.isArray(descriptors) || typeof readText !== "function") {
    throw new Error("INVALID_BASELINE_INPUT");
  }
  const sourceByPath = {};
  for (const sourceFile of [...new Set(descriptors.map((descriptor) => descriptor.sourceFile))]) {
    sourceByPath[sourceFile] = await readText(join(rootDir, sourceFile));
  }

  const inventory = buildRouteShellInventory(descriptors, sourceByPath);
  const segmentFindings = [];
  const sourceFindings = descriptors.flatMap((descriptor) => {
    const segment = sourceSegment(sourceByPath[descriptor.sourceFile], descriptor);
    if (segment.finding) segmentFindings.push(finding(segment.finding, { route: descriptor.route, source: descriptor.sourceFile }));
    return auditUiSource({
      route: descriptor.route,
      sourceFile: descriptor.sourceFile,
      source: segment.source,
      lineOffset: segment.lineOffset,
    }, policy).findings;
  });

  return createBaselineReport({
    sourceFingerprint: createSourceFingerprint(sourceByPath),
    fixtureHash: createFixtureHash(descriptors),
    inventory,
    findings: [...inventory.findings, ...segmentFindings, ...sourceFindings],
  });
}

export function createBaselineReport({ sourceFingerprint, fixtureHash, inventory, findings }) {
  const categories = {};
  for (const item of findings) categories[item.category] = (categories[item.category] ?? 0) + 1;
  return Object.freeze({
    marker: "phase013_ui_baseline=recorded",
    state: "BaselineRecorded",
    sourceFingerprint,
    fixtureHash,
    routeCount: inventory.records.length,
    shellOwnerCount: inventory.records.filter((record) => record.ownsSidebar || record.ownsTopbar).length,
    findingCount: findings.length,
    categoryCounts: Object.freeze(categories),
    inventory: Object.freeze([...inventory.records]),
    findings: Object.freeze([...findings]),
  });
}

export function validateBaselineReport(report, expected = {}) {
  const findingIds = [];
  if (report?.marker !== "phase013_ui_baseline=recorded") findingIds.push("marker");
  if (report?.state !== "BaselineRecorded") findingIds.push("state");
  if (!isSha256(report?.sourceFingerprint)) findingIds.push("source_fingerprint");
  if (!isSha256(report?.fixtureHash)) findingIds.push("fixture_hash");
  if (expected.sourceFingerprint && report?.sourceFingerprint !== expected.sourceFingerprint) findingIds.push("stale_source_fingerprint");
  if (expected.fixtureHash && report?.fixtureHash !== expected.fixtureHash) findingIds.push("stale_fixture_hash");
  if (!Array.isArray(report?.inventory) || !Array.isArray(report?.findings)) findingIds.push("collections");
  for (const item of [...(report?.inventory ?? []), ...(report?.findings ?? [])]) {
    if (typeof item?.source === "string" && isAbsolutePath(item.source, DEFAULT_UI_AUDIT_POLICY)) findingIds.push("unsafe_source_path");
    if (typeof item?.sourceFile === "string" && isAbsolutePath(item.sourceFile, DEFAULT_UI_AUDIT_POLICY)) findingIds.push("unsafe_source_path");
    if (Object.hasOwn(item ?? {}, "sourceText")) findingIds.push("source_text_payload");
  }
  return [...new Set(findingIds)];
}

export function renderBaselineArtifact(report) {
  const categoryLines = Object.entries(report.categoryCounts)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([category, count]) => `- ${category}: ${count}`);
  const inventoryRows = report.inventory.map((record) =>
    `| ${cell(record.route)} | \`${cell(record.sourceFile)}\` | \`${cell(record.shellMarker)}\` | ${record.ownsSidebar} | ${record.ownsTopbar} |`,
  );
  const findingRows = report.findings.map((item) =>
    `| ${cell(item.category)} | ${cell(item.route ?? "Unknown")} | ${cell(item.channel ?? "source")} | \`${cell(item.source ?? "synthetic")}:${Number(item.line ?? 0)}\` | \`${cell(item.evidence ?? "") || "-"}\` |`,
  );

  return [
    "# Phase 013 UI Baseline",
    "",
    report.marker,
    `state=${report.state}`,
    `source_fingerprint=${report.sourceFingerprint}`,
    `fixture_hash=${report.fixtureHash}`,
    `route_count=${report.routeCount}`,
    `shell_owner_count=${report.shellOwnerCount}`,
    `finding_count=${report.findingCount}`,
    "raw_user_content_excluded=true",
    "absolute_workspace_path_excluded=true",
    "",
    "## Finding Categories",
    "",
    ...(categoryLines.length ? categoryLines : ["- none: 0"]),
    "",
    "## Route/Shell Inventory",
    "",
    "| Route | Source | Shell marker | Owns sidebar | Owns topbar |",
    "| --- | --- | --- | --- | --- |",
    ...inventoryRows,
    "",
    "## Current Findings",
    "",
    "These findings are the failing baseline. They are not completion evidence.",
    "",
    "| Category | Route | Channel | Source | Sanitized evidence |",
    "| --- | --- | --- | --- | --- |",
    ...(findingRows.length ? findingRows : ["| none | - | - | - | - |"]),
    "",
  ].join("\n");
}

function normalizeRecord(input) {
  if (!input || typeof input !== "object") return undefined;
  if (typeof input.route !== "string" || typeof input.state !== "string" || typeof input.channel !== "string" || typeof input.value !== "string") return undefined;
  return { route: input.route, state: input.state, channel: input.channel, value: input.value, source: relativeSource(input.source), line: Number(input.line ?? 0) };
}

function classifySemanticValue(value, policy) {
  if (isAbsolutePath(value, policy)) return "absolute_path";
  if (policy.internalErrorCodePattern.test(value)) return "internal_error_code";
  if (policy.internalIdentityPatterns.some((pattern) => pattern.test(value))) return "internal_identity";
  if (isApprovedFileName(value, policy) || isApprovedCopy(value, policy)) return undefined;
  return /[A-Za-z]{2,}/.test(value) ? "mixed_language" : undefined;
}

function isApprovedCopy(value, policy) {
  let remaining = value;
  for (const term of policy.approvedEnglishTerms) remaining = remaining.replaceAll(term, "");
  return !/[A-Za-z]{2,}/.test(remaining);
}

function isApprovedFileName(value, policy) {
  return policy.approvedFileNamePattern.test(value) && !value.includes("..") && !value.includes(":");
}

function isAbsolutePath(value, policy) {
  return policy.absolutePathPatterns.some((pattern) => pattern.test(value));
}

function inferSourceChannel(line) {
  if (/aria-label/.test(line)) return "accessible_name";
  if (/\btitle\s*:/.test(line)) return "tooltip";
  return "visible_text";
}

function extractUserFacingLiterals(line) {
  const candidates = [];
  collectMatches(candidates, line, /(?:placeholder|title|"aria-label"|'aria-label')\s*:\s*"([^"]*)"/g, "tooltip", (match) =>
    /aria-label/.test(match[0]) ? "accessible_name" : /placeholder/.test(match[0]) ? "accessible_name" : "tooltip",
  );
  collectMatches(candidates, line, /(?:null|\})\s*,\s*"([^"]*)"/g, "visible_text");
  collectMatches(candidates, line, /(?:null|\})\s*,\s*`([^`]*)`/g, "visible_text");

  if (/return\s*\(\{/.test(line)) {
    collectMatches(candidates, line, /:\s*"([^"]*)"/g, "visible_text");
  }
  if (/\b(?:const|let)\s+[A-Za-z0-9_]*(?:Label|Status|Message|Title|Copy)\s*=/.test(line)) {
    collectMatches(candidates, line, /(?:\?|:)\s*"([^"]*)"/g, "visible_text");
    collectMatches(candidates, line, /(?:\?|:)\s*`([^`]*)`/g, "visible_text");
  }

  const deduplicated = new Map();
  for (const item of candidates) deduplicated.set(`${item.channel}\0${item.value}`, item);
  return [...deduplicated.values()];
}

function collectMatches(target, line, pattern, defaultChannel, channelForMatch) {
  for (const match of line.matchAll(pattern)) {
    const value = match[1];
    if (typeof value !== "string") continue;
    target.push({ value, channel: channelForMatch?.(match) ?? defaultChannel });
  }
}

function sourceSegment(source, descriptor) {
  if (!descriptor.auditStartMarker) return { source, lineOffset: 0 };
  const start = source.indexOf(descriptor.auditStartMarker);
  if (start < 0) return { source, lineOffset: 0, finding: "audit_start_marker_missing" };
  const end = descriptor.auditEndMarker ? source.indexOf(descriptor.auditEndMarker, start + descriptor.auditStartMarker.length) : source.length;
  if (end < 0) return { source: source.slice(start), lineOffset: lineOffsetFor(source, start), finding: "audit_end_marker_missing" };
  return { source: source.slice(start, end), lineOffset: lineOffsetFor(source, start) };
}

function lineOffsetFor(source, characterOffset) {
  return source.slice(0, characterOffset).split(/\r?\n/).length - 1;
}

function finding(category, record = {}, evidence = "") {
  return Object.freeze({
    category,
    route: typeof record?.route === "string" ? record.route : "Unknown",
    channel: typeof record?.channel === "string" ? record.channel : "source",
    source: relativeSource(record?.source),
    line: Number(record?.line ?? 0),
    evidence: sanitizeEvidence(evidence),
  });
}

function relativeSource(value) {
  if (typeof value !== "string" || value.length === 0) return "synthetic";
  return value.replaceAll("\\", "/").replace(/^.*\/(apps|packages|scripts)\//, "$1/");
}

function sanitizeEvidence(value) {
  return String(value ?? "").replace(/[\r\n|`]/g, " ").slice(0, 80);
}

function cell(value) {
  return sanitizeEvidence(value) || "-";
}

function isSha256(value) {
  return typeof value === "string" && /^[a-f0-9]{64}$/.test(value);
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}
