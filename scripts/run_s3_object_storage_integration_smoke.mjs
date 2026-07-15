import crypto from "node:crypto";
import http from "node:http";
import https from "node:https";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const root = process.cwd();
const outputArtifactPath = join(root, ".tmp", "s3-object-storage-integration-smoke-output.txt");
const requiredConfigKeys = ["endpoint", "bucket", "accessKeyId", "secretAccessKey"];
const stableErrors = Object.freeze({
  InvalidConfig: "OBJECT_STORAGE_INTEGRATION_INVALID_CONFIG",
  HealthDegraded: "OBJECT_STORAGE_HEALTH_DEGRADED",
  MissingObjectUnexpected: "OBJECT_STORAGE_MISSING_OBJECT_UNEXPECTED",
  PutFailed: "OBJECT_STORAGE_PUT_FAILED",
  MetadataReadFailed: "OBJECT_STORAGE_METADATA_READ_FAILED",
  ContentReadFailed: "OBJECT_STORAGE_CONTENT_READ_FAILED",
  DeleteFailed: "OBJECT_STORAGE_DELETE_FAILED",
  ContentMismatch: "OBJECT_STORAGE_CONTENT_MISMATCH",
  RequestFailed: "OBJECT_STORAGE_REQUEST_FAILED",
  InvalidTransition: "OBJECT_STORAGE_SMOKE_INVALID_TRANSITION",
});

export const S3SmokeState = Object.freeze({
  NotConfigured: "NotConfigured",
  Configured: "Configured",
  Running: "Running",
  Passed: "Passed",
  Failed: "Failed",
});

export const S3SmokeEvent = Object.freeze({
  LoadConfig: "LoadConfig",
  ValidateConfig: "ValidateConfig",
  RunOperation: "RunOperation",
  Complete: "Complete",
  Fail: "Fail",
});

export function transitionS3SmokeState(currentState, event, detail = {}) {
  if (currentState === S3SmokeState.NotConfigured && event === S3SmokeEvent.LoadConfig) {
    return { state: detail.configured ? S3SmokeState.Configured : S3SmokeState.NotConfigured };
  }
  if (currentState === S3SmokeState.Configured && event === S3SmokeEvent.ValidateConfig) {
    return { state: S3SmokeState.Configured };
  }
  if (currentState === S3SmokeState.Configured && event === S3SmokeEvent.RunOperation) {
    return { state: S3SmokeState.Running };
  }
  if (currentState === S3SmokeState.Running && event === S3SmokeEvent.Complete) {
    return { state: S3SmokeState.Passed };
  }
  if (
    [S3SmokeState.Configured, S3SmokeState.Running].includes(currentState) &&
    event === S3SmokeEvent.Fail
  ) {
    return {
      state: S3SmokeState.Failed,
      errorCode: detail.errorCode ?? stableErrors.RequestFailed,
      operation: detail.operation,
    };
  }
  return {
    state: S3SmokeState.Failed,
    errorCode: stableErrors.InvalidTransition,
  };
}

export function parseS3SmokeConfig(argv) {
  const raw = parseArgs(argv);
  if (requiredConfigKeys.every((key) => raw[key] === undefined)) {
    return {
      configured: false,
      state: S3SmokeState.NotConfigured,
    };
  }

  const missing = requiredConfigKeys.filter((key) => !isNonEmptyString(raw[key]));
  if (missing.length > 0) {
    return failedConfig(stableErrors.InvalidConfig, missing);
  }

  let endpoint;
  try {
    endpoint = normalizeEndpoint(raw.endpoint);
  } catch {
    return failedConfig(stableErrors.InvalidConfig, ["endpoint"]);
  }

  const region = raw.region?.trim() || "us-east-1";
  const prefix = raw.prefix?.trim() || `sponzey-cabinet-smoke-${Date.now()}`;
  const bucket = raw.bucket.trim();
  if (!/^[a-zA-Z0-9][a-zA-Z0-9._-]*$/.test(bucket) || bucket.includes("/")) {
    return failedConfig(stableErrors.InvalidConfig, ["bucket"]);
  }

  return {
    configured: true,
    state: S3SmokeState.Configured,
    config: {
      endpoint,
      bucket,
      accessKeyId: raw.accessKeyId.trim(),
      secretAccessKey: raw.secretAccessKey,
      region,
      prefix,
      forcePathStyle: raw.forcePathStyle !== "false",
    },
  };
}

export function renderRedactedS3Config(config) {
  return {
    endpoint_hash: hashForLog(config.endpoint),
    bucket_hash: hashForLog(config.bucket),
    access_key_id_hash: hashForLog(config.accessKeyId),
    secret_access_key: "redacted",
    region: config.region,
    prefix_hash: hashForLog(config.prefix),
    force_path_style: String(config.forcePathStyle),
  };
}

