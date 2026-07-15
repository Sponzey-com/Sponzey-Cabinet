import assert from "node:assert/strict";
import test from "node:test";

import {
  createMobilePushNotificationPayload,
  transitionMobileNotificationDeliveryState,
} from "../src/index.ts";

test("mobile push payload excludes sensitive document comment token and canvas data", () => {
  const payload = createMobilePushNotificationPayload({
    eventName: "review.state_changed",
    target: {
      kind: "review_request",
      id: "review-1",
    },
    title: "Review state changed",
    correlationId: "corr-1",
    deliveryState: "Queued",
    unsafeDocumentBody: "local raw document body",
    unsafeCommentBody: "local raw comment body",
    unsafeSessionToken: "local-session-token",
    unsafeSessionId: "local-session-id",
    unsafeRawCanvasState: "{\"nodes\":[{\"text\":\"secret\"}]}",
  });

  assert.deepEqual(payload, {
    eventName: "review.state_changed",
    targetKind: "review_request",
    targetId: "review-1",
    title: "Review state changed",
    correlationId: "corr-1",
    deliveryState: "Queued",
  });
  assert.doesNotMatch(
    JSON.stringify(payload),
    /local raw document body|local raw comment body|local-session-token|local-session-id|secret/i,
  );
});

test("mobile notification delivery state machine exposes queued sent failed and retry transitions", () => {
  assert.deepEqual(transitionMobileNotificationDeliveryState("Queued", "SendSucceeded"), {
    state: "Sent",
  });
  assert.deepEqual(transitionMobileNotificationDeliveryState("Queued", "SendFailed"), {
    state: "Failed",
  });
  assert.deepEqual(transitionMobileNotificationDeliveryState("Failed", "RetryRequested"), {
    state: "Retry",
  });
  assert.deepEqual(transitionMobileNotificationDeliveryState("Retry", "RetryScheduled"), {
    state: "Queued",
  });
  assert.deepEqual(transitionMobileNotificationDeliveryState("Failed", "GiveUp"), {
    state: "Failed",
  });
});

test("mobile notification delivery state machine rejects invalid transitions explicitly", () => {
  assert.deepEqual(transitionMobileNotificationDeliveryState("Sent", "RetryRequested"), {
    state: "Sent",
    errorCode: "MOBILE_NOTIFICATION_INVALID_TRANSITION",
  });
  assert.deepEqual(transitionMobileNotificationDeliveryState("Queued", "Enqueue"), {
    state: "Queued",
    errorCode: "MOBILE_NOTIFICATION_INVALID_TRANSITION",
  });
});
