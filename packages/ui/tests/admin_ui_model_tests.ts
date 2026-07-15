import assert from "node:assert/strict";
import test from "node:test";

import type {
  AddGroupMemberCommand,
  AdminSessionView,
  CabinetAdminApiClient,
  GroupMemberMutationResultView,
  GroupPageView,
  ListGroupsQuery,
  ListRoleAssignmentsQuery,
  LoginCommand,
  RemoveGroupMemberCommand,
  RevokeRoleCommand,
  RevokeRoleResultView,
  RoleAssignmentCommand,
  RoleAssignmentPageView,
  RoleAssignmentView,
  UserPageView,
  ValidateSessionQuery,
} from "../../client-core/src/index.ts";
import {
  addAdminGroupMember,
  assignAdminWorkspaceRole,
  createAdminLoginFormModel,
  createInitialAdminViewModel,
  loadAdminWorkspaceViewModel,
  loginToSelfHostAdmin,
  mapApiClientErrorToAdminMessage,
  removeAdminGroupMember,
  revokeAdminWorkspaceRole,
  transitionAdminDisplayState,
  type AdminDevelopmentLogger,
} from "../src/index.ts";

test("admin login display state uses fake API client and never computes domain permission rules", async () => {
  const client = new FakeAdminApiClient();
  const developmentLogger = new CapturingDevelopmentLogger();
  const initial = createInitialAdminViewModel({
    serverBaseUrl: "https://cabinet.local",
    workspaceId: "workspace-1",
  });

  const authenticating = transitionAdminDisplayState(initial, { type: "login-submit" });
  const authenticated = await loginToSelfHostAdmin(
    authenticating,
    createAdminLoginFormModel("admin", "password"),
    client,
    developmentLogger,
  );

  assert.equal(initial.displayState, "Unauthenticated");
  assert.equal(authenticating.displayState, "Authenticating");
  assert.equal(authenticated.displayState, "Authenticated");
  assert.equal(authenticated.session?.userId, "user-admin");
  assert.deepEqual(client.calls, ["login"]);
  assert.deepEqual(developmentLogger.events, ["admin.login.submit", "admin.login.success"]);
  assert.equal("permissionRules" in authenticated, false);
});

test("admin workspace model loads users, groups, roles, and applies membership changes through API client", async () => {
  const client = new FakeAdminApiClient();
  const developmentLogger = new CapturingDevelopmentLogger();
  const authenticated = await loginToSelfHostAdmin(
    createInitialAdminViewModel({
      serverBaseUrl: "https://cabinet.local",
      workspaceId: "workspace-1",
    }),
    createAdminLoginFormModel("admin", "password"),
    client,
    developmentLogger,
  );

  const loaded = await loadAdminWorkspaceViewModel(authenticated, client, developmentLogger);
  const added = await addAdminGroupMember(loaded, "group-1", "user-editor", client, developmentLogger);
  const removed = await removeAdminGroupMember(
    added,
    "group-1",
    "user-editor",
    client,
    developmentLogger,
  );

  assert.equal(loaded.users.length, 2);
  assert.equal(loaded.groups.length, 1);
  assert.equal(loaded.roleAssignments.length, 1);
  assert.deepEqual(added.lastMembershipResult, {
    groupId: "group-1",
    userId: "user-editor",
    result: "added",
  });
  assert.deepEqual(removed.lastMembershipResult, {
    groupId: "group-1",
    userId: "user-editor",
    result: "removed",
  });
  assert.deepEqual(client.calls, ["login", "listUsers", "listGroups", "listRoleAssignments", "addGroupMember", "removeGroupMember"]);
});

test("admin role assignment and revocation UI actions delegate to API result instead of recalculating RBAC", async () => {
  const client = new FakeAdminApiClient();
  const authenticated = await loginToSelfHostAdmin(
    createInitialAdminViewModel({
      serverBaseUrl: "https://cabinet.local",
      workspaceId: "workspace-1",
    }),
    createAdminLoginFormModel("admin", "password"),
    client,
  );
  const loaded = await loadAdminWorkspaceViewModel(authenticated, client);

  const assigned = await assignAdminWorkspaceRole(
    loaded,
    { kind: "group", id: "group-1" },
    "editor",
    client,
  );
  const revoked = await revokeAdminWorkspaceRole(assigned, "role-new", client);

  assert.equal(assigned.lastRoleAssignment?.assignmentId, "role-new");
  assert.equal(assigned.lastRoleAssignment?.role, "editor");
  assert.equal(revoked.lastRoleRevocation?.result, "revoked");
  assert.equal(revoked.roleAssignments.some((assignment) => assignment.assignmentId === "role-new"), false);
});

