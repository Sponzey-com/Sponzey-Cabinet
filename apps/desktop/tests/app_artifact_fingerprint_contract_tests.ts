import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import {
  AppArtifactFingerprintError,
  createAppArtifactFingerprint,
  type AppArtifactEntry,
} from "../src/app_artifact_fingerprint_contract.ts";

const expectedPaths = ["app.bundle.js", "index.html", "styles.css"] as const;
const sha256 = (value: string | Uint8Array): string =>
  createHash("sha256").update(value).digest("hex");

test("app fingerprint is deterministic and independent of artifact input order", () => {
  const entries = fixtures();
  const first = createAppArtifactFingerprint({ expectedPaths, artifacts: entries, hash: sha256 });
  const second = createAppArtifactFingerprint({
    expectedPaths,
    artifacts: [...entries].reverse(),
    hash: sha256,
  });

  assert.deepEqual(first, second);
  assert.match(first.digest, /^[a-f0-9]{64}$/);
  assert.equal(first.artifactCount, 3);
  assert.equal(first.totalBytes, 60);
  assert.equal(Object.isFrozen(first), true);
});

test("app fingerprint changes when a required artifact digest changes", () => {
  const baseline = createAppArtifactFingerprint({ expectedPaths, artifacts: fixtures(), hash: sha256 });
  const changed = fixtures().map((entry) => entry.path === "styles.css"
    ? { ...entry, digest: sha256("changed") }
    : entry);

  assert.notEqual(
    createAppArtifactFingerprint({ expectedPaths, artifacts: changed, hash: sha256 }).digest,
    baseline.digest,
  );
});

test("app fingerprint rejects missing unexpected duplicate and unsafe artifacts", () => {
  assert.throws(() => createAppArtifactFingerprint({
    expectedPaths,
    artifacts: fixtures().slice(1),
    hash: sha256,
  }), fingerprintError("APP_ARTIFACT_MISSING"));

  assert.throws(() => createAppArtifactFingerprint({
    expectedPaths,
    artifacts: [...fixtures(), { path: "debug.map", digest: sha256("map"), byteLength: 3 }],
    hash: sha256,
  }), fingerprintError("APP_ARTIFACT_UNEXPECTED"));

  assert.throws(() => createAppArtifactFingerprint({
    expectedPaths,
    artifacts: [...fixtures(), fixtures()[0]!],
    hash: sha256,
  }), fingerprintError("APP_ARTIFACT_PATH_DUPLICATE"));

  assert.throws(() => createAppArtifactFingerprint({
    expectedPaths: ["../index.html"],
    artifacts: [{ path: "../index.html", digest: sha256("html"), byteLength: 4 }],
    hash: sha256,
  }), fingerprintError("APP_ARTIFACT_PATH_INVALID"));
});

test("artifact manifest produces a reproducible path-free fingerprint", () => {
  const artifacts: AppArtifactEntry[] = fixtures();
  const first = createAppArtifactFingerprint({ expectedPaths, artifacts, hash: sha256 });
  const second = createAppArtifactFingerprint({ expectedPaths, artifacts, hash: sha256 });
  assert.deepEqual(first, second);
  assert.match(first.digest, /^[a-f0-9]{64}$/);
  assert.equal(first.artifactCount, expectedPaths.length);
  assert.equal(first.totalBytes, 60);
  assert.deepEqual(Object.keys(first).sort(), ["artifactCount", "digest", "totalBytes"]);
});

function fixtures(): AppArtifactEntry[] {
  return [
    { path: "app.bundle.js", digest: sha256("bundle"), byteLength: 30 },
    { path: "index.html", digest: sha256("html"), byteLength: 10 },
    { path: "styles.css", digest: sha256("styles"), byteLength: 20 },
  ];
}

function fingerprintError(code: string): (error: unknown) => boolean {
  return (error) => error instanceof AppArtifactFingerprintError && error.code === code;
}
