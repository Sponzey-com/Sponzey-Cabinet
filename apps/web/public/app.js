import { markdown } from "@codemirror/lang-markdown";
import { basicSetup, EditorView } from "codemirror";

const STORAGE_KEY = "sponzey-cabinet.local-workspace.v1";

const seedWorkspace = {
  workspaceId: "workspace-local",
  selectedDocumentId: "doc-source",
  documents: [
    {
      id: "doc-source",
      title: "Source Document",
      path: "docs/source.md",
      body:
        "# Source Document\n\nThis document links to [[Target Document]] and keeps attachment metadata outside the body.\n\nSearch term: searchneedle\n\n![[asset:asset-mvp|MVP Asset]]\n",
      assets: [
        {
          id: "asset-mvp",
          label: "MVP Asset",
          fileName: "mvp-e2e.txt",
          mediaType: "text/plain",
          byteSize: 23,
          status: "available",
        },
      ],
      versions: [
        {
          id: "source-v-0003",
          summary: "Restore source",
          author: "system",
          createdAt: "2026-06-23T13:00:00.000Z",
          body:
            "# Source Document\n\nThis document links to [[Target Document]] and keeps attachment metadata outside the body.\n\nSearch term: searchneedle\n\n![[asset:asset-mvp|MVP Asset]]\n",
        },
        {
          id: "source-v-0002",
          summary: "Edit source",
          author: "system",
          createdAt: "2026-06-23T12:58:00.000Z",
          body:
            "# Source Document\n\nEdited body with [[Target Document]] and ![[asset:asset-mvp|MVP Asset]].\n\nSearch term: searchneedle\n",
        },
        {
          id: "source-v-0001",
          summary: "Create source",
          author: "system",
          createdAt: "2026-06-23T12:55:00.000Z",
          body: "# Source Document\n\nInitial local document.\n",
        },
      ],
    },
    {
      id: "doc-target",
      title: "Target Document",
      path: "docs/target.md",
      body: "# Target Document\n\nBacklinks from the source document resolve here.\n",
      assets: [],
      versions: [
        {
          id: "target-v-0001",
          summary: "Create target",
          author: "system",
          createdAt: "2026-06-23T12:54:00.000Z",
          body: "# Target Document\n\nBacklinks from the source document resolve here.\n",
        },
      ],
    },
    {
      id: "doc-orphan",
      title: "Orphan Note",
      path: "docs/orphan.md",
      body: "# Orphan Note\n\nNo current document links to this note yet.\n",
      assets: [],
      versions: [
        {
          id: "orphan-v-0001",
          summary: "Create orphan",
          author: "system",
          createdAt: "2026-06-23T12:53:00.000Z",
          body: "# Orphan Note\n\nNo current document links to this note yet.\n",
        },
      ],
    },
  ],
};

let state = loadWorkspace();
let draft = createDraft(getSelectedDocument());
let searchText = "searchneedle";
let lastOperation = "Workspace loaded";
let lastQueryMs = 0;
let editorView;
let editorState = {
  state: "Loading",
  currentVersionId: getCurrentVersionId(getSelectedDocument()),
};

const app = document.querySelector("#app");

function loadWorkspace() {
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) {
    return structuredClone(seedWorkspace);
  }

  try {
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed.documents) && parsed.documents.length > 0) {
      return parsed;
    }
  } catch {
    return structuredClone(seedWorkspace);
  }

  return structuredClone(seedWorkspace);
}