test("admin UI maps API errors to stable display state messages", () => {
  assert.deepEqual(mapApiClientErrorToAdminMessage({ code: "UNAUTHORIZED" }), {
    code: "UNAUTHORIZED",
    message: "Sign in again to continue.",
    retryable: false,
  });
  assert.deepEqual(mapApiClientErrorToAdminMessage({ code: "SESSION_EXPIRED" }), {
    code: "SESSION_EXPIRED",
    message: "The session expired. Sign in again.",
    retryable: false,
  });
  assert.deepEqual(mapApiClientErrorToAdminMessage({ code: "NETWORK_FAILURE" }), {
    code: "NETWORK_FAILURE",
    message: "The server is unreachable. Check the self-host server address.",
    retryable: true,
  });
  assert.deepEqual(mapApiClientErrorToAdminMessage({ code: "VALIDATION_ERROR" }), {
    code: "VALIDATION_ERROR",
    message: "Review the submitted values and try again.",
    retryable: false,
  });
});

class FakeAdminApiClient implements CabinetAdminApiClient {
  readonly calls: string[] = [];

  async login(_command: LoginCommand): Promise<AdminSessionView> {
    this.calls.push("login");
    return {
      userId: "user-admin",
      token: "token",
      sessionStatus: "active",
    };
  }

  async validateSession(_query: ValidateSessionQuery): Promise<AdminSessionView> {
    this.calls.push("validateSession");
    return {
      userId: "user-admin",
      token: "token",
      sessionStatus: "active",
    };
  }

  async listUsers(): Promise<UserPageView> {
    this.calls.push("listUsers");
    return {
      users: [
        {
          userId: "user-admin",
          login: "admin",
          email: "admin@example.invalid",
          displayName: "Admin",
          status: "active",
        },
        {
          userId: "user-editor",
          login: "editor",
          email: "editor@example.invalid",
          displayName: "Editor",
          status: "active",
        },
      ],
    };
  }

  async listGroups(_query: ListGroupsQuery): Promise<GroupPageView> {
    this.calls.push("listGroups");
    return {
      groups: [
        {
          workspaceId: "workspace-1",
          groupId: "group-1",
          name: "Editors",
          memberUserIds: ["user-admin"],
        },
      ],
    };
  }

  async addGroupMember(command: AddGroupMemberCommand): Promise<GroupMemberMutationResultView> {
    this.calls.push("addGroupMember");
    return {
      groupId: command.groupId,
      userId: command.userId,
      result: "added",
    };
  }

  async removeGroupMember(command: RemoveGroupMemberCommand): Promise<GroupMemberMutationResultView> {
    this.calls.push("removeGroupMember");
    return {
      groupId: command.groupId,
      userId: command.userId,
      result: "removed",
    };
  }

  async listRoleAssignments(_query: ListRoleAssignmentsQuery): Promise<RoleAssignmentPageView> {
    this.calls.push("listRoleAssignments");
    return {
      assignments: [
        {
          assignmentId: "role-owner",
          workspaceId: "workspace-1",
          subject: { kind: "user", id: "user-admin" },
          role: "owner",
        },
      ],
    };
  }

  async assignWorkspaceRole(command: RoleAssignmentCommand): Promise<RoleAssignmentView> {
    this.calls.push("assignWorkspaceRole");
    return {
      assignmentId: "role-new",
      workspaceId: command.workspaceId,
      subject: command.subject,
      role: command.role,
    };
  }

  async revokeWorkspaceRole(command: RevokeRoleCommand): Promise<RevokeRoleResultView> {
    this.calls.push("revokeWorkspaceRole");
    return {
      assignmentId: command.assignmentId,
      result: "revoked",
    };
  }
}

class CapturingDevelopmentLogger implements AdminDevelopmentLogger {
  readonly events: string[] = [];

  writeDevelopment(eventName: string): void {
    this.events.push(eventName);
  }
}
