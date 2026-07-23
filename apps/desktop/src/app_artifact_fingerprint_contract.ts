const SHA256_PATTERN = /^[a-f0-9]{64}$/;

export interface AppArtifactEntry {
  readonly path: string;
  readonly digest: string;
  readonly byteLength: number;
}

export type AppArtifactFingerprint = Readonly<{
  digest: string;
  artifactCount: number;
  totalBytes: number;
}>;

export class AppArtifactFingerprintError extends Error {
  readonly code: string;

  constructor(code: string) {
    super(code);
    this.name = "AppArtifactFingerprintError";
    this.code = code;
  }
}

export function createAppArtifactFingerprint(input: Readonly<{
  expectedPaths: readonly string[];
  artifacts: readonly AppArtifactEntry[];
  hash: (framedManifest: string) => string;
}>): AppArtifactFingerprint {
  if (!Array.isArray(input.expectedPaths) || input.expectedPaths.length === 0) {
    fail("APP_ARTIFACT_EXPECTED_EMPTY");
  }
  if (!Array.isArray(input.artifacts) || input.artifacts.length === 0) {
    fail("APP_ARTIFACT_SET_EMPTY");
  }

  const expectedPaths = input.expectedPaths.map(validatePath);
  if (new Set(expectedPaths).size !== expectedPaths.length) {
    fail("APP_ARTIFACT_EXPECTED_DUPLICATE");
  }

  const artifacts = input.artifacts.map((artifact) => {
    const path = validatePath(artifact?.path);
    if (typeof artifact.digest !== "string" || !SHA256_PATTERN.test(artifact.digest)) {
      fail("APP_ARTIFACT_DIGEST_INVALID");
    }
    if (!Number.isSafeInteger(artifact.byteLength) || artifact.byteLength <= 0) {
      fail("APP_ARTIFACT_SIZE_INVALID");
    }
    return Object.freeze({ path, digest: artifact.digest, byteLength: artifact.byteLength });
  });
  if (new Set(artifacts.map((artifact) => artifact.path)).size !== artifacts.length) {
    fail("APP_ARTIFACT_PATH_DUPLICATE");
  }

  const expected = new Set(expectedPaths);
  const actual = new Set(artifacts.map((artifact) => artifact.path));
  if (expectedPaths.some((path) => !actual.has(path))) fail("APP_ARTIFACT_MISSING");
  if (artifacts.some((artifact) => !expected.has(artifact.path))) fail("APP_ARTIFACT_UNEXPECTED");

  const ordered = [...artifacts].sort((left, right) => left.path.localeCompare(right.path));
  const framedManifest = ordered.map((artifact) =>
    `${artifact.path.length}:${artifact.path}${String(artifact.byteLength).length}:${artifact.byteLength}${artifact.digest.length}:${artifact.digest}`
  ).join("");
  const digest = input.hash(framedManifest).trim();
  if (!SHA256_PATTERN.test(digest)) fail("APP_ARTIFACT_HASH_INVALID");

  return Object.freeze({
    digest,
    artifactCount: ordered.length,
    totalBytes: ordered.reduce((total, artifact) => total + artifact.byteLength, 0),
  });
}

function validatePath(value: unknown): string {
  if (typeof value !== "string") fail("APP_ARTIFACT_PATH_INVALID");
  const path = value.trim();
  if (path.length === 0
    || path.startsWith("/")
    || path.includes("\\")
    || path.includes("\u0000")
    || path.split("/").some((segment) => segment.length === 0 || segment === "." || segment === "..")) {
    fail("APP_ARTIFACT_PATH_INVALID");
  }
  return path;
}

function fail(code: string): never {
  throw new AppArtifactFingerprintError(code);
}