export function renderS3SmokeResult(result) {
  const lines = [];
  if (result.status === "not_configured") {
    lines.push("s3_object_storage_integration_smoke=not_configured");
    lines.push(`smoke_state=${S3SmokeState.NotConfigured}`);
    lines.push("reason=explicit_config_required");
    return lines.join("\n");
  }

  if (result.status === "passed") {
    lines.push("s3_object_storage_integration_smoke=passed");
    lines.push(`smoke_state=${S3SmokeState.Passed}`);
    lines.push(`operation_count=${result.operationCount}`);
    lines.push(renderConfigSummaryLine(result.redactedConfig));
    return lines.join("\n");
  }

  lines.push("s3_object_storage_integration_smoke=failed");
  lines.push(`smoke_state=${S3SmokeState.Failed}`);
  lines.push(`error_code=${result.errorCode}`);
  if (result.operation) {
    lines.push(`operation=${result.operation}`);
  }
  if (result.redactedConfig) {
    lines.push(renderConfigSummaryLine(result.redactedConfig));
  }
  return lines.join("\n");
}

export async function runS3CompatibleObjectStorageSmoke({
  config,
  client = createS3CompatibleSmokeClient(config),
}) {
  let state = transitionS3SmokeState(S3SmokeState.NotConfigured, S3SmokeEvent.LoadConfig, {
    configured: true,
  });
  state = transitionS3SmokeState(state.state, S3SmokeEvent.ValidateConfig);
  state = transitionS3SmokeState(state.state, S3SmokeEvent.RunOperation);

  const redactedConfig = renderRedactedS3Config(config);
  const key = `${config.prefix}/object-${crypto.randomUUID()}.txt`;
  const missingKey = `${config.prefix}/missing-${crypto.randomUUID()}.txt`;
  const content = "sponzey cabinet object storage integration content";
  const metadataValue = "metadata-content-split";
  const operations = [];

  try {
    const health = await client.probeHealth();
    operations.push("probeHealth");
    if (health.status !== "healthy") {
      state = transitionS3SmokeState(state.state, S3SmokeEvent.Fail, {
        errorCode: stableErrors.HealthDegraded,
        operation: "probeHealth",
      });
      return failedSmokeResult(state, redactedConfig);
    }

    const missing = await client.headMetadata(missingKey);
    operations.push("headMissingMetadata");
    if (missing.exists) {
      state = transitionS3SmokeState(state.state, S3SmokeEvent.Fail, {
        errorCode: stableErrors.MissingObjectUnexpected,
        operation: "headMissingMetadata",
      });
      return failedSmokeResult(state, redactedConfig);
    }

    await client.putObject(key, content, { cabinetFixture: metadataValue });
    operations.push("putObject");

    const metadata = await client.headMetadata(key);
    operations.push("headMetadata");
    if (!metadata.exists || metadata.metadata.cabinetFixture !== metadataValue) {
      state = transitionS3SmokeState(state.state, S3SmokeEvent.Fail, {
        errorCode: stableErrors.MetadataReadFailed,
        operation: "headMetadata",
      });
      return failedSmokeResult(state, redactedConfig);
    }

    const loaded = await client.getObject(key);
    operations.push("getObject");
    if (loaded.content !== content) {
      state = transitionS3SmokeState(state.state, S3SmokeEvent.Fail, {
        errorCode: stableErrors.ContentMismatch,
        operation: "getObject",
      });
      return failedSmokeResult(state, redactedConfig);
    }

    await client.deleteObject(key);
    operations.push("deleteObject");
    await client.deleteObject(key);
    operations.push("deleteObjectAgain");

    const deleted = await client.headMetadata(key);
    operations.push("headDeletedMetadata");
    if (deleted.exists) {
      state = transitionS3SmokeState(state.state, S3SmokeEvent.Fail, {
        errorCode: stableErrors.DeleteFailed,
        operation: "headDeletedMetadata",
      });
      return failedSmokeResult(state, redactedConfig);
    }

    state = transitionS3SmokeState(state.state, S3SmokeEvent.Complete);
    return {
      status: "passed",
      state: state.state,
      operationCount: operations.length,
      redactedConfig,
    };
  } catch (error) {
    state = transitionS3SmokeState(state.state, S3SmokeEvent.Fail, {
      errorCode: error instanceof S3SmokeError ? error.code : stableErrors.RequestFailed,
      operation: error instanceof S3SmokeError ? error.operation : "request",
    });
    return failedSmokeResult(state, redactedConfig);
  }
}

