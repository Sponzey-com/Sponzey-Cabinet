import assert from "node:assert/strict";
import test from "node:test";

import {
  beginDesktopRouteQuery,
  canApplyDesktopRouteQuery,
  createDesktopRouteQueryLifecycle,
  transitionDesktopRouteQueryLifecycle,
} from "../src/desktop_route_query_lifecycle.ts";

test("route query lifecycle accepts only the active route latest query", () => {
  const initial = createDesktopRouteQueryLifecycle("Home");
  const first = beginDesktopRouteQuery(initial, "Home");
  assert.ok(first.ticket);
  const second = beginDesktopRouteQuery(first.state, "Home");
  assert.ok(second.ticket);

  assert.equal(canApplyDesktopRouteQuery(second.state, first.ticket!), false);
  assert.equal(canApplyDesktopRouteQuery(second.state, second.ticket!), true);

  const document = transitionDesktopRouteQueryLifecycle(second.state, {
    type: "RouteActivated",
    route: "Document",
  });
  assert.equal(canApplyDesktopRouteQuery(document, second.ticket!), false);
  assert.equal(beginDesktopRouteQuery(document, "Home").ticket, undefined);
});

test("twenty rapid route transitions reject reverse stale completions", () => {
  let state = createDesktopRouteQueryLifecycle("Document");
  const tickets = [];
  for (let index = 0; index < 20; index += 1) {
    state = transitionDesktopRouteQueryLifecycle(state, { type: "RouteActivated", route: "Home" });
    const started = beginDesktopRouteQuery(state, "Home");
    state = started.state;
    assert.ok(started.ticket);
    tickets.push(started.ticket!);
    if (index < 19) {
      state = transitionDesktopRouteQueryLifecycle(state, { type: "RouteActivated", route: "Document" });
    }
  }

  const decisions = tickets.toReversed().map((ticket) => canApplyDesktopRouteQuery(state, ticket));
  assert.deepEqual(decisions, [true, ...Array.from({ length: 19 }, () => false)]);
});

test("route query lifecycle rejects invalid epochs", () => {
  assert.throws(() => createDesktopRouteQueryLifecycle("Unknown" as "Home"), /INVALID_ROUTE_QUERY_LIFECYCLE/);
  const state = createDesktopRouteQueryLifecycle("Home");
  assert.equal(canApplyDesktopRouteQuery(state, { route: "Home", epoch: -1 }), false);
});
