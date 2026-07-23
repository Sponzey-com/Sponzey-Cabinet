const SHA256_PATTERN = /^[a-f0-9]{64}$/;

export type DesktopBuildIdentity = Readonly<{
  sourceFingerprint: string;
  sourceFileCount: number;
  appFingerprint: string;
  artifactCount: number;
  totalArtifactBytes: number;
  identityFingerprint: string;
}>;

export class DesktopBuildIdentityError extends Error {
  readonly code: string;

  constructor(code: string) {
    super(code);
    this.name = "DesktopBuildIdentityError";
    this.code = code;
  }
}

export function createDesktopBuildIdentity(input: Readonly<{
  sourceFingerprint: string;
  sourceFileCount: number;
  appFingerprint: string;
  artifactCount: number;
  totalArtifactBytes: number;
  hash: (framedIdentity: string) => string;
}>): DesktopBuildIdentity {
  if (!SHA256_PATTERN.test(input.sourceFingerprint)
    || !SHA256_PATTERN.test(input.appFingerprint)) {
    fail("DESKTOP_BUILD_IDENTITY_FINGERPRINT_INVALID");
  }
  if (![input.sourceFileCount, input.artifactCount, input.totalArtifactBytes]
    .every((value) => Number.isSafeInteger(value) && value > 0)) {
    fail("DESKTOP_BUILD_IDENTITY_COUNT_INVALID");
  }

  const fields = [
    input.sourceFingerprint,
    String(input.sourceFileCount),
    input.appFingerprint,
    String(input.artifactCount),
    String(input.totalArtifactBytes),
  ];
  const framedIdentity = fields.map((value) => `${value.length}:${value}`).join("");
  const identityFingerprint = input.hash(framedIdentity).trim();
  if (!SHA256_PATTERN.test(identityFingerprint)) {
    fail("DESKTOP_BUILD_IDENTITY_HASH_INVALID");
  }

  return Object.freeze({
    sourceFingerprint: input.sourceFingerprint,
    sourceFileCount: input.sourceFileCount,
    appFingerprint: input.appFingerprint,
    artifactCount: input.artifactCount,
    totalArtifactBytes: input.totalArtifactBytes,
    identityFingerprint,
  });
}

function fail(code: string): never {
  throw new DesktopBuildIdentityError(code);
}