function saveWorkspace() {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

function getSelectedDocument() {
  return (
    state.documents.find((document) => document.id === state.selectedDocumentId) ??
    state.documents[0]
  );
}

function createDraft(document) {
  return {
    documentId: document.id,
    title: document.title,
    path: document.path,
    body: document.body,
  };
}

function getCurrentVersionId(document) {
  return document.versions[0]?.id ?? "version-current";
}

function transitionEditorState(current, event) {
  if (
    event.type === "DocumentLoaded" &&
    ["Loading", "ReadyClean", "Saved"].includes(current.state)
  ) {
    return {
      state: "ReadyClean",
      currentVersionId: event.currentVersionId ?? current.currentVersionId,
    };
  }

  if (
    event.type === "ContentChanged" &&
    event.dirtyContentRef &&
    ["ReadyClean", "ReadyDirty", "Saved", "SaveFailed"].includes(current.state)
  ) {
    return {
      state: "ReadyDirty",
      currentVersionId: current.currentVersionId,
      dirtyContentRef: event.dirtyContentRef,
    };
  }

  if (
    event.type === "SaveRequested" &&
    ["ReadyDirty", "SaveFailed"].includes(current.state)
  ) {
    return {
      state: "Saving",
      currentVersionId: current.currentVersionId,
      dirtyContentRef: current.dirtyContentRef,
    };
  }

  if (event.type === "SaveSucceeded" && current.state === "Saving" && event.savedVersionId) {
    return {
      state: "Saved",
      currentVersionId: event.savedVersionId,
      savedVersionId: event.savedVersionId,
    };
  }

  if (event.type === "SaveFailed" && current.state === "Saving") {
    return {
      state: "SaveFailed",
      currentVersionId: current.currentVersionId,
      dirtyContentRef: current.dirtyContentRef,
      errorCode: event.errorCode ?? "DOCUMENT_SAVE_FAILED",
    };
  }

  if (event.type === "ReloadRequested" && current.state !== "Saving") {
    return { state: "Loading" };
  }

  return {
    ...current,
    errorCode: "DOCUMENT_EDITOR_INVALID_TRANSITION",
  };
}

function render() {
  destroyCodeMirrorEditor();

  const started = performance.now();
  const selected = getSelectedDocument();
  if (draft.documentId !== selected.id) {
    draft = createDraft(selected);
    editorState = transitionEditorState({ state: "Loading" }, {
      type: "DocumentLoaded",
      currentVersionId: getCurrentVersionId(selected),
    });
  }

  const searchResults = searchDocuments(searchText);
  const linkOverview = getLinkOverview(selected.id);
  const currentAssets = listDocumentAssets(selected.id);
  const workspaceSummary = createWorkspaceSummary();
  const dirty = draft.title !== selected.title || draft.path !== selected.path || draft.body !== selected.body;
  if (editorState.state === "Loading") {
    editorState = transitionEditorState(editorState, {
      type: "DocumentLoaded",
      currentVersionId: getCurrentVersionId(selected),
    });
  } else if (!dirty && ["ReadyDirty", "SaveFailed"].includes(editorState.state)) {
    editorState = {
      state: "ReadyClean",
      currentVersionId: getCurrentVersionId(selected),
    };
  }
  const saveState = saveStateFromEditorState(editorState, dirty);
  lastQueryMs = Math.round((performance.now() - started) * 10) / 10;

  app.innerHTML = `
    <main
      class="app-shell"
      data-cabinet-app-root="mounted"
      data-cabinet-bootstrap-state="ready"
    >
      <header class="topbar">
        <div class="brand">
          <div class="brand-mark">SC</div>
          <div>
            <div class="brand-title">Sponzey Cabinet</div>
            <div class="muted">Local workspace</div>
          </div>
          <span class="runtime-pill">web-local</span>
        </div>
        <div class="toolbar">
          <button id="new-document">New Document</button>
          <button id="reset-demo" class="danger">Reset Demo</button>
          <button
            id="save-document"
            class="primary"
            data-cabinet-save-state="${escapeAttribute(saveState)}"
            data-cabinet-saved-version="${escapeAttribute(editorState.savedVersionId ?? editorState.currentVersionId ?? "")}"
          >${dirty ? "Save Changes" : "Saved"}</button>
        </div>
      </header>

      <aside class="sidebar">
        <input id="search-input" value="${escapeAttribute(searchText)}" placeholder="Search documents" />
        <div class="section-title">Documents</div>
        <div class="document-list">
          ${state.documents.map((document) => renderDocumentItem(document, selected.id)).join("")}
        </div>
        <div class="section-title">Search Results</div>
        <div class="panel-list search-results">
          ${searchResults.length === 0 ? renderEmpty("No matching document") : searchResults.map(renderSearchResult).join("")}
        </div>
      </aside>

      <section class="workspace" data-cabinet-workspace-shell="ready">
        <div class="editor-header">
          <input id="title-input" value="${escapeAttribute(draft.title)}" aria-label="Document title" />
          <input id="path-input" value="${escapeAttribute(draft.path)}" aria-label="Document path" />
        </div>
        <div
          id="body-editor"
          class="codemirror-host"
          data-cabinet-editor="mounting"
          data-cabinet-editor-state="${escapeAttribute(editorState.state)}"
          data-editor-document-id="${escapeAttribute(draft.documentId)}"
          aria-label="Document body"
        ></div>
        <div class="editor-actions">
          <button id="insert-wikilink">Insert Wikilink</button>
          <button id="insert-asset-ref">Insert Asset Reference</button>
          <button id="restore-latest">Restore Latest Version</button>
          <span class="metric-pill">current query ${lastQueryMs}ms</span>
          <span class="metric-pill">${selected.versions.length} history entries</span>
        </div>
        <div class="panel">
          <h2>Markdown Preview</h2>
          <div class="preview markdown-preview">${renderMarkdownPreview(draft.body)}</div>
        </div>
      </section>

      <aside class="inspector" data-cabinet-discovery-panel="ready">
        <section class="panel">
          <h2>History</h2>
          <div class="panel-list">
            ${selected.versions.map(renderHistoryEntry).join("")}
          </div>
        </section>

        <section class="panel">
          <h2>Links</h2>
          <div class="stack">
            ${renderLinkGroup("Backlinks", linkOverview.backlinks)}
            ${renderLinkGroup("Unresolved", linkOverview.unresolvedLinks)}
            ${renderLinkGroup("Orphans", linkOverview.orphans)}
          </div>
        </section>

        <section class="panel" data-cabinet-graph-panel="ready">
          <h2>Graph</h2>
          ${renderGraph(selected.id)}
        </section>

        <section class="panel" data-cabinet-asset-panel="ready">
          <h2>Assets</h2>
          <input id="asset-file" class="file-input" type="file" />
          <div class="panel-list" style="margin-top: 10px">
            ${currentAssets.length === 0 ? renderEmpty("No asset metadata") : currentAssets.map(renderAsset).join("")}
          </div>
        </section>

        <section class="panel" data-cabinet-backup-panel="ready">
          <h2>Backup</h2>
          <div class="stack" data-cabinet-backup-manifest="ready">
            <div class="row">
              <strong>${workspaceSummary.documentCount} documents</strong>
              <span class="muted">${workspaceSummary.versionCount} versions</span>
            </div>
            <div class="muted">${workspaceSummary.assetCount} assets · platform app data · no manual config required</div>
            <button type="button">Create Backup</button>
          </div>
        </section>

        <section class="panel" data-cabinet-import-panel="preview-ready">
          <h2>Import Preview</h2>
          <div class="stack">
            <div class="row">
              <strong>${workspaceSummary.documentCount} scanned documents</strong>
              <span class="muted">${workspaceSummary.assetCount} asset references</span>
            </div>
            <div class="muted">Preview only · workspace is not changed before apply</div>
            <button type="button">Scan Source</button>
          </div>
        </section>

        <section class="panel" data-cabinet-restore-panel="confirmation-required">
          <h2>Restore</h2>
          <div class="stack">
            <div class="row">
              <strong>Validation required</strong>
              <span class="muted">Confirmation required before apply</span>
            </div>
            <div class="muted">Restore uses staging and does not expose Git concepts</div>
          </div>
        </section>

        <section class="panel" data-cabinet-recovery-panel="action-ready">
          <h2>Recovery</h2>
          <div class="stack">
            <button type="button">Rebuild Index</button>
            <button type="button">Repair Workspace</button>
          </div>
        </section>
      </aside>

      <footer class="statusbar" data-cabinet-current-history-split="ready">
        <span>${escapeHtml(lastOperation)}</span>
        <span>current/history split: current snapshot reads do not scan history</span>
      </footer>
    </main>
  `;

  bindEvents();
  mountCodeMirrorEditor(draft.body);
}

function createWorkspaceSummary() {
  return state.documents.reduce(
    (summary, document) => ({
      documentCount: summary.documentCount + 1,
      versionCount: summary.versionCount + document.versions.length,
      assetCount: summary.assetCount + document.assets.length,
    }),
    { documentCount: 0, versionCount: 0, assetCount: 0 },
  );
}

function saveStateFromEditorState(snapshot, dirty) {
  if (snapshot.state === "Saved") {
    return "saved";
  }
  if (snapshot.state === "Saving") {
    return "saving";
  }
  if (snapshot.state === "SaveFailed") {
    return "failed";
  }
  if (snapshot.state === "ReadyDirty" || dirty) {
    return "dirty";
  }
  return "clean";
}

function renderDocumentItem(document, selectedId) {
  const active = document.id === selectedId ? " active" : "";
  return `
    <button class="document-item${active}" data-select-document="${escapeAttribute(document.id)}">
      <span class="document-title">${escapeHtml(document.title)}</span>
      <span class="document-path">${escapeHtml(document.path)}</span>
    </button>
  `;
}

function renderSearchResult(result) {
  return `
    <button class="result-item document-item" data-select-document="${escapeAttribute(result.id)}">
      <span class="document-title">${escapeHtml(result.title)}</span>
      <span class="document-path">${escapeHtml(result.path)}</span>
      <span class="snippet">${escapeHtml(result.snippet)}</span>
    </button>
  `;
}

function renderHistoryEntry(entry) {
  return `
    <div class="history-item">
      <div class="row">
        <strong>${escapeHtml(entry.id)}</strong>
        <span class="muted">${formatDate(entry.createdAt)}</span>
      </div>
      <div class="muted">${escapeHtml(entry.summary)} by ${escapeHtml(entry.author)}</div>
      <button data-restore-version="${escapeAttribute(entry.id)}">Restore</button>
    </div>
  `;
}

function renderLinkGroup(label, items) {
  const body = items.length === 0 ? renderEmpty("None") : items.map((item) => {
    if (item.documentId) {
      return `<button class="link-item document-item" data-select-document="${escapeAttribute(item.documentId)}">${escapeHtml(item.label)}</button>`;
    }
    return `<div class="link-item">${escapeHtml(item.label)}</div>`;
  }).join("");
  return `<div data-cabinet-link-group="${escapeAttribute(label.toLowerCase())}"><div class="section-title">${escapeHtml(label)}</div>${body}</div>`;
}

function renderAsset(asset) {
  return `
    <div class="asset-item" data-cabinet-asset-metadata="ready">
      <div class="row">
        <strong>${escapeHtml(asset.label)}</strong>
        <span class="muted">${escapeHtml(asset.status)}</span>
      </div>
      <div class="muted">${escapeHtml(asset.fileName)} · ${escapeHtml(asset.mediaType)} · ${asset.byteSize} bytes</div>
    </div>
  `;
}

function renderEmpty(text) {
  return `<div class="empty">${escapeHtml(text)}</div>`;
}

function renderGraph(activeDocumentId) {
  const nodes = state.documents.map((document, index) => {
    const angle = (Math.PI * 2 * index) / Math.max(state.documents.length, 1) - Math.PI / 2;
    return {
      id: document.id,
      title: document.title,
      x: 150 + Math.cos(angle) * 94,
      y: 102 + Math.sin(angle) * 72,
    };
  });
  const edges = [];
  for (const document of state.documents) {
    for (const target of parseWikilinks(document.body)) {
      const targetDocument = findDocumentByTarget(target);
      if (targetDocument) {
        edges.push({ source: document.id, target: targetDocument.id });
      }
    }
  }

  const lines = edges.map((edge) => {
    const source = nodes.find((node) => node.id === edge.source);
    const target = nodes.find((node) => node.id === edge.target);
    if (!source || !target) {
      return "";
    }
    return `<line x1="${source.x}" y1="${source.y}" x2="${target.x}" y2="${target.y}"></line>`;
  }).join("");

  const circles = nodes.map((node) => `
    <g>
      <circle class="${node.id === activeDocumentId ? "active-node" : ""}" cx="${node.x}" cy="${node.y}" r="12"></circle>
      <text x="${node.x + 16}" y="${node.y + 4}">${escapeHtml(node.title)}</text>
    </g>
  `).join("");

  return `<svg class="graph" viewBox="0 0 300 210" role="img" aria-label="Document graph">${lines}${circles}</svg>`;
}

function renderMarkdownPreview(source) {
  const lines = source.split(/\r?\n/);
  const html = [];
  let inCodeBlock = false;
  let codeLines = [];
  let inList = false;

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex];
    if (line.trim().startsWith("```")) {
      if (inCodeBlock) {
        html.push(`<pre><code>${escapeHtml(codeLines.join("\n"))}</code></pre>`);
        codeLines = [];
        inCodeBlock = false;
      } else {
        closeList();
        inCodeBlock = true;
      }
      continue;
    }

    if (inCodeBlock) {
      codeLines.push(line);
      continue;
    }

    const trimmed = line.trim();
    if (!trimmed) {
      closeList();
      continue;
    }

    if (isMarkdownTableStart(lines, lineIndex)) {
      closeList();
      const table = collectMarkdownTable(lines, lineIndex);
      html.push(renderMarkdownTable(table.rows, table.alignments));
      lineIndex += table.consumed - 1;
      continue;
    }

    const heading = trimmed.match(/^(#{1,6})\s+(.+)$/);
    if (heading) {
      closeList();
      const level = heading[1].length;
      html.push(`<h${level}>${renderInlineMarkdown(heading[2])}</h${level}>`);
      continue;
    }

    const listItem = trimmed.match(/^[-*]\s+(.+)$/);
    if (listItem) {
      if (!inList) {
        html.push("<ul>");
        inList = true;
      }
      html.push(`<li>${renderInlineMarkdown(listItem[1])}</li>`);
      continue;
    }

    closeList();
    html.push(`<p>${renderInlineMarkdown(trimmed)}</p>`);
  }

  if (inCodeBlock) {
    html.push(`<pre><code>${escapeHtml(codeLines.join("\n"))}</code></pre>`);
  }
  closeList();

  return html.length > 0 ? html.join("") : "<p></p>";

  function closeList() {
    if (inList) {
      html.push("</ul>");
      inList = false;
    }
  }
}

function isMarkdownTableStart(lines, lineIndex) {
  const header = lines[lineIndex]?.trim() ?? "";
  const separator = lines[lineIndex + 1]?.trim() ?? "";
  return parseMarkdownTableRow(header).length > 1 && parseMarkdownTableSeparator(separator);
}

function collectMarkdownTable(lines, startIndex) {
  const rows = [parseMarkdownTableRow(lines[startIndex])];
  const alignments = parseMarkdownTableSeparator(lines[startIndex + 1]) ?? [];
  let consumed = 2;

  for (let lineIndex = startIndex + 2; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex].trim();
    if (!line || !line.includes("|")) {
      break;
    }

    rows.push(parseMarkdownTableRow(line));
    consumed += 1;
  }

  return { rows, alignments, consumed };
}

