import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  CollaborationCoverageAuditErrorCode,
  CollaborationCoverageAuditEvent,
  CollaborationCoverageAuditState,
  analyzeCollaborationCoverageSources,
  renderCollaborationCoverageAuditMarkdown,
  transitionCollaborationCoverageAuditState,
} from "./phase004_collaboration_coverage_audit.mjs";

const completeSources = {
  "crates/cabinet-domain/src/collaboration.rs":
    "DocumentOperation EditSessionState transition_edit_session_state detect_collaboration_conflict Presence",
  "crates/cabinet-domain/tests/collaboration_tests.rs":
    "stale_base_revision_is_reported_as_collaboration_conflict presence_is_validated_without_document_body_or_selection_text edit_session_state_machine_handles_sync_conflict_and_resolve",
  "crates/cabinet-usecases/src/collaboration.rs":
    "StartEditSessionUsecase ApplyCollaborativeEditUsecase UpdatePresenceUsecase collaboration.operation.accepted collaboration.conflict.detected collaboration.session.started",
  "crates/cabinet-usecases/tests/collaboration_usecase_tests.rs":
    "start_edit_session_requires_write_permission_and_saves_session unauthorized_edit_operation_is_rejected_without_event_append stale_base_revision_returns_conflict_without_event_append presence_update_is_saved_without_durable_operation_append",
  "crates/cabinet-ports/src/collaboration.rs":
    "CollaborationSessionStore CollaborationEventLog CollaborationOperationEvent",
  "crates/cabinet-ports/tests/collaboration_contract_tests.rs":
    "collaboration_session_store_preserves_session_state_and_presence_separately collaboration_event_log_appends_operations_without_presence_updates",
  "crates/cabinet-domain/src/realtime.rs":
    "RealtimeConnectionState RealtimeConnectionEvent transition_realtime_connection_state ConflictDetected ReplayingLocalChanges",
  "crates/cabinet-domain/tests/realtime_tests.rs":
    "realtime_connection_transition_supports_join_sync_conflict_and_replay_flow",
  "crates/cabinet-ports/src/realtime.rs":
    "DocumentRoomOwnerPolicy RealtimeTransport JoinDocumentRoomRequest OperationBroadcastRequest PresenceBroadcastRequest ReplayLocalChangesRequest",
  "crates/cabinet-ports/tests/realtime_contract_tests.rs":
    "room_owner_policy_maps_same_room_to_same_owner_without_global_singleton realtime_transport_contract_keeps_join_operation_presence_and_replay_separate",
  "crates/cabinet-adapters/src/local_realtime.rs":
    "LocalDocumentRoomOwnerPolicy LocalRealtimeTransport RoomNotJoined",
  "crates/cabinet-adapters/tests/local_realtime_adapter_tests.rs":
    "local_room_owner_policy_uses_explicit_namespace_without_global_state local_realtime_transport_records_join_operation_presence_and_replay_separately local_realtime_transport_rejects_unjoined_room_broadcasts_with_stable_error",
  "crates/cabinet-server/src/composition.rs":
    "collaboration.join_document_room collaboration.broadcast_operation collaboration.broadcast_presence collaboration.request_replay",
  "crates/cabinet-server/src/runtime.rs":
    "LocalDocumentRoomOwnerPolicy LocalRealtimeTransport collaboration.join_document_room collaboration.broadcast_operation collaboration.broadcast_presence collaboration.request_replay",
  "crates/cabinet-server/src/collaboration_realtime.rs":
    "command_from_input execute_realtime_command CollaborationRealtimeRuntimeTarget SplitRealtimeServerTarget is_collaboration_realtime_route",
  "crates/cabinet-server/tests/collaboration_realtime_command_mapper_tests.rs":
    "command_mapper_maps_join_operation_and_replay_without_framework_request command_mapper_strips_sensitive_presence_body_fields acknowledgement_response_mapper_excludes_raw_document_and_operation_text",
  "crates/cabinet-server/tests/collaboration_realtime_executor_tests.rs":
    "executor_starts_session_before_transport_join executor_appends_operation_before_transport_broadcast executor_rejects_conflict_without_transport_broadcast executor_updates_presence_without_event_log_and_requests_replay_through_transport",
  "crates/cabinet-server/tests/collaboration_realtime_runtime_target_tests.rs":
    "realtime_runtime_target_executes_join_route_through_handle_request realtime_runtime_target_executes_operation_route_through_handle_request realtime_runtime_target_returns_stable_rejected_response_for_invalid_body",
  "crates/cabinet-server/tests/split_realtime_server_target_tests.rs":
    "split_target_sends_collaboration_realtime_routes_to_realtime_target collaboration_realtime_route_helper_is_explicit",
  "crates/cabinet-server/tests/server_runtime_wiring_tests.rs":
    "composition_wires_every_route_id_to_a_runtime_handler collaboration.join_document_room",
  "crates/cabinet-server/tests/server_dependency_manifest_tests.rs":
    "realtime_room_owner_policy LocalDocumentRoomOwnerPolicy realtime_transport LocalRealtimeTransport",
  "packages/editor/src/index.ts":
    "createCollaborativeEditInputFromEditorTransaction createPresenceInputFromEditorSelection EditorCollaborationAdapterError selectedText documentBody token",
  "packages/editor/tests/collaboration_adapter_tests.ts":
    "editor collaboration adapter maps a single text change to plain edit DTO editor presence adapter excludes selected text body and token fields editor collaboration adapter does not import CodeMirror runtime types",
  "packages/client-core/src/index.ts":
    "CollaborationRealtimeClient createCollaborationRealtimeClient CabinetRealtimeClientError selectedText documentBody token",
  "packages/client-core/tests/realtime_client_tests.ts":
    "collaboration realtime client sends join operation presence and replay as plain DTOs collaboration realtime client strips sensitive presence draft fields collaboration realtime client does not import runtime transport or editor types",
  "scripts/run_phase004_realtime_collaboration_smoke.sh":
    "phase004_realtime_collaboration_smoke=passed server_split_dispatch=passed server_runtime_target=passed",
  "scripts/run_phase004_realtime_collaboration_smoke_tests.mjs":
    "phase004 realtime collaboration smoke runner executes required test commands phase004_realtime_collaboration_smoke=passed",
  ".tasks/realtime-collaboration-smoke-result.md":
    "phase004_realtime_collaboration_smoke=passed server_split_dispatch=passed server_runtime_target=passed",
  ".tasks/release/security-log-policy-manifest.json":
    "phase004_realtime_collaboration_smoke_result realtime_operation_text_fixture realtime_selection_text_fixture operation_text selection_text codemirror_transaction_dump",
  "scripts/security_log_scanner_tests.mjs":
    "active security manifest includes Phase 004 realtime collaboration artifacts and denied fixtures",
  "package.json":
    "run:phase004-realtime-collaboration-smoke run:security-log-scanner",
};

