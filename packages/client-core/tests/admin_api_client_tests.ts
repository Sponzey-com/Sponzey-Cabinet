import assert from "node:assert/strict";
import test from "node:test";

import {
  CabinetApiClientError,
  createSelfHostApiClient,
  createSelfHostApiClientConfig,
  type CabinetHttpRequest,
  type CabinetHttpResponse,
  type CabinetHttpTransport,
} from "../src/index.ts";

test("self-host API client sends login and session requests through explicit base URL config", async () => {
  const transport = new CapturingTransport([
    jsonResponse(200, {
      userId: "user-admin",
      token: "session-token",
      sessionStatus: "active",
    }),
    jsonResponse(200, {
      userId: "user-admin",
      sessionStatus: "active",
    }),
  ]);
  const client = createSelfHostApiClient(
    createSelfHostApiClientConfig({ baseUrl: "https://cabinet.local/root/" }),
    transport.handle,
  );

  const login = await client.login({ login: "admin", credential: "password" });
  const session = await client.validateSession({ token: login.token });

  assert.equal(login.userId, "user-admin");
  assert.equal(session.userId, "user-admin");
  assert.deepEqual(
    transport.requests.map((request) => [request.method, request.url]),
    [
      ["POST", "https://cabinet.local/root/api/auth/login"],
      ["POST", "https://cabinet.local/root/api/auth/session/validate"],
    ],
  );
  assert.deepEqual(JSON.parse(transport.requests[0].body ?? "{}"), {
    login: "admin",
    credential: "password",
  });
  assert.equal(transport.requests[0].headers["content-type"], "application/json");
});

test("self-host API client exposes user, group, membership, and role admin API calls", async () => {
  const transport = new CapturingTransport([
    jsonResponse(200, {
      users: [
        {
          userId: "user-1",
          login: "owner",
          email: "owner@example.invalid",
          displayName: "Owner",
          status: "active",
        },
      ],
    }),
    jsonResponse(200, {
      groups: [
        {
          workspaceId: "workspace-1",
          groupId: "group-1",
          name: "Editors",
          memberUserIds: ["user-1"],
        },
      ],
    }),
    jsonResponse(200, {
      groupId: "group-1",
      userId: "user-2",
      result: "added",
    }),
    jsonResponse(200, {
      groupId: "group-1",
      userId: "user-2",
      result: "removed",
    }),
    jsonResponse(200, {
      assignments: [
        {
          assignmentId: "role-1",
          workspaceId: "workspace-1",
          subject: { kind: "user", id: "user-1" },
          role: "owner",
        },
      ],
    }),
    jsonResponse(200, {
      assignmentId: "role-2",
      workspaceId: "workspace-1",
      subject: { kind: "group", id: "group-1" },
      role: "editor",
    }),
    jsonResponse(200, {
      assignmentId: "role-2",
      result: "revoked",
    }),
  ]);
  const client = createSelfHostApiClient(
    createSelfHostApiClientConfig({ baseUrl: "https://cabinet.local" }),
    transport.handle,
  );

  const users = await client.listUsers();
  const groups = await client.listGroups({ workspaceId: "workspace-1" });
  const added = await client.addGroupMember({
    workspaceId: "workspace-1",
    groupId: "group-1",
    userId: "user-2",
  });
  const removed = await client.removeGroupMember({
    workspaceId: "workspace-1",
    groupId: "group-1",
    userId: "user-2",
  });
  const assignments = await client.listRoleAssignments({ workspaceId: "workspace-1" });
  const assigned = await client.assignWorkspaceRole({
    workspaceId: "workspace-1",
    subject: { kind: "group", id: "group-1" },
    role: "editor",
  });
  const revoked = await client.revokeWorkspaceRole({
    workspaceId: "workspace-1",
    assignmentId: "role-2",
  });

  assert.equal(users.users[0].status, "active");
  assert.equal(groups.groups[0].memberUserIds[0], "user-1");
  assert.equal(added.result, "added");
  assert.equal(removed.result, "removed");
  assert.equal(assignments.assignments[0].role, "owner");
  assert.equal(assigned.subject.kind, "group");
  assert.equal(revoked.result, "revoked");
  assert.deepEqual(
    transport.requests.map((request) => [request.method, request.url]),
    [
      ["GET", "https://cabinet.local/api/users"],
      ["GET", "https://cabinet.local/api/workspaces/workspace-1/groups"],
      ["POST", "https://cabinet.local/api/workspaces/workspace-1/groups/group-1/members"],
      ["DELETE", "https://cabinet.local/api/workspaces/workspace-1/groups/group-1/members/user-2"],
      ["GET", "https://cabinet.local/api/workspaces/workspace-1/roles"],
      ["POST", "https://cabinet.local/api/workspaces/workspace-1/roles"],
      ["DELETE", "https://cabinet.local/api/workspaces/workspace-1/roles/role-2"],
    ],
  );
});

test("self-host API client maps stable server and network errors without product logging", async () => {
  const serverFailure = new CapturingTransport([
    jsonResponse(401, {
      errorCode: "SESSION_EXPIRED",
      message: "session expired",
    }),
  ]);
  const serverClient = createSelfHostApiClient(
    createSelfHostApiClientConfig({ baseUrl: "https://cabinet.local" }),
    serverFailure.handle,
  );

  await assert.rejects(
    () => serverClient.listUsers(),
    (error) => error instanceof CabinetApiClientError && error.code === "SESSION_EXPIRED",
  );

  const networkClient = createSelfHostApiClient(
    createSelfHostApiClientConfig({ baseUrl: "https://cabinet.local" }),
    async () => {
      throw new Error("connection refused");
    },
  );

  await assert.rejects(
    () => networkClient.listUsers(),
    (error) => error instanceof CabinetApiClientError && error.code === "NETWORK_FAILURE",
  );
});

class CapturingTransport {
  readonly requests: CabinetHttpRequest[] = [];
  private responses: CabinetHttpResponse[];

  constructor(responses: CabinetHttpResponse[]) {
    this.responses = [...responses];
  }

  readonly handle: CabinetHttpTransport = async (request) => {
    this.requests.push(request);
    const response = this.responses.shift();
    if (!response) {
      throw new Error(`Unexpected request ${request.method} ${request.url}`);
    }
    return response;
  };
}

function jsonResponse(status: number, body: unknown): CabinetHttpResponse {
  return {
    status,
    body: JSON.stringify(body),
    headers: { "content-type": "application/json" },
  };
}