function renderMarkdownTable(rows, alignments) {
  const [header, ...bodyRows] = rows;
  const headerHtml = header.map((cell, index) => {
    return `<th${renderTableAlignmentAttribute(alignments[index])}>${renderInlineMarkdown(cell)}</th>`;
  }).join("");
  const bodyHtml = bodyRows.map((row) => {
    const cells = header.map((_, index) => row[index] ?? "");
    return `<tr>${cells.map((cell, index) => {
      return `<td${renderTableAlignmentAttribute(alignments[index])}>${renderInlineMarkdown(cell)}</td>`;
    }).join("")}</tr>`;
  }).join("");

  return `<table><thead><tr>${headerHtml}</tr></thead><tbody>${bodyHtml}</tbody></table>`;
}

function renderTableAlignmentAttribute(alignment) {
  return alignment ? ` class="align-${alignment}"` : "";
}

function parseMarkdownTableRow(line) {
  const trimmed = line.trim();
  if (!trimmed.includes("|")) {
    return [];
  }

  const withoutOuterPipes = trimmed.replace(/^\|/, "").replace(/\|$/, "");
  return withoutOuterPipes.split("|").map((cell) => cell.trim());
}

function parseMarkdownTableSeparator(line) {
  const cells = parseMarkdownTableRow(line);
  if (cells.length < 2) {
    return undefined;
  }

  const alignments = [];
  for (const cell of cells) {
    const normalized = cell.replace(/\s+/g, "");
    if (!/^:?-{3,}:?$/.test(normalized)) {
      return undefined;
    }

    const left = normalized.startsWith(":");
    const right = normalized.endsWith(":");
    if (left && right) {
      alignments.push("center");
    } else if (right) {
      alignments.push("right");
    } else if (left) {
      alignments.push("left");
    } else {
      alignments.push(undefined);
    }
  }

  return alignments;
}

