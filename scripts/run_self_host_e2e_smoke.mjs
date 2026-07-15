import { spawn } from "node:child_process";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import net from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";

const root = process.cwd();
const serverOutputArtifactPath = join(root, ".tmp", "self-host-e2e-server-output.txt");
const sensitiveFixtures = [
  "e2e-password-should-not-log",
  "e2e-session-token-should-not-log",
  "E2E document body should not be logged",
  "comment body should not leak",
  "asset-content-should-not-log",
  "phase002-secret-fixture-should-not-log",
];

async function main() {
  const port = await reservePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const tempRoot = await mkdtemp(join(tmpdir(), "sponzey-cabinet-self-host-e2e-"));
  const server = startServer(port, tempRoot);
  let token = "";

  try {
    await waitForServer(baseUrl, server);

    await step("unauthorized_error", async () => {
      const unauthorized = await request(baseUrl, "GET", "/api/users", {
        expectedStatus: 401,
      });
      assertEqual(unauthorized.errorCode, "SESSION_EXPIRED", "missing_session_error_code");
    });

    await step("login_session_users", async () => {
      const login = await request(baseUrl, "POST", "/api/auth/login", {
        expectedStatus: 200,
        body: {
          login: "actor-a",
          credential: "e2e-password-should-not-log",
        },
      });
      assertEqual(login.userId, "actor-a", "login_user");
      assertEqual(login.sessionStatus, "active", "login_session_status");
      token = login.token;
      assertTruthy(token, "login_token_present");

      const session = await request(baseUrl, "POST", "/api/auth/session/validate", {
        expectedStatus: 200,
        token,
        body: { token },
      });
      assertEqual(session.sessionStatus, "active", "session_active");

      const users = await request(baseUrl, "GET", "/api/users", {
        expectedStatus: 200,
        token,
      });
      assertTruthy(users.users.length >= 2, "users_listed");
    });

    await step("groups_roles", async () => {
      const groups = await request(baseUrl, "GET", "/api/workspaces/workspace-1/groups", {
        expectedStatus: 200,
        token,
      });
      assertTruthy(groups.groups.some((group) => group.groupId === "editors"), "editors_group_listed");

      const added = await request(
        baseUrl,
        "POST",
        "/api/workspaces/workspace-1/groups/editors/members",
        {
          expectedStatus: 200,
          token,
          body: { userId: "actor-b" },
        },
      );
      assertTruthy(added.memberUserIds.includes("actor-b"), "group_member_added");

      const removed = await request(
        baseUrl,
        "DELETE",
        "/api/workspaces/workspace-1/groups/editors/members/actor-b",
        {
          expectedStatus: 200,
          token,
        },
      );
      assertTruthy(!removed.memberUserIds.includes("actor-b"), "group_member_removed");

      const roles = await request(baseUrl, "GET", "/api/workspaces/workspace-1/roles", {
        expectedStatus: 200,
        token,
      });
      assertTruthy(roles.assignments.length >= 1, "role_assignments_listed");

      const assigned = await request(baseUrl, "POST", "/api/workspaces/workspace-1/roles", {
        expectedStatus: 200,
        token,
        body: { subjectId: "actor-b", role: "editor" },
      });
      assertEqual(assigned.assignmentId, "role-assignment-2", "role_assigned");

      const revoked = await request(
        baseUrl,
        "DELETE",
        "/api/workspaces/workspace-1/roles/role-assignment-2",
        {
          expectedStatus: 200,
          token,
        },
      );
      assertEqual(revoked.revoked, true, "role_revoked");
    });

    await step("document_permissions_search_sharing", async () => {
      const allowed = await request(
        baseUrl,
        "GET",
        "/api/workspaces/workspace-1/documents/doc-allowed/current",
        {
          expectedStatus: 200,
          token,
        },
      );
      assertEqual(allowed.permissionDecision.effect, "allow", "allowed_document_visible");

      const denied = await request(
        baseUrl,
        "GET",
        "/api/workspaces/workspace-1/documents/doc-denied/current",
        {
          expectedStatus: 403,
          token,
        },
      );
      assertEqual(denied.errorCode, "DOCUMENT_ACCESS_DENIED", "denied_document_hidden");

      const search = await request(baseUrl, "GET", "/api/workspaces/workspace-1/search?text=needle&limit=10", {
        expectedStatus: 200,
        token,
      });
      assertTruthy(search.items.length === 1, "permission_aware_search_returned");
      assertTruthy(search.performance.observedMs <= 300, "search_under_300ms_target");

      const graph = await request(baseUrl, "GET", "/api/workspaces/workspace-1/documents/doc-allowed/graph", {
        expectedStatus: 200,
        token,
      });
      assertEqual(graph.centerDocumentId, "doc-allowed", "graph_center_document");
      assertEqual(graph.status, "clean", "graph_projection_clean");
      assertTruthy(graph.nodes.some((node) => node.id === "doc-visible"), "graph_visible_node_returned");
      assertTruthy(!graph.nodes.some((node) => node.id === "doc-hidden"), "graph_hidden_node_filtered");
      assertEqual(graph.stats.candidateCount, 3, "graph_candidate_count");
      assertEqual(graph.stats.filteredCount, 1, "graph_filtered_count");
      assertTruthy(graph.performance.observedMs <= 300, "graph_under_300ms_target");

      const sharing = await request(baseUrl, "GET", "/api/documents/doc-allowed/sharing?workspaceId=workspace-1", {
        expectedStatus: 200,
        token,
      });
      assertTruthy(sharing.entries.length >= 1, "sharing_listed");

      const updatedSharing = await request(baseUrl, "PUT", "/api/documents/doc-allowed/sharing", {
        expectedStatus: 200,
        token,
        body: {
          workspaceId: "workspace-1",
          entries: [
            {
              subject: { subjectId: "actor-b", subjectType: "user" },
              permission: "comment",
              effect: "allow",
            },
          ],
        },
      });
      assertEqual(updatedSharing.entries[0].permission, "comment", "sharing_updated");
    });

    await step("comments_review_publish", async () => {
      const comments = await request(baseUrl, "GET", "/api/documents/doc-allowed/comments?workspaceId=workspace-1", {
        expectedStatus: 200,
        token,
      });
      assertTruthy(comments.threads.length >= 1, "comments_listed");

      const addedComment = await request(baseUrl, "POST", "/api/documents/doc-allowed/comments", {
        expectedStatus: 200,
        token,
        body: {
          workspaceId: "workspace-1",
          authorUserId: "actor-b",
          body: "comment body should not leak",
        },
      });
      assertEqual(addedComment.state, "open", "comment_added");

      const inlineComment = await request(baseUrl, "POST", "/api/documents/doc-allowed/inline-comments", {
        expectedStatus: 200,
        token,
        body: {
          workspaceId: "workspace-1",
          authorUserId: "actor-b",
          body: "comment body should not leak",
          anchor: { documentVersionId: "version-3", startOffset: 1, endOffset: 4 },
        },
      });
      assertEqual(inlineComment.anchor.status, "valid", "inline_anchor_valid");

      const resolved = await request(baseUrl, "POST", "/api/comments/comment-thread-1/resolve", {
        expectedStatus: 200,
        token,
        body: { workspaceId: "workspace-1", documentId: "doc-allowed" },
      });
      assertEqual(resolved.nextState, "resolved", "comment_resolved");

      const reopened = await request(baseUrl, "POST", "/api/comments/comment-thread-1/reopen", {
        expectedStatus: 200,
        token,
        body: { workspaceId: "workspace-1", documentId: "doc-allowed" },
      });
      assertEqual(reopened.nextState, "reopened", "comment_reopened");

      const reviewList = await request(
        baseUrl,
        "GET",
        "/api/review-requests?workspaceId=workspace-1&documentId=doc-allowed",
        {
          expectedStatus: 200,
          token,
        },
      );
      assertTruthy(Array.isArray(reviewList.requests), "review_requests_listed");

      const requested = await request(baseUrl, "POST", "/api/documents/doc-allowed/review-requests", {
        expectedStatus: 200,
        token,
        body: { workspaceId: "workspace-1", reviewerUserId: "reviewer-1" },
      });
      assertEqual(requested.nextState, "InReview", "review_requested");

      const approved = await request(baseUrl, "POST", "/api/review-requests/review-request-1/approve", {
        expectedStatus: 200,
        token,
        body: { workspaceId: "workspace-1", approverUserId: "reviewer-1" },
      });
      assertEqual(approved.nextState, "Approved", "review_approved");

      const rejected = await request(baseUrl, "POST", "/api/review-requests/review-request-2/reject", {
        expectedStatus: 200,
        token,
        body: { workspaceId: "workspace-1", reviewerUserId: "reviewer-1" },
      });
      assertEqual(rejected.nextState, "Rejected", "review_rejected");

      const published = await request(baseUrl, "POST", "/api/documents/doc-allowed/publish", {
        expectedStatus: 200,
        token,
        body: { workspaceId: "workspace-1" },
      });
      assertEqual(published.nextState, "Published", "document_published");

      const publishDenied = await request(baseUrl, "POST", "/api/documents/doc-denied/publish", {
        expectedStatus: 403,
        token,
        body: { workspaceId: "workspace-1" },
      });
      assertEqual(publishDenied.errorCode, "PUBLISH_DENIED", "publish_denied_error");
    });

    await step("locks_audit_field_debug_backup_export", async () => {
      const initialLock = await request(baseUrl, "GET", "/api/documents/doc-allowed/locks/current?workspaceId=workspace-1", {
        expectedStatus: 200,
        token,
      });
      assertEqual(initialLock.status, "unlocked", "initial_lock_unlocked");

      const locked = await request(baseUrl, "POST", "/api/documents/doc-allowed/locks", {
        expectedStatus: 200,
        token,
        body: { workspaceId: "workspace-1", ownerUserId: "actor-a" },
      });
      assertEqual(locked.status, "locked", "document_locked");

      const conflict = await request(baseUrl, "POST", "/api/documents/doc-allowed/locks", {
        expectedStatus: 409,
        token,
        body: { workspaceId: "workspace-1", ownerUserId: "actor-b" },
      });
      assertEqual(conflict.errorCode, "DOCUMENT_LOCK_CONFLICT", "lock_conflict_error");

      const unlocked = await request(
        baseUrl,
        "DELETE",
        "/api/documents/doc-allowed/locks/current?workspaceId=workspace-1",
        {
          expectedStatus: 200,
          token,
        },
      );
      assertEqual(unlocked.status, "unlocked", "document_unlocked");

      const requestedDebug = await request(baseUrl, "POST", "/api/field-debug-sessions", {
        expectedStatus: 200,
        token,
        body: { workspaceId: "workspace-1", scope: "workspace", ttlSeconds: 60 },
      });
      assertEqual(requestedDebug.state, "Requested", "field_debug_requested");

      const approvedDebug = await request(
        baseUrl,
        "POST",
        "/api/field-debug-sessions/field-debug-session-1/approve",
        {
          expectedStatus: 200,
          token,
          body: { adminUserId: "actor-a" },
        },
      );
      assertEqual(approvedDebug.nextState, "Active", "field_debug_active");

      const expiredDebug = await request(
        baseUrl,
        "POST",
        "/api/field-debug-sessions/field-debug-session-1/expire",
        {
          expectedStatus: 200,
          token,
        },
      );
      assertEqual(expiredDebug.nextState, "Expired", "field_debug_expired");

      const backup = await request(baseUrl, "POST", "/api/backups", {
        expectedStatus: 200,
        token,
        body: { workspaceId: "workspace-1", assetFixture: "asset-content-should-not-log" },
      });
      assertEqual(backup.state, "Queued", "backup_queued");

      const backupStatus = await request(baseUrl, "GET", "/api/backups/backup-job-1?workspaceId=workspace-1", {
        expectedStatus: 200,
        token,
      });
      assertEqual(backupStatus.state, "Completed", "backup_completed");

      const restore = await request(baseUrl, "POST", "/api/backups/backup-job-1/restore", {
        expectedStatus: 200,
        token,
        body: { workspaceId: "workspace-1", restoreJobId: "restore-job-1" },
      });
      assertEqual(restore.state, "Completed", "restore_completed");

      const exportJob = await request(baseUrl, "POST", "/api/exports", {
        expectedStatus: 200,
        token,
        body: { workspaceId: "workspace-1" },
      });
      assertEqual(exportJob.state, "Queued", "export_queued");

      const exportStatus = await request(baseUrl, "GET", "/api/exports/export-job-1?workspaceId=workspace-1", {
        expectedStatus: 200,
        token,
      });
      assertEqual(exportStatus.state, "Completed", "export_completed");

      const auditEvents = await request(baseUrl, "GET", "/api/audit-events?workspaceId=workspace-1&scope=workspace&limit=50", {
        expectedStatus: 200,
        token,
      });
      assertTruthy(auditEvents.events.length >= 2, "audit_events_listed");
    });

    await step("product_log_sensitive_exclusion", async () => {
      const output = server.output();
      assertOutputContains(output, "product_log_event=server.started", "server_started_product_log");
      assertOutputContains(output, "product_log_event=graph.query.completed", "graph_query_product_log");
      assertOutputContains(output, "product_log_event=field_debug.approved", "field_debug_product_log");
      assertOutputContains(output, "product_log_event=backup.created", "backup_product_log");
      assertSensitiveOutputClean(output);
    });

    console.log("self_host_e2e_smoke=passed");
  } catch (error) {
    console.error("self_host_e2e_smoke=failed");
    console.error(`failure_category=${error instanceof SmokeAssertionError ? error.category : "unexpected_failure"}`);
    process.exitCode = 1;
  } finally {
    await stopServer(baseUrl, server);
    await writeServerOutputArtifact(server.output());
    await rm(tempRoot, { recursive: true, force: true });
    console.log("self_host_e2e_child_cleanup=completed");
  }
}

