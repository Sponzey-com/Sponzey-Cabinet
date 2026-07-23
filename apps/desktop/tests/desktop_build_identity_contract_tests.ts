import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import test from "node:test";

import { buildDesktopAssets } from "../../../scripts/desktop_asset_builder.mjs";
import {
  DesktopBuildIdentityError,
  createDesktopBuildIdentity,
} from "../src/desktop_build_identity_contract.ts";
import { createAppArtifactFingerprint } from "../src/app_artifact_fingerprint_contract.ts";
import { createSourceFingerprint } from "../src/source_fingerprint_contract.ts";

const sha256 = (value: string | Uint8Array): string =>
  createHash("sha256").update(value).digest("hex");
const hash = (character: string): string => character.repeat(64);

test("desktop build identity is immutable deterministic and content-bound", () => {
  const input = {
    sourceFingerprint: hash("a"),
    sourceFileCount: 100,
    appFingerprint: hash("b"),
    artifactCount: 3,
    totalArtifactBytes: 1_000_000,
    hash: sha256,
  } as const;
  const first = createDesktopBuildIdentity(input);
  const second = createDesktopBuildIdentity(input);
  const changed = createDesktopBuildIdentity({ ...input, sourceFingerprint: hash("c") });

  assert.deepEqual(first, second);
  assert.notEqual(first.identityFingerprint, changed.identityFingerprint);
  assert.match(first.identityFingerprint, /^[a-f0-9]{64}$/);
  assert.deepEqual(Object.keys(first).sort(), [
    "appFingerprint",
    "artifactCount",
    "identityFingerprint",
    "sourceFileCount",
    "sourceFingerprint",
    "totalArtifactBytes",
  ]);
  assert.equal(Object.isFrozen(first), true);
});

test("desktop build identity rejects invalid digest and non-positive counts", () => {
  const valid = {
    sourceFingerprint: hash("a"), sourceFileCount: 1,
    appFingerprint: hash("b"), artifactCount: 1, totalArtifactBytes: 1,
    hash: sha256,
  };
  assert.throws(() => createDesktopBuildIdentity({ ...valid, sourceFingerprint: "stale" }),
    identityError("DESKTOP_BUILD_IDENTITY_FINGERPRINT_INVALID"));
  assert.throws(() => createDesktopBuildIdentity({ ...valid, artifactCount: 0 }),
    identityError("DESKTOP_BUILD_IDENTITY_COUNT_INVALID"));
  assert.throws(() => createDesktopBuildIdentity({ ...valid, hash: () => "invalid" }),
    identityError("DESKTOP_BUILD_IDENTITY_HASH_INVALID"));
});

test("current desktop builder returns exact repository source inputs for a reproducible identity", async () => {
  const root = process.cwd();
  const build = await buildDesktopAssets(root);
  assert.ok(Array.isArray(build.sourcePaths));
  assert.ok(build.sourcePaths.length > 2);
  assert.deepEqual(build.sourcePaths, [...build.sourcePaths].sort());
  assert.equal(new Set(build.sourcePaths).size, build.sourcePaths.length);
  assert.ok(build.sourcePaths.includes("apps/desktop/src/desktop_entry.ts"));
  assert.ok(build.sourcePaths.includes("apps/desktop/public/index.html"));
  assert.ok(build.sourcePaths.includes("apps/desktop/public/styles.css"));
  assert.equal(build.sourcePaths.some((path: string) => path.startsWith("/") || path.includes("node_modules")), false);

  const sourceEntries = await Promise.all(build.sourcePaths.map(async (path: string) => ({
    path,
    content: await readFile(join(root, path), "utf8"),
  })));
  const source = createSourceFingerprint(sourceEntries, sha256);

  const artifactPaths = ["app.bundle.js", "index.html", "styles.css"] as const;
  const artifacts = await Promise.all(artifactPaths.map(async (path) => {
    const bytes = await readFile(join(build.distDir, path));
    return { path, digest: sha256(bytes), byteLength: bytes.byteLength };
  }));
  const app = createAppArtifactFingerprint({ expectedPaths: artifactPaths, artifacts, hash: sha256 });
  const identity = createDesktopBuildIdentity({
    sourceFingerprint: source.digest,
    sourceFileCount: source.fileCount,
    appFingerprint: app.digest,
    artifactCount: app.artifactCount,
    totalArtifactBytes: app.totalBytes,
    hash: sha256,
  });

  assert.match(identity.sourceFingerprint, /^[a-f0-9]{64}$/);
  assert.match(identity.appFingerprint, /^[a-f0-9]{64}$/);
  assert.match(identity.identityFingerprint, /^[a-f0-9]{64}$/);
  assert.equal(identity.sourceFileCount, build.sourcePaths.length);
  assert.equal(identity.artifactCount, artifactPaths.length);
});

function identityError(code: string): (error: unknown) => boolean {
  return (error) => error instanceof DesktopBuildIdentityError && error.code === code;
}
