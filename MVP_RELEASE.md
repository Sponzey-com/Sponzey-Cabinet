# Sponzey Cabinet MVP Release Notes

This document defines the current MVP release gate for the local single-user Knowledge Base Core.
It separates end-user local runtime expectations from developer validation requirements.

## MVP Scope

The MVP supports a local personal knowledge base flow:

- Create and update Markdown documents.
- Read the current document without scanning history.
- Read document history and specific versions through separate query paths.
- Preview and restore a previous version.
- Parse Markdown links, Wikilinks, and attachment references.
- Search documents through the local search index.
- View backlink projection through the link index.
- Attach local files as separate assets and list attachment metadata.
- Initialize local stores on first launch without manual setup.

The MVP excludes multi-user collaboration, realtime editing, SaaS tenancy, OAuth/SSO, plugin runtime,
AI answer generation, CRM objects, Canvas/Edgeless UI, iOS app implementation, and Android app
implementation.

## Local Data Location

The local app receives an app data directory once during bootstrap and derives all local paths from
that validated config object. Internal flows receive those paths through explicit configuration and
dependency injection. Runtime code must not reread or mutate environment values after startup.

Default local data layout:

```text
app_data_dir/
  metadata/
    first-run marker
    migration versions
    document-asset associations
  version-store/
    document version entries
    version snapshots
  assets/
    asset metadata
    asset object bytes
  search-index/
    local search index data
  workspaces/
    current document metadata and body snapshots
```

The local user must not install or configure an external DB, search server, Git CLI, Node.js runtime,
or manual config file for the default local flow. Any advanced location change must be an explicit
settings/import/export action, not a hidden runtime requirement.

Git, commit, branch, and repository concepts are not exposed to the user experience. Git provider
integration is not part of this MVP.

## Backup and Export Policy

Backup policy:

- Stop the local app before filesystem-level backup.
- Back up the entire `app_data_dir` as one unit.
- Treat `metadata`, `version-store`, `assets`, `search-index`, and `workspaces` as one consistency
  boundary.
- Do not back up document body without the version store and asset store if restore fidelity matters.

Export policy:

- Markdown export is the primary portable document export path.
- HTML export is available as a minimum rendering/export foundation.
- PDF export is represented by an extension boundary and test coverage, not a full production PDF
  pipeline.
- Attachment objects remain separate assets and must be exported with their metadata when a complete
  workspace export is required.

## Logging Policy

### Product Log

Product Log is the production-facing minimum log stream. It records user-impacting outcomes,
stable error codes, major state transitions, and operation success/failure metadata. It must not
record document body, attachment bytes, secrets, tokens, or full internal object dumps.

MVP smoke tests verify that release smoke product events do not include document body or attachment
bytes.

### Field Debug Log

Field Debug Log is reserved for scoped operational diagnosis. It must have an activation scope,
limited retention, and sensitive data masking before it can be used in customer or production
environments. It is not enabled by default in the MVP.

### Development Log

Development Log is for local development and test diagnosis only. It must not be included in
production default behavior. Development-only diagnostics remain outside the end-user local runtime
contract.

## State Machine Evidence

The release gate verifies stateful internal procedures through explicit state results:

- first-run reaches `Completed`.
- migration reaches `Completed` and rerun is idempotent.
- restore reaches `Completed`.

Complex flows must remain state-machine driven and testable. They must not be represented by hidden
flag combinations in UI, adapter, or infrastructure code.

## Performance and Reliability Evidence

The MVP release gate includes:

- p95 300ms benchmark coverage for current document lookup, history list lookup, specific version
  lookup, search lookup, link/backlink lookup, and asset metadata lookup.
- clean install smoke validating local first-run without external DB/search/Git CLI/Node.js/manual
  config.
- data preservation smoke validating current document, version history, specific version snapshot,
  asset metadata, asset object bytes, and migration idempotency after reinitialization.
- MVP end-to-end smoke validating create, edit, Wikilink parsing, attachment reference parsing,
  search, backlink projection, asset metadata listing, restore preview, restore, and current
  read-back.
- architecture boundary checks for layered and clean architecture rules.
- Git CLI absence check for local runtime behavior.

## Developer Release Gate

Run the MVP developer release gate from the repository root:

```sh
sh scripts/mvp_release_gate.sh
```

The developer gate runs Rust tests, formatting checks, architecture boundary checks, no-Git-CLI
checks, runtime config checks, first-run/migration/logging checks, domain checks, frontend boundary
checks, editor/UI smoke checks, platform adapter smoke, desktop shell boundary checks, and this
release documentation check.

Developer validation may require Rust, Node.js, and shell tooling. These are development
requirements only. They are not end-user local app runtime requirements.

## Known Limitations

- The MVP validates Web and desktop shell contracts, but it does not ship a full production UI
  automation gate.
- iOS and Android are official target platforms for the final product, but not implemented in this
  MVP.
- Multi-user collaboration, realtime editing, SaaS tenancy, OAuth/SSO, plugin runtime, AI answer
  generation, CRM objects, and Canvas/Edgeless UI are outside this MVP.
- Search and link indexes are treated as derived projections. The MVP release smoke verifies their
  update and query behavior in the local flow; long-term index rebuild and persistence policies need
  a later dedicated phase.
- PDF export is an extension boundary in the MVP, not a full production-grade PDF export pipeline.