export function createS3CompatibleSmokeClient(config, transport = defaultHttpTransport) {
  return {
    async probeHealth() {
      const response = await signedS3Request(config, transport, "HEAD", "");
      if (response.status >= 200 && response.status < 300) {
        return { status: "healthy" };
      }
      return { status: "degraded", errorCode: mapS3Status(response.status) };
    },

    async headMetadata(key) {
      const response = await signedS3Request(config, transport, "HEAD", key);
      if (response.status === 404) {
        return { exists: false };
      }
      if (response.status < 200 || response.status >= 300) {
        throw new S3SmokeError(stableErrors.MetadataReadFailed, "headMetadata");
      }
      return {
        exists: true,
        metadata: {
          cabinetFixture: response.headers["x-amz-meta-cabinet-fixture"],
          contentType: response.headers["content-type"],
          contentLength: response.headers["content-length"],
        },
      };
    },

    async putObject(key, content, metadata) {
      const response = await signedS3Request(config, transport, "PUT", key, content, {
        "content-type": "text/plain; charset=utf-8",
        "x-amz-meta-cabinet-fixture": metadata.cabinetFixture,
      });
      if (response.status < 200 || response.status >= 300) {
        throw new S3SmokeError(stableErrors.PutFailed, "putObject");
      }
    },

    async getObject(key) {
      const response = await signedS3Request(config, transport, "GET", key);
      if (response.status < 200 || response.status >= 300) {
        throw new S3SmokeError(stableErrors.ContentReadFailed, "getObject");
      }
      return { content: response.body };
    },

    async deleteObject(key) {
      const response = await signedS3Request(config, transport, "DELETE", key);
      if (response.status !== 404 && (response.status < 200 || response.status >= 300)) {
        throw new S3SmokeError(stableErrors.DeleteFailed, "deleteObject");
      }
    },
  };
}

async function signedS3Request(config, transport, method, key, body = "", headers = {}) {
  const request = buildSignedS3Request(config, method, key, body, headers);
  return transport(request);
}

export function buildSignedS3Request(config, method, key, body = "", headers = {}) {
  const now = new Date();
  const amzDate = toAmzDate(now);
  const dateStamp = amzDate.slice(0, 8);
  const url = buildS3Url(config, key);
  const payloadHash = sha256Hex(body);
  const lowerHeaders = normalizeHeaders({
    ...headers,
    host: url.host,
    "x-amz-content-sha256": payloadHash,
    "x-amz-date": amzDate,
  });
  const signedHeaders = Object.keys(lowerHeaders).sort().join(";");
  const canonicalHeaders = Object.keys(lowerHeaders)
    .sort()
    .map((name) => `${name}:${lowerHeaders[name]}\n`)
    .join("");
  const canonicalRequest = [
    method,
    url.pathname,
    url.searchParams.toString(),
    canonicalHeaders,
    signedHeaders,
    payloadHash,
  ].join("\n");
  const credentialScope = `${dateStamp}/${config.region}/s3/aws4_request`;
  const stringToSign = [
    "AWS4-HMAC-SHA256",
    amzDate,
    credentialScope,
    sha256Hex(canonicalRequest),
  ].join("\n");
  const signature = hmacHex(signingKey(config.secretAccessKey, dateStamp, config.region), stringToSign);

  return {
    method,
    url: url.toString(),
    headers: {
      ...lowerHeaders,
      authorization:
        `AWS4-HMAC-SHA256 Credential=${config.accessKeyId}/${credentialScope}, SignedHeaders=${signedHeaders}, Signature=${signature}`,
    },
    body: body.length > 0 ? body : undefined,
  };
}

async function defaultHttpTransport(request) {
  return new Promise((resolve, reject) => {
    const url = new URL(request.url);
    const client = url.protocol === "https:" ? https : http;
    const req = client.request(
      url,
      {
        method: request.method,
        headers: request.headers,
      },
      (res) => {
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => {
          resolve({
            status: res.statusCode ?? 0,
            headers: normalizeHeaders(res.headers),
            body: Buffer.concat(chunks).toString("utf8"),
          });
        });
      },
    );
    req.on("error", () => reject(new S3SmokeError(stableErrors.RequestFailed, "request")));
    if (request.body) {
      req.write(request.body);
    }
    req.end();
  });
}

function buildS3Url(config, key) {
  const url = new URL(config.endpoint);
  const basePath = url.pathname.replace(/\/+$/, "");
  const encodedBucket = encodePathPart(config.bucket);
  const encodedKey = key
    .split("/")
    .filter((part) => part.length > 0)
    .map(encodePathPart)
    .join("/");
  url.pathname = encodedKey
    ? `${basePath}/${encodedBucket}/${encodedKey}`
    : `${basePath}/${encodedBucket}`;
  return url;
}