test("collaboration coverage audit marks complete fixture as covered", () => {
  const audit = analyzeCollaborationCoverageSources({ sources: completeSources });

  assert.equal(audit.summary.totalTargets, 5);
  assert.equal(audit.summary.covered, 5);
  assert.equal(audit.summary.targetsNeedingWork, 0);
  assert.equal(audit.nextImplementationTarget, null);
});

test("collaboration coverage audit selects server runtime target when split dispatch evidence is missing", () => {
  const {
    "crates/cabinet-server/tests/split_realtime_server_target_tests.rs": _splitTarget,
    ...sources
  } = completeSources;

  const audit = analyzeCollaborationCoverageSources({ sources });
  const target = audit.targets.find((entry) => entry.id === "collaboration_server_runtime_boundary");

  assert.equal(target.status, "missing");
  assert.equal(audit.nextImplementationTarget.id, "collaboration_server_runtime_boundary");
});

test("collaboration coverage audit selects smoke security target when scanner fixture is missing", () => {
  const sources = {
    ...completeSources,
    ".tasks/release/security-log-policy-manifest.json":
      "phase004_realtime_collaboration_smoke_result",
  };

  const audit = analyzeCollaborationCoverageSources({ sources });
  const target = audit.targets.find((entry) => entry.id === "collaboration_smoke_security_evidence");

  assert.equal(target.status, "missing");
  assert.deepEqual(target.missingEvidence, [
    "realtime_operation_text_fixture",
    "realtime_selection_text_fixture",
    "operation_text",
    "selection_text",
    "codemirror_transaction_dump",
  ]);
});

test("collaboration coverage audit fails with stable code when source set is empty", () => {
  assert.throws(
    () => analyzeCollaborationCoverageSources({ sources: {} }),
    (error) => error.code === CollaborationCoverageAuditErrorCode.SourceSetEmpty,
  );
});

test("collaboration coverage audit state machine rejects invalid transitions", () => {
  assert.equal(
    transitionCollaborationCoverageAuditState(
      CollaborationCoverageAuditState.NotStarted,
      CollaborationCoverageAuditEvent.Start,
    ),
    CollaborationCoverageAuditState.ReadingSource,
  );
  assert.equal(
    transitionCollaborationCoverageAuditState(
      CollaborationCoverageAuditState.ReadingSource,
      CollaborationCoverageAuditEvent.SourceLoaded,
    ),
    CollaborationCoverageAuditState.Auditing,
  );
  assert.equal(
    transitionCollaborationCoverageAuditState(
      CollaborationCoverageAuditState.Auditing,
      CollaborationCoverageAuditEvent.AuditComplete,
    ),
    CollaborationCoverageAuditState.Reported,
  );
  assert.throws(
    () =>
      transitionCollaborationCoverageAuditState(
        CollaborationCoverageAuditState.NotStarted,
        CollaborationCoverageAuditEvent.ReportWritten,
      ),
    (error) => error.code === CollaborationCoverageAuditErrorCode.InvalidTransition,
  );
});

test("collaboration coverage markdown records phase targets and next action", () => {
  const {
    "packages/client-core/tests/realtime_client_tests.ts": _clientTests,
    ...sources
  } = completeSources;
  const audit = analyzeCollaborationCoverageSources({ sources });
  const markdown = renderCollaborationCoverageAuditMarkdown(audit);

  assert.match(markdown, /# Phase 004 Collaboration Coverage Audit/);
  assert.match(markdown, /Phase 004\.4/);
  assert.match(markdown, /collaboration_client_editor_contract/);
  assert.match(markdown, /missing/);
});

test("package scripts expose collaboration coverage audit runners", async () => {
  const packageJson = JSON.parse(await readFile("package.json", "utf8"));
  const testRunner = await readFile(
    "scripts/run_phase004_collaboration_coverage_audit_tests.sh",
    "utf8",
  );
  const auditRunner = await readFile("scripts/run_phase004_collaboration_coverage_audit.sh", "utf8");

  assert.equal(
    packageJson.scripts["run:phase004-collaboration-coverage-audit-tests"],
    "sh scripts/run_phase004_collaboration_coverage_audit_tests.sh",
  );
  assert.equal(
    packageJson.scripts["run:phase004-collaboration-coverage-audit"],
    "sh scripts/run_phase004_collaboration_coverage_audit.sh",
  );
  assert.match(testRunner, /node --test scripts\/phase004_collaboration_coverage_audit_tests\.mjs/);
  assert.match(auditRunner, /node scripts\/phase004_collaboration_coverage_audit\.mjs/);
});