function renderInlineMarkdown(source) {
  let output = escapeHtml(source);
  output = output.replace(/`([^`]+)`/g, "<code>$1</code>");
  output = output.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  output = output.replace(/!\[\[asset:([^|\]]+)\|([^\]]+)]]/g, (_, assetId, label) => {
    return `<span class="asset-token" title="${escapeAttribute(assetId)}">${escapeHtml(label)}</span>`;
  });
  output = output.replace(/\[\[([^\]|]+)(?:\|([^\]]+))?]]/g, (_, target, label) => {
    const text = label || target;
    return `<button class="inline-link" data-preview-wikilink="${escapeAttribute(target)}">${escapeHtml(text)}</button>`;
  });
  output = output.replace(/\[([^\]]+)]\(([^)]+)\)/g, (_, text, href) => {
    const safeHref = sanitizeHref(href);
    return `<a href="${safeHref}" target="_blank" rel="noreferrer">${escapeHtml(text)}</a>`;
  });
  return output;
}

function sanitizeHref(href) {
  const value = href.trim();
  if (
    value.startsWith("http://") ||
    value.startsWith("https://") ||
    value.startsWith("mailto:") ||
    value.startsWith("#") ||
    value.startsWith("/")
  ) {
    return escapeAttribute(value);
  }
  return "#";
}

function bindEvents() {
  document.querySelectorAll("[data-select-document]").forEach((button) => {
    button.addEventListener("click", () => {
      state.selectedDocumentId = button.dataset.selectDocument;
      draft = createDraft(getSelectedDocument());
      lastOperation = "Current document loaded";
      saveWorkspace();
      render();
    });
  });

  document.querySelector("#search-input").addEventListener("input", (event) => {
    searchText = event.target.value;
    lastOperation = "Search query updated";
    render();
  });

  document.querySelector("#title-input").addEventListener("input", (event) => {
    draft.title = event.target.value;
    lastOperation = "Title edited";
    markDraftChanged();
  });

  document.querySelector("#path-input").addEventListener("input", (event) => {
    draft.path = event.target.value;
    lastOperation = "Path edited";
    markDraftChanged();
  });

  document.querySelector("#save-document").addEventListener("click", saveCurrentDocument);
  document.querySelector("#new-document").addEventListener("click", createDocument);
  document.querySelector("#reset-demo").addEventListener("click", resetDemo);
  document.querySelector("#insert-wikilink").addEventListener("click", insertWikilink);
  document.querySelector("#insert-asset-ref").addEventListener("click", insertAssetReference);
  document.querySelector("#restore-latest").addEventListener("click", restoreLatestVersion);

  document.querySelectorAll("[data-restore-version]").forEach((button) => {
    button.addEventListener("click", () => restoreVersion(button.dataset.restoreVersion));
  });

  document.querySelector("#asset-file").addEventListener("change", attachFileMetadata);

  document.querySelectorAll("[data-preview-wikilink]").forEach((button) => {
    button.addEventListener("click", () => {
      const target = button.dataset.previewWikilink ?? "";
      const document = findDocumentByTarget(target);
      if (document) {
        state.selectedDocumentId = document.id;
        draft = createDraft(document);
        lastOperation = "Wikilink opened";
        saveWorkspace();
        render();
      }
    });
  });
}

function mountCodeMirrorEditor(initialBody) {
  const parent = document.querySelector("#body-editor");
  if (!parent) {
    return;
  }

  editorView = new EditorView({
    doc: initialBody,
    parent,
    extensions: [
      basicSetup,
      markdown(),
      EditorView.lineWrapping,
      EditorView.updateListener.of((update) => {
        if (!update.docChanged) {
          return;
        }

        draft.body = update.state.doc.toString();
        lastOperation = "Body edited";
        markDraftChanged();
      }),
    ],
  });
  parent.dataset.cabinetEditor = "mounted";
}

function destroyCodeMirrorEditor() {
  if (editorView) {
    editorView.destroy();
    editorView = undefined;
  }
}

function markDraftChanged() {
  editorState = transitionEditorState(editorState, {
    type: "ContentChanged",
    dirtyContentRef: draft.documentId,
  });
  const saveButton = document.querySelector("#save-document");
  const status = document.querySelector(".statusbar span");
  const preview = document.querySelector(".preview");
  const editorHost = document.querySelector("#body-editor");
  if (saveButton) {
    saveButton.textContent = "Save Changes";
    saveButton.dataset.cabinetSaveState = "dirty";
  }
  if (status) {
    status.textContent = lastOperation;
  }
  if (preview) {
    preview.innerHTML = renderMarkdownPreview(draft.body);
  }
  if (editorHost) {
    editorHost.dataset.cabinetEditorState = editorState.state;
  }
}

function saveCurrentDocument() {
  const document = getSelectedDocument();
  const changed = draft.title !== document.title || draft.path !== document.path || draft.body !== document.body;
  if (changed) {
    editorState = transitionEditorState(editorState, { type: "SaveRequested" });
  }
  document.title = draft.title.trim() || "Untitled";
  document.path = draft.path.trim() || `docs/${document.id}.md`;
  document.body = draft.body;

  if (changed) {
    const nextVersionId = `v-${Date.now()}`;
    document.versions.unshift({
      id: nextVersionId,
      summary: "Save document",
      author: "local",
      createdAt: new Date().toISOString(),
      body: document.body,
    });
    editorState = transitionEditorState(editorState, {
      type: "SaveSucceeded",
      savedVersionId: nextVersionId,
    });
  } else {
    editorState = {
      state: "ReadyClean",
      currentVersionId: getCurrentVersionId(document),
    };
  }

  lastOperation = changed ? "Document saved" : "No changes to save";
  saveWorkspace();
  render();
}

function createDocument() {
  const count = state.documents.length + 1;
  const id = `doc-${Date.now()}`;
  const document = {
    id,
    title: `Untitled ${count}`,
    path: `docs/untitled-${count}.md`,
    body: `# Untitled ${count}\n\n`,
    assets: [],
    versions: [
      {
        id: `v-${Date.now()}`,
        summary: "Create document",
        author: "local",
        createdAt: new Date().toISOString(),
        body: `# Untitled ${count}\n\n`,
      },
    ],
  };
  state.documents.push(document);
  state.selectedDocumentId = id;
  draft = createDraft(document);
  editorState = transitionEditorState({ state: "Loading" }, {
    type: "DocumentLoaded",
    currentVersionId: getCurrentVersionId(document),
  });
  lastOperation = "Document created";
  saveWorkspace();
  render();
}

