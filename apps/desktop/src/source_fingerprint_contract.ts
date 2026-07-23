export interface SourceFingerprintEntry {
  readonly path: string;
  readonly content: string;
}

export interface SourceFingerprint {
  readonly digest: string;
  readonly fileCount: number;
}

export class SourceFingerprintError extends Error {
  readonly code: "SOURCE_SET_EMPTY" | "SOURCE_PATH_INVALID" | "SOURCE_PATH_DUPLICATE" | "SOURCE_HASH_INVALID";

  constructor(code: SourceFingerprintError["code"]) {
    super(code);
    this.name = "SourceFingerprintError";
    this.code = code;
  }
}

export function createSourceFingerprint(
  entries: readonly SourceFingerprintEntry[],
  hash: (framedSource: string) => string,
): SourceFingerprint {
  if (entries.length === 0) throw new SourceFingerprintError("SOURCE_SET_EMPTY");
  const normalized = entries.map((entry) => {
    const path = entry.path.trim();
    if (path.length === 0 || path.startsWith("/") || path.includes("..") || path.includes("\u0000")) {
      throw new SourceFingerprintError("SOURCE_PATH_INVALID");
    }
    return { path, content: entry.content };
  }).sort((left, right) => left.path.localeCompare(right.path));
  if (new Set(normalized.map((entry) => entry.path)).size !== normalized.length) {
    throw new SourceFingerprintError("SOURCE_PATH_DUPLICATE");
  }
  const framed = normalized
    .map((entry) => `${entry.path.length}:${entry.path}${entry.content.length}:${entry.content}`)
    .join("");
  const digest = hash(framed).trim();
  if (digest.length === 0) throw new SourceFingerprintError("SOURCE_HASH_INVALID");
  return Object.freeze({ digest, fileCount: normalized.length });
}
