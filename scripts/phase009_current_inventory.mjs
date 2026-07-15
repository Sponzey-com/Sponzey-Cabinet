import { access, mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase009InventoryState = Object.freeze({
  NotStarted: "NotStarted",
  InspectingPaths: "InspectingPaths",
  RenderingArtifact: "RenderingArtifact",
  WritingArtifact: "WritingArtifact",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase009InventoryEvent = Object.freeze({
  Start: "Start",
  PathsInspected: "PathsInspected",
  ArtifactRendered: "ArtifactRendered",
  ArtifactWritten: "ArtifactWritten",
  Fail: "Fail",
});

export const Phase009InventoryErrorCode = Object.freeze({
  RequiredPathMissing: "PHASE009_REQUIRED_PATH_MISSING",
  Phase008ReleaseMarkerMissing: "PHASE009_PHASE008_RELEASE_MARKER_MISSING",
  PlanMissingRequiredScope: "PHASE009_PLAN_REQUIRED_SCOPE_MISSING",
  ShellSmokeMisclassified: "PHASE009_SHELL_SMOKE_MISCLASSIFIED",
  InventoryTextMissingTerm: "PHASE009_INVENTORY_TEXT_MISSING_TERM",
  IoFailed: "PHASE009_INVENTORY_IO_FAILED",
  InvalidTransition: "PHASE009_INVENTORY_INVALID_TRANSITION",
});

const requiredCurrentPaths = [
  {
    area: "Product UI Runner",
    path: "scripts/run_desktop_app.sh",
    responsibility: "Visible desktop app launcher for development.",
    boundaryRule: "May use script-level environment variables only at process boundary.",
  },
  {
    area: "Internal Shell Smoke",
    path: "scripts/run_desktop_shell.sh",
    responsibility: "Internal shell or command-boundary smoke; not the product UI launcher.",
    boundaryRule: "Do not document as user-facing desktop UI.",
  },
  {
    area: "Desktop UI Server",
    path: "scripts/run_web_app.mjs",
    responsibility: "Development static UI server for Tauri devUrl and browser preview.",
    boundaryRule: "Installed app must not depend on this dev server.",
  },
  {
    area: "Desktop Asset Build",
    path: "scripts/build_desktop_assets.mjs",
    responsibility: "Build desktop static assets.",
    boundaryRule: "Build tooling stays outside runtime domain/usecase layers.",
  },
  {
    area: "Desktop UI Composition",
    path: "apps/desktop/src/index.ts",
    responsibility: "Local desktop UI facade and command-client binding.",
    boundaryRule: "Remote/self-host paths remain future-only by default.",
  },
  {
    area: "Tauri Shell",
    path: "apps/desktop/src-tauri/tauri.conf.json",
    responsibility: "Tauri shell configuration and platform boundary.",
    boundaryRule: "Tauri types must not enter domain/usecase.",
  },
  {
    area: "Shared UI",
    path: "packages/ui/src/index.ts",
    responsibility: "Workspace shell, document, discovery, asset, backup view models.",
    boundaryRule: "UI displays state and sends DTOs; it does not own domain rules.",
  },
  {
    area: "Editor Boundary",
    path: "packages/editor/src/index.ts",
    responsibility: "CodeMirror editor setup and editor event boundary.",
    boundaryRule: "CodeMirror must not own document domain state.",
  },
  {
    area: "Client Contract",
    path: "packages/client-core/src/index.ts",
    responsibility: "Typed client DTOs and UI-safe command contracts.",
    boundaryRule: "No hidden runtime config or global command client.",
  },
  {
    area: "Rust Platform Runtime",
    path: "crates/cabinet-platform/src/local_desktop_runtime.rs",
    responsibility: "Native desktop bootstrap state machine and runtime composition boundary.",
    boundaryRule: "Read startup config once and inject dependencies explicitly.",
  },
  {
    area: "Rust Product Smoke",
    path: "crates/cabinet-platform/src/release_smoke.rs",
    responsibility: "Local release smoke for clean install, document, search, asset, restore.",
    boundaryRule: "Smoke code verifies behavior; it must not become hidden product logic.",
  },
  {
    area: "Document Usecase",
    path: "crates/cabinet-usecases/src/document.rs",
    responsibility: "Current document, history, restore, asset document usecases.",
    boundaryRule: "Usecases call ports and return explicit results.",
  },
  {
    area: "Search Usecase",
    path: "crates/cabinet-usecases/src/search.rs",
    responsibility: "Search command/usecase boundary.",
    boundaryRule: "Search must use index/projection ports.",
  },
  {
    area: "Graph Usecase",
    path: "crates/cabinet-usecases/src/graph.rs",
    responsibility: "Graph projection usecase boundary.",
    boundaryRule: "Graph view uses projection rather than UI-owned domain rules.",
  },
  {
    area: "Backup Usecase",
    path: "crates/cabinet-usecases/src/backup.rs",
    responsibility: "Backup package usecase boundary.",
    boundaryRule: "Filesystem package handling stays behind backup adapters.",
  },
  {
    area: "Import Usecase",
    path: "crates/cabinet-usecases/src/import.rs",
    responsibility: "Import preview/usecase boundary.",
    boundaryRule: "Preview must not mutate workspace.",
  },
  {
    area: "Ports",
    path: "crates/cabinet-ports/src/lib.rs",
    responsibility: "Repository, version, search, graph, asset, backup, logger contracts.",
    boundaryRule: "Usecases depend on port interfaces, not concrete adapters.",
  },
  {
    area: "Document Adapter",
    path: "crates/cabinet-adapters/src/local_document_repository.rs",
    responsibility: "Local filesystem-backed current document repository.",
    boundaryRule: "Filesystem access stays in adapter.",
  },
  {
    area: "Version Adapter",
    path: "crates/cabinet-adapters/src/local_version_store.rs",
    responsibility: "Local internal version store.",
    boundaryRule: "Users must not see Git/commit/branch concepts.",
  },
  {
    area: "Search Adapter",
    path: "crates/cabinet-adapters/src/local_search_index.rs",
    responsibility: "Local search index adapter.",
    boundaryRule: "Normal search must not scan raw documents.",
  },
  {
    area: "Link Adapter",
    path: "crates/cabinet-adapters/src/local_link_index.rs",
    responsibility: "Local link/backlink index adapter.",
    boundaryRule: "Backlinks use projection/index.",
  },
  {
    area: "Graph Adapter",
    path: "crates/cabinet-adapters/src/local_graph_projection.rs",
    responsibility: "Local graph projection adapter.",
    boundaryRule: "Graph reads use projection.",
  },
  {
    area: "Asset Adapter",
    path: "crates/cabinet-adapters/src/local_asset_store.rs",
    responsibility: "Local asset store adapter.",
    boundaryRule: "Asset metadata is separate from binary content.",
  },
  {
    area: "Backup Adapter",
    path: "crates/cabinet-adapters/src/local_backup_store.rs",
    responsibility: "Local backup store adapter.",
    boundaryRule: "Backup package filesystem access stays in adapter.",
  },
];

const futureOutOfScopePaths = [
  {
    area: "Server Runtime",
    path: "crates/cabinet-server",
    reason: "Future self-host/SaaS path; not a Phase 009 release target.",
  },
  {
    area: "Mobile App",
    path: "apps/mobile",
    reason: "Future iOS/Android path; not a Phase 009 release target.",
  },
  {
    area: "Self-host Scripts",
    path: "scripts",
    reason: "Existing self-host scripts may remain but Phase 009 must not require them.",
  },
];

const requiredInventoryTerms = [
  "phase009_current_inventory=passed",
  "product_scope: `personal_local_desktop`",
  "Product UI Runner",
  "Internal Shell Smoke",
  "Future Out Of Scope Paths",
  "sensitive data exclusion",
];

const forbiddenShellDescriptions = [
  /run_desktop_shell\.sh`?\s+is\s+the\s+product\s+ui\s+launcher/i,
  /run_desktop_shell\.sh`?\s+as\s+the\s+product\s+ui\s+launcher/i,
  /run_desktop_shell\.sh`?\s+제품\s+ui\s+실행/i,
];

export function transitionPhase009InventoryState(currentState, event, detail = {}) {
  if (
    currentState === Phase009InventoryState.NotStarted &&
    event === Phase009InventoryEvent.Start
  ) {
    return { state: Phase009InventoryState.InspectingPaths };
  }
  if (
    currentState === Phase009InventoryState.InspectingPaths &&
    event === Phase009InventoryEvent.PathsInspected
  ) {
    return { state: Phase009InventoryState.RenderingArtifact };
  }
  if (
    currentState === Phase009InventoryState.RenderingArtifact &&
    event === Phase009InventoryEvent.ArtifactRendered
  ) {
    return { state: Phase009InventoryState.WritingArtifact };
  }
  if (
    currentState === Phase009InventoryState.WritingArtifact &&
    event === Phase009InventoryEvent.ArtifactWritten
  ) {
    return { state: Phase009InventoryState.Passed };
  }
  if (
    [
      Phase009InventoryState.InspectingPaths,
      Phase009InventoryState.RenderingArtifact,
      Phase009InventoryState.WritingArtifact,
    ].includes(currentState) &&
    event === Phase009InventoryEvent.Fail
  ) {
    return {
      state: Phase009InventoryState.Failed,
      errorCode: detail.errorCode ?? Phase009InventoryErrorCode.IoFailed,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase009InventoryState.Failed,
    errorCode: Phase009InventoryErrorCode.InvalidTransition,
  };
}

export function validatePhase009CurrentInventoryText(text) {
  for (const pattern of forbiddenShellDescriptions) {
    if (pattern.test(text)) {
      return [
        {
          errorCode: Phase009InventoryErrorCode.ShellSmokeMisclassified,
          findingId: "run_desktop_shell.sh",
        },
      ];
    }
  }

  for (const term of requiredInventoryTerms) {
    if (!text.includes(term)) {
      return [
        {
          errorCode: Phase009InventoryErrorCode.InventoryTextMissingTerm,
          findingId: term,
        },
      ];
    }
  }

  return [];
}

export async function runPhase009CurrentInventory({
  root = process.cwd(),
  writeArtifact = true,
} = {}) {
  let state = transitionPhase009InventoryState(
    Phase009InventoryState.NotStarted,
    Phase009InventoryEvent.Start,
  ).state;

  try {
    const planText = await readFile(join(root, ".tasks", "plan.md"), "utf8");
    if (
      !planText.includes("Current product scope marker: `personal_local_desktop`") ||
      !planText.includes("phase009_current_inventory=passed")
    ) {
      const failed = transitionPhase009InventoryState(state, Phase009InventoryEvent.Fail, {
        errorCode: Phase009InventoryErrorCode.PlanMissingRequiredScope,
        findingId: ".tasks/plan.md",
      });
      return toFailedResult(failed);
    }

    const phase008Release = await readFile(
      join(root, ".tasks", "phase008", "phase008-release-gate-result.md"),
      "utf8",
    );
    if (!phase008Release.includes("phase008_release_gate=passed")) {
      const failed = transitionPhase009InventoryState(state, Phase009InventoryEvent.Fail, {
        errorCode: Phase009InventoryErrorCode.Phase008ReleaseMarkerMissing,
        findingId: ".tasks/phase008/phase008-release-gate-result.md",
      });
      return toFailedResult(failed);
    }

    for (const item of [...requiredCurrentPaths, ...futureOutOfScopePaths]) {
      const exists = await pathExists(join(root, item.path));
      if (!exists) {
        const failed = transitionPhase009InventoryState(state, Phase009InventoryEvent.Fail, {
          errorCode: Phase009InventoryErrorCode.RequiredPathMissing,
          findingId: item.path,
        });
        return toFailedResult(failed);
      }
    }

    state = transitionPhase009InventoryState(
      state,
      Phase009InventoryEvent.PathsInspected,
    ).state;

    const result = {
      passed: true,
      state: Phase009InventoryState.Passed,
      productScope: "personal_local_desktop",
      requiredCurrentPaths,
      futureOutOfScopePaths,
      validationCommandCount: 2,
      sensitiveDataExcluded: true,
    };
    const artifact = renderPhase009CurrentInventoryArtifact(result);
    const findings = validatePhase009CurrentInventoryText(artifact);
    if (findings.length > 0) {
      const failed = transitionPhase009InventoryState(state, Phase009InventoryEvent.Fail, {
        errorCode: findings[0].errorCode,
        findingId: findings[0].findingId,
      });
      return toFailedResult(failed);
    }

    state = transitionPhase009InventoryState(
      state,
      Phase009InventoryEvent.ArtifactRendered,
    ).state;

    if (writeArtifact) {
      await mkdir(join(root, ".tasks"), { recursive: true });
      await writeFile(
        join(root, ".tasks", "phase009-current-implementation-inventory.md"),
        artifact,
      );
    }

    state = transitionPhase009InventoryState(
      state,
      Phase009InventoryEvent.ArtifactWritten,
    ).state;

    return { ...result, state };
  } catch (error) {
    const failed = transitionPhase009InventoryState(state, Phase009InventoryEvent.Fail, {
      errorCode: Phase009InventoryErrorCode.IoFailed,
      findingId: error.path ?? error.message,
    });
    return toFailedResult(failed);
  }
}

export function renderPhase009CurrentInventoryArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  const marker = result.passed
    ? "phase009_current_inventory=passed"
    : "phase009_current_inventory=failed";

  return [
    "# Phase 009 Current Implementation Inventory",
    "",
    marker,
    `validation_state=${result.state}`,
    `product_scope: \`${result.productScope ?? "personal_local_desktop"}\``,
    "",
    `- status: \`${status}\``,
    "- purpose: lock current implementation boundaries before Phase 009 product work",
    "- validation commands:",
    "  - `npm run run:phase009-current-inventory-tests`",
    "  - `npm run run:phase009-current-inventory`",
    "",
    "## Current Implementation Paths",
    "",
    "| Area | Path | Responsibility | Boundary Rule |",
    "| --- | --- | --- | --- |",
    ...(result.requiredCurrentPaths ?? requiredCurrentPaths).map(
      (item) =>
        `| ${item.area} | \`${item.path}\` | ${item.responsibility} | ${item.boundaryRule} |`,
    ),
    "",
    "## Future Out Of Scope Paths",
    "",
    "| Area | Path | Reason |",
    "| --- | --- | --- |",
    ...(result.futureOutOfScopePaths ?? futureOutOfScopePaths).map(
      (item) => `| ${item.area} | \`${item.path}\` | ${item.reason} |`,
    ),
    "",
    "## Scope Lock",
    "",
    "- `scripts/run_desktop_app.sh` is the product UI runner for development.",
    "- `scripts/run_desktop_shell.sh` is internal shell smoke and not the product UI launcher.",
    "- Server runtime, mobile app, self-host scripts, collaboration scripts, and SaaS paths are future/out-of-scope for Phase 009 release gates.",
    "- Phase 009 follow-up tasks must not require server/SaaS/multi-user paths.",
    "",
    "## sensitive data exclusion",
    "",
    "- raw document body excluded",
    "- asset content excluded",
    "- AI prompt and answer excluded",
    "- provider key, token, credential, secret excluded",
    "- personal absolute local path excluded",
    "",
  ].join("\n");
}

function toFailedResult(failedTransition) {
  return {
    passed: false,
    state: Phase009InventoryState.Failed,
    errorCode: failedTransition.errorCode,
    findingId: failedTransition.findingId,
    productScope: "personal_local_desktop",
    requiredCurrentPaths,
    futureOutOfScopePaths,
  };
}

async function pathExists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

async function main() {
  const result = await runPhase009CurrentInventory({ root: process.cwd(), writeArtifact: true });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log("phase009_current_inventory=passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