function resetDemo() {
  state = structuredClone(seedWorkspace);
  draft = createDraft(getSelectedDocument());
  editorState = transitionEditorState({ state: "Loading" }, {
    type: "DocumentLoaded",
    currentVersionId: getCurrentVersionId(getSelectedDocument()),
  });
  searchText = "searchneedle";
  lastOperation = "Demo workspace reset";
  saveWorkspace();
  render();
}

function insertWikilink() {
  const target = state.documents.find((document) => document.id !== draft.documentId)?.title ?? "Target Document";
  appendToBody(`[[${target}]]`);
  lastOperation = "Wikilink inserted";
  render();
}

function insertAssetReference() {
  const document = getSelectedDocument();
  const asset = document.assets[0] ?? {
    id: "asset-mvp",
    label: "MVP Asset",
  };
  appendToBody(`![[asset:${asset.id}|${asset.label}]]`);
  lastOperation = "Asset reference inserted";
  render();
}

function appendToBody(text) {
  const separator = draft.body.endsWith("\n") ? "" : "\n";
  draft.body = `${draft.body}${separator}${text}\n`;
}

function attachFileMetadata(event) {
  const file = event.target.files?.[0];
  if (!file) {
    return;
  }
  const document = getSelectedDocument();
  const asset = {
    id: `asset-${Date.now()}`,
    label: file.name,
    fileName: file.name,
    mediaType: file.type || "application/octet-stream",
    byteSize: file.size,
    status: "available",
  };
  document.assets.push(asset);
  appendToBody(`![[asset:${asset.id}|${asset.label}]]`);
  lastOperation = "Asset metadata attached";
  saveWorkspace();
  render();
}