function normalizeEndpoint(value) {
  const endpoint = new URL(value.trim());
  if (endpoint.protocol !== "http:" && endpoint.protocol !== "https:") {
    throw new Error("unsupported_protocol");
  }
  endpoint.pathname = endpoint.pathname.replace(/\/+$/, "");
  endpoint.search = "";
  endpoint.hash = "";
  return endpoint.toString().replace(/\/+$/, "");
}

function parseArgs(argv) {
  const parsed = {};
  for (let index = 0; index < argv.length; index += 1) {
    const current = argv[index];
    if (!current.startsWith("--")) {
      continue;
    }
    const key = current.slice(2);
    const normalized = {
      endpoint: "endpoint",
      bucket: "bucket",
      "access-key-id": "accessKeyId",
      "secret-access-key": "secretAccessKey",
      region: "region",
      prefix: "prefix",
      "force-path-style": "forcePathStyle",
    }[key];
    if (!normalized) {
      continue;
    }
    parsed[normalized] = argv[index + 1];
    index += 1;
  }
  return parsed;
}

function failedConfig(errorCode, missingFields) {
  return {
    configured: true,
    state: S3SmokeState.Failed,
    errorCode,
    missingFields,
  };
}

function failedSmokeResult(state, redactedConfig) {
  return {
    status: "failed",
    state: state.state,
    errorCode: state.errorCode,
    operation: state.operation,
    redactedConfig,
  };
}

function renderConfigSummaryLine(redactedConfig) {
  return Object.entries(redactedConfig)
    .map(([key, value]) => `${key}=${value}`)
    .join(" ");
}

function mapS3Status(status) {
  if (status === 401 || status === 403) {
    return "object_storage.unauthorized";
  }
  if (status === 404) {
    return "object_storage.missing_bucket";
  }
  if (status === 408 || status === 429 || status >= 500) {
    return "object_storage.storage_unavailable";
  }
  return "object_storage.api_error";
}

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function normalizeHeaders(headers) {
  const record = {};
  for (const [key, value] of Object.entries(headers)) {
    if (value === undefined) {
      continue;
    }
    const stringValue = Array.isArray(value) ? value.join(",") : String(value);
    record[key.toLowerCase()] = stringValue.trim().replace(/\s+/g, " ");
  }
  return record;
}

function encodePathPart(value) {
  return encodeURIComponent(value).replace(/%2F/g, "/");
}

function hashForLog(value) {
  return `sha256:${sha256Hex(value).slice(0, 12)}`;
}

function sha256Hex(value) {
  return crypto.createHash("sha256").update(value).digest("hex");
}

function hmac(key, value) {
  return crypto.createHmac("sha256", key).update(value).digest();
}

function hmacHex(key, value) {
  return crypto.createHmac("sha256", key).update(value).digest("hex");
}

function signingKey(secretAccessKey, dateStamp, region) {
  const dateKey = hmac(`AWS4${secretAccessKey}`, dateStamp);
  const regionKey = hmac(dateKey, region);
  const serviceKey = hmac(regionKey, "s3");
  return hmac(serviceKey, "aws4_request");
}

function toAmzDate(date) {
  return date.toISOString().replace(/[:-]|\.\d{3}/g, "");
}

class S3SmokeError extends Error {
  constructor(code, operation) {
    super(code);
    this.code = code;
    this.operation = operation;
  }
}

async function writeOutputArtifact(output) {
  await mkdir(join(root, ".tmp"), { recursive: true });
  await writeFile(outputArtifactPath, `${output}\n`);
}

async function runCli() {
  const parsed = parseS3SmokeConfig(process.argv.slice(2));
  let result;
  if (!parsed.configured) {
    result = { status: "not_configured" };
  } else if (parsed.state === S3SmokeState.Failed) {
    result = {
      status: "failed",
      errorCode: parsed.errorCode,
      operation: "validateConfig",
    };
  } else {
    result = await runS3CompatibleObjectStorageSmoke({ config: parsed.config });
  }

  const rendered = renderS3SmokeResult(result);
  await writeOutputArtifact(rendered);
  if (result.status === "failed") {
    console.error(rendered);
    process.exit(1);
  }
  console.log(rendered);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  runCli().catch(async () => {
    const rendered = renderS3SmokeResult({
      status: "failed",
      errorCode: stableErrors.RequestFailed,
      operation: "unexpected",
    });
    await writeOutputArtifact(rendered);
    console.error(rendered);
    process.exit(1);
  });
}
