import assert from "node:assert/strict";
import test from "node:test";

import {
  buildSignedS3Request,
  parseS3SmokeConfig,
  renderRedactedS3Config,
  renderS3SmokeResult,
  runS3CompatibleObjectStorageSmoke,
  transitionS3SmokeState,
  S3SmokeEvent,
  S3SmokeState,
} from "./run_s3_object_storage_integration_smoke.mjs";

const config = {
  endpoint: "https://s3.fixture.example",
  bucket: "cabinet-fixture-bucket",
  accessKeyId: "fixture-access-key-id",
  secretAccessKey: "fixture-secret-access-key",
  region: "us-east-1",
  prefix: "cabinet-smoke-prefix",
  forcePathStyle: true,
};

test("s3 smoke config reports explicit not configured result", () => {
  const parsed = parseS3SmokeConfig([]);
  const rendered = renderS3SmokeResult({ status: "not_configured" });

  assert.equal(parsed.configured, false);
  assert.equal(parsed.state, "NotConfigured");
  assert.match(rendered, /s3_object_storage_integration_smoke=not_configured/);
  assert.match(rendered, /explicit_config_required/);
});

test("s3 smoke config validates required config without logging raw values", () => {
  const parsed = parseS3SmokeConfig([
    "--endpoint",
    "not-a-url",
    "--bucket",
    "cabinet-fixture-bucket",
    "--access-key-id",
    "fixture-access-key-id",
    "--secret-access-key",
    "fixture-secret-access-key",
  ]);
  const rendered = renderS3SmokeResult({
    status: "failed",
    errorCode: parsed.errorCode,
    operation: "validateConfig",
  });

  assert.equal(parsed.state, "Failed");
  assert.equal(parsed.errorCode, "OBJECT_STORAGE_INTEGRATION_INVALID_CONFIG");
  assert.doesNotMatch(rendered, /fixture-access-key-id|fixture-secret-access-key|cabinet-fixture-bucket/);
});

test("s3 smoke redacts endpoint bucket access key and secret", () => {
  const redacted = renderRedactedS3Config(config);
  const rendered = renderS3SmokeResult({
    status: "passed",
    operationCount: 8,
    redactedConfig: redacted,
  });

  assert.match(rendered, /endpoint_hash=sha256:/);
  assert.match(rendered, /bucket_hash=sha256:/);
  assert.match(rendered, /access_key_id_hash=sha256:/);
  assert.match(rendered, /secret_access_key=redacted/);
  assert.doesNotMatch(
    rendered,
    /s3\.fixture\.example|cabinet-fixture-bucket|fixture-access-key-id|fixture-secret-access-key/,
  );
});

test("s3 smoke verifies metadata content split missing object health and delete idempotency", async () => {
  const client = new FakeS3SmokeClient();
  const result = await runS3CompatibleObjectStorageSmoke({ config, client });

  assert.equal(result.status, "passed");
  assert.deepEqual(client.calls, [
    "probeHealth",
    "headMetadata:missing",
    "putObject",
    "headMetadata:object",
    "getObject",
    "deleteObject:object",
    "deleteObject:missing",
    "headMetadata:missing",
  ]);
});

test("s3 smoke reports health degraded with stable error and redacted context", async () => {
  const client = new FakeS3SmokeClient({ health: "degraded" });
  const result = await runS3CompatibleObjectStorageSmoke({ config, client });
  const rendered = renderS3SmokeResult(result);

  assert.equal(result.status, "failed");
  assert.equal(result.errorCode, "OBJECT_STORAGE_HEALTH_DEGRADED");
  assert.equal(result.operation, "probeHealth");
  assert.doesNotMatch(rendered, /fixture-secret-access-key|fixture-access-key-id|cabinet-fixture-bucket/);
});

test("s3 smoke state machine exposes explicit terminal states", () => {
  const configured = transitionS3SmokeState(S3SmokeState.NotConfigured, S3SmokeEvent.LoadConfig, {
    configured: true,
  });
  const running = transitionS3SmokeState(configured.state, S3SmokeEvent.RunOperation);
  const passed = transitionS3SmokeState(running.state, S3SmokeEvent.Complete);
  const failed = transitionS3SmokeState(running.state, S3SmokeEvent.Fail, {
    errorCode: "OBJECT_STORAGE_PUT_FAILED",
    operation: "putObject",
  });

  assert.equal(configured.state, "Configured");
  assert.equal(running.state, "Running");
  assert.equal(passed.state, "Passed");
  assert.equal(failed.state, "Failed");
  assert.equal(failed.errorCode, "OBJECT_STORAGE_PUT_FAILED");
});

test("s3 signed request keeps provider details at script boundary", () => {
  const request = buildSignedS3Request(config, "PUT", "prefix/object.txt", "body", {
    "content-type": "text/plain",
  });

  assert.equal(request.method, "PUT");
  assert.equal(request.url, "https://s3.fixture.example/cabinet-fixture-bucket/prefix/object.txt");
  assert.match(request.headers.authorization, /^AWS4-HMAC-SHA256 Credential=fixture-access-key-id\//);
  assert.equal(request.headers["x-amz-content-sha256"].length, 64);
});

class FakeS3SmokeClient {
  calls = [];
  #content = "";
  #metadata = undefined;

  constructor(options = {}) {
    this.options = options;
  }

  async probeHealth() {
    this.calls.push("probeHealth");
    return this.options.health === "degraded"
      ? { status: "degraded", errorCode: "object_storage.storage_unavailable" }
      : { status: "healthy" };
  }

  async headMetadata(key) {
    const objectExists = this.#metadata !== undefined && key.includes("/object-");
    this.calls.push(`headMetadata:${objectExists ? "object" : "missing"}`);
    return objectExists
      ? {
          exists: true,
          metadata: this.#metadata,
        }
      : { exists: false };
  }

  async putObject(_key, content, metadata) {
    this.calls.push("putObject");
    this.#content = content;
    this.#metadata = metadata;
  }

  async getObject() {
    this.calls.push("getObject");
    return { content: this.#content };
  }

  async deleteObject() {
    this.calls.push(`deleteObject:${this.#metadata ? "object" : "missing"}`);
    this.#metadata = undefined;
    this.#content = "";
  }
}