function startServer(port, tempRoot) {
  const stdout = [];
  const stderr = [];
  const child = spawn("sh", ["scripts/run_self_host_server.sh", "--e2e-http-server"], {
    cwd: root,
    env: {
      ...process.env,
      SPONZEY_CABINET_SERVER_BIND_ADDRESS: `127.0.0.1:${port}`,
      SPONZEY_CABINET_SERVER_PUBLIC_URL: `http://127.0.0.1:${port}`,
      SPONZEY_CABINET_SERVER_METADATA_STORE_LOCATION: join(tempRoot, "metadata.sqlite3"),
      SPONZEY_CABINET_SERVER_OBJECT_STORAGE_BACKEND: "local-disk",
      SPONZEY_CABINET_SERVER_OBJECT_STORAGE_LOCATION: join(tempRoot, "object-store"),
      SPONZEY_CABINET_SERVER_BACKUP_STORE_LOCATION: join(tempRoot, "backups"),
      SPONZEY_CABINET_AUTH_TOKEN_SECRET: "phase002-self-host-e2e-token-secret",
      SPONZEY_CABINET_AUTH_TOKEN_BYTE_LENGTH: "32",
      SPONZEY_CABINET_SERVER_PRODUCT_LOG_SINK: "stdout",
      SPONZEY_CABINET_SERVER_DEVELOPMENT_LOG_MODE: "disabled",
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  child.stdout.on("data", (chunk) => stdout.push(chunk.toString("utf8")));
  child.stderr.on("data", (chunk) => stderr.push(chunk.toString("utf8")));

  return {
    child,
    output() {
      return `${stdout.join("")}${stderr.join("")}`;
    },
  };
}

async function waitForServer(baseUrl, server) {
  const deadline = Date.now() + 45_000;
  while (Date.now() < deadline) {
    if (server.child.exitCode !== null) {
      throw new SmokeAssertionError("server_start_failed");
    }
    try {
      const response = await fetch(`${baseUrl}/api/health`);
      if (response.status === 200) {
        await response.arrayBuffer();
        return;
      }
    } catch {
      await sleep(200);
    }
  }
  throw new SmokeAssertionError("server_start_timeout");
}

async function stopServer(baseUrl, server) {
  if (server.child.exitCode !== null) {
    return;
  }
  try {
    await fetch(`${baseUrl}/__shutdown`, { method: "POST" });
  } catch {
    // The process may already be exiting after a startup or test failure.
  }
  const stopped = await waitForChildExit(server.child, 5_000);
  if (!stopped) {
    server.child.kill("SIGTERM");
    await waitForChildExit(server.child, 5_000);
  }
  assertSensitiveOutputClean(server.output());
}

async function writeServerOutputArtifact(output) {
  assertSensitiveOutputClean(output);
  await mkdir(join(root, ".tmp"), { recursive: true });
  await writeFile(serverOutputArtifactPath, output);
}

async function request(baseUrl, method, path, options = {}) {
  const headers = {};
  if (options.body !== undefined) {
    headers["content-type"] = "application/json";
  }
  if (options.token) {
    headers.authorization = `Bearer ${options.token}`;
  }
  const response = await fetch(`${baseUrl}${path}`, {
    method,
    headers,
    body: options.body === undefined ? undefined : JSON.stringify(options.body),
  });
  const text = await response.text();
  if (response.status !== options.expectedStatus) {
    throw new SmokeAssertionError(`http_status_${method}_${sanitizePath(path)}`);
  }
  try {
    return text.length === 0 ? {} : JSON.parse(text);
  } catch {
    throw new SmokeAssertionError(`invalid_json_${method}_${sanitizePath(path)}`);
  }
}

async function step(id, run) {
  console.log(`self_host_e2e_step_start=${id}`);
  await run();
  console.log(`self_host_e2e_step_passed=${id}`);
}

function assertTruthy(value, category) {
  if (!value) {
    throw new SmokeAssertionError(category);
  }
}

function assertEqual(actual, expected, category) {
  if (actual !== expected) {
    throw new SmokeAssertionError(category);
  }
}

function assertOutputContains(output, expected, category) {
  if (!output.includes(expected)) {
    throw new SmokeAssertionError(category);
  }
}

function assertSensitiveOutputClean(output) {
  for (const fixture of sensitiveFixtures) {
    if (output.includes(fixture)) {
      throw new SmokeAssertionError("sensitive_output_detected");
    }
  }
}

function sanitizePath(path) {
  return path.replace(/[^a-zA-Z0-9]+/g, "_").replace(/^_+|_+$/g, "");
}

async function reservePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close(() => resolve(port));
    });
  });
}

async function waitForChildExit(child, timeoutMs) {
  if (child.exitCode !== null) {
    return true;
  }
  return new Promise((resolve) => {
    const timeout = setTimeout(() => resolve(false), timeoutMs);
    child.once("exit", () => {
      clearTimeout(timeout);
      resolve(true);
    });
  });
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

class SmokeAssertionError extends Error {
  constructor(category) {
    super(category);
    this.category = category;
  }
}

main();