function restoreLatestVersion() {
  const document = getSelectedDocument();
  const latest = document.versions[0];
  if (latest) {
    restoreVersion(latest.id);
  }
}

function restoreVersion(versionId) {
  const document = getSelectedDocument();
  const version = document.versions.find((entry) => entry.id === versionId);
  if (!version) {
    return;
  }
  document.body = version.body;
  const restoreVersionId = `v-${Date.now()}`;
  document.versions.unshift({
    id: restoreVersionId,
    summary: `Restore ${version.id}`,
    author: "local",
    createdAt: new Date().toISOString(),
    body: document.body,
  });
  draft = createDraft(document);
  editorState = {
    state: "Saved",
    currentVersionId: restoreVersionId,
    savedVersionId: restoreVersionId,
  };
  lastOperation = `Restored ${version.id}`;
  saveWorkspace();
  render();
}

function searchDocuments(text) {
  const query = text.trim().toLowerCase();
  if (!query) {
    return [];
  }
  return state.documents
    .filter((document) => {
      const content = `${document.title}\n${document.path}\n${document.body}`.toLowerCase();
      return content.includes(query);
    })
    .map((document) => ({
      id: document.id,
      title: document.title,
      path: document.path,
      snippet: createSnippet(document.body, query),
    }));
}

function createSnippet(body, query) {
  const lower = body.toLowerCase();
  const index = lower.indexOf(query);
  if (index === -1) {
    return body.slice(0, 90);
  }
  const start = Math.max(index - 32, 0);
  return body.slice(start, start + 110).replace(/\s+/g, " ");
}

