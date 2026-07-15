import assert from "node:assert/strict";
import test from "node:test";

import {
  createConnectorAdminViewModel,
  type ConnectorDefinitionView,
  type ConnectorInstallationView,
} from "../src/index.ts";

test("connector admin model maps definitions and installed states without provider payloads", () => {
  const model = createConnectorAdminViewModel({
    definitions: definitions(),
    installations: [
      {
        installationId: "installation-1",
        workspaceId: "workspace-hash-1",
        connectorId: "connector-jira",
        state: "Installed",
        scopes: ["read", "write"],
      },
    ],
  });

  assert.equal(model.cards.length, 3);
  assert.equal(model.cards.find((card) => card.connectorId === "connector-teams")?.supportsWrite, false);
  assert.equal(model.cards.find((card) => card.connectorId === "connector-jira")?.supportsWrite, true);
  assert.equal(model.cards.find((card) => card.connectorId === "connector-jira")?.installationState, "Installed");
  assert.equal(JSON.stringify(model).includes("connector_access_token_fixture"), false);
  assert.equal(JSON.stringify(model).includes("connector_refresh_token_fixture"), false);
  assert.equal(JSON.stringify(model).includes("connector_client_secret_fixture"), false);
  assert.equal(JSON.stringify(model).includes("connector_payload"), false);
  assert.equal(JSON.stringify(model).includes("credential"), false);
});

test("connector admin model exposes stable scope labels", () => {
  const model = createConnectorAdminViewModel({
    definitions: definitions(),
    installations: [],
  });

  assert.deepEqual(
    model.cards.map((card) => [card.connectorId, card.scopeLabel]),
    [
      ["connector-slack", "Read and write"],
      ["connector-teams", "Read only"],
      ["connector-jira", "Read and write"],
    ],
  );
});

function definitions(): ConnectorDefinitionView[] {
  return [
    {
      connectorId: "connector-slack",
      kind: "Slack",
      displayName: "Slack",
      scopes: ["read", "write"],
    },
    {
      connectorId: "connector-teams",
      kind: "Teams",
      displayName: "Microsoft Teams",
      scopes: ["read"],
    },
    {
      connectorId: "connector-jira",
      kind: "Jira",
      displayName: "Jira",
      scopes: ["read", "write"],
    },
  ];
}

const _typeCheckInstallation: ConnectorInstallationView = {
  installationId: "installation-typecheck",
  workspaceId: "workspace-hash-1",
  connectorId: "connector-slack",
  state: "Installed",
  scopes: ["read"],
};