function getLinkOverview(activeDocumentId) {
  const selected = state.documents.find((document) => document.id === activeDocumentId);
  const backlinks = [];
  const unresolvedLinks = [];
  const outgoingIds = new Set();

  for (const document of state.documents) {
    const links = parseWikilinks(document.body);
    for (const target of links) {
      const targetDocument = findDocumentByTarget(target);
      if (targetDocument) {
        outgoingIds.add(targetDocument.id);
        if (selected && targetDocument.id === selected.id && document.id !== selected.id) {
          backlinks.push({
            documentId: document.id,
            label: document.title,
          });
        }
      } else {
        unresolvedLinks.push({
          label: `${document.title} -> ${target}`,
        });
      }
    }
  }

  const incomingIds = new Set(backlinks.map((backlink) => backlink.documentId));
  const orphans = state.documents
    .filter((document) => document.id !== activeDocumentId)
    .filter((document) => !incomingIds.has(document.id) && !outgoingIds.has(document.id))
    .map((document) => ({ documentId: document.id, label: document.title }));

  return { backlinks, unresolvedLinks, orphans };
}

function listDocumentAssets(documentId) {
  const document = state.documents.find((candidate) => candidate.id === documentId);
  return document?.assets ?? [];
}

function parseWikilinks(body) {
  const links = [];
  const pattern = /(?<!!)\[\[([^\]]+)]]/g;
  let match = pattern.exec(body);
  while (match) {
    const [target] = match[1].split("|");
    if (target.trim()) {
      links.push(target.trim());
    }
    match = pattern.exec(body);
  }
  return links;
}

function findDocumentByTarget(target) {
  const normalized = normalizeTarget(target);
  return state.documents.find((document) => {
    return (
      normalizeTarget(document.title) === normalized ||
      normalizeTarget(document.path) === normalized ||
      normalizeTarget(document.id) === normalized
    );
  });
}

function normalizeTarget(value) {
  return value.trim().toLowerCase().replace(/\.md$/, "");
}

function formatDate(value) {
  return new Intl.DateTimeFormat(undefined, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function escapeAttribute(value) {
  return escapeHtml(value).replaceAll("`", "&#096;");
}

render();
