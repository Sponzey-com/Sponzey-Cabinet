import React, { useEffect, useRef, useState } from "react";
import { Maximize2, Pause, Play, RotateCcw, ZoomIn, ZoomOut } from "lucide-react";

import { ForceAtlas2TopologyLayoutAdapter } from "./forceatlas2_topology_layout_adapter.ts";
import { SigmaTopologyRendererAdapter } from "./sigma_topology_renderer_adapter.ts";
import { TopologyVisualOrchestrator } from "./topology_visual_orchestrator.ts";
import type { TopologyRendererModel } from "./topology_renderer_port.ts";
import {
  cameraPreferenceFromRenderer,
  rendererCameraFromPreference,
  type DesktopGraphCameraPreference,
} from "./desktop_graph_preference.ts";

export interface TopologySemanticNode {
  readonly identity: string;
  readonly label: string;
  readonly kind: TopologyRendererModel["nodes"][number]["kind"];
  readonly kindLabel: string;
  readonly canNavigate: boolean;
}

export type TopologySemanticFocusKey = "ArrowDown" | "ArrowUp" | "Home" | "End";

export function nextTopologySemanticFocus(
  keys: readonly string[],
  currentKey: string | undefined,
  key: TopologySemanticFocusKey,
): string | undefined {
  if (keys.length === 0) return undefined;
  if (key === "Home") return keys[0];
  if (key === "End") return keys[keys.length - 1];
  const currentIndex = keys.indexOf(currentKey ?? "");
  if (currentIndex < 0) return keys[0];
  const delta = key === "ArrowDown" ? 1 : -1;
  return keys[(currentIndex + delta + keys.length) % keys.length];
}

export function ReactTopologyVisualHost({
  model,
  semanticNodes,
  onNodeSelected,
  onNodeActivated,
  cameraPreference,
  onCameraPreferenceChanged,
}: {
  readonly model: TopologyRendererModel;
  readonly semanticNodes: readonly TopologySemanticNode[];
  readonly onNodeSelected: (key: string) => void;
  readonly onNodeActivated: (key: string) => void;
  readonly cameraPreference?: DesktopGraphCameraPreference;
  readonly onCameraPreferenceChanged?: (camera: DesktopGraphCameraPreference) => void;
}): React.ReactElement {
  const e = React.createElement;
  const hostRef = useRef<HTMLDivElement | null>(null);
  const orchestratorRef = useRef<TopologyVisualOrchestrator<HTMLElement> | null>(null);
  const mountedRef = useRef(false);
  const modelRef = useRef(model);
  const callbackRef = useRef({ onNodeSelected, onNodeActivated, onCameraPreferenceChanged });
  const restoringCameraRef = useRef(Boolean(cameraPreference));
  const semanticButtonRefs = useRef(new Map<string, HTMLButtonElement>());
  const [visualState, setVisualState] = useState<"Initializing" | "Ready" | "Failed">("Initializing");
  const [zoomPercent, setZoomPercent] = useState(cameraPreference?.zoomPercent ?? 100);
  const [layoutPaused, setLayoutPaused] = useState(false);
  const selectedKey = model.nodes.find((node) => node.selected)?.key;
  const [focusedKey, setFocusedKey] = useState<string | undefined>(() => selectedKey ?? semanticNodes[0]?.identity);
  modelRef.current = model;
  callbackRef.current = { onNodeSelected, onNodeActivated, onCameraPreferenceChanged };

  useEffect(() => {
    const keys = new Set(semanticNodes.map((node) => node.identity));
    if (selectedKey && keys.has(selectedKey)) {
      setFocusedKey(selectedKey);
      return;
    }
    setFocusedKey((current) => current && keys.has(current) ? current : semanticNodes[0]?.identity);
  }, [selectedKey, semanticNodes]);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return undefined;
    const renderer = new SigmaTopologyRendererAdapter<HTMLElement>();
    const layout = new ForceAtlas2TopologyLayoutAdapter();
    const orchestrator = new TopologyVisualOrchestrator(renderer, layout, {
      onNodeSelected: (key) => callbackRef.current.onNodeSelected(key),
      onNodeActivated: (key) => callbackRef.current.onNodeActivated(key),
      onFailure: () => setVisualState("Failed"),
      onLayoutPaused: setLayoutPaused,
      onCameraChanged: (camera) => {
        const preference = cameraPreferenceFromRenderer(camera);
        if (!preference) return;
        setZoomPercent(preference.zoomPercent);
        if (!restoringCameraRef.current) callbackRef.current.onCameraPreferenceChanged?.(preference);
      },
    });
    orchestratorRef.current = orchestrator;
    const reducedMotion = globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
    let active = true;
    let observer: ResizeObserver | undefined;
    const mountedModel = modelRef.current;
    void orchestrator.mount(
      host,
      mountedModel,
      reducedMotion,
      cameraPreference ? rendererCameraFromPreference(cameraPreference) : undefined,
    ).then(() => {
      if (!active) return;
      restoringCameraRef.current = false;
      mountedRef.current = true;
      setVisualState("Ready");
      if (modelRef.current !== mountedModel) orchestrator.update(modelRef.current, reducedMotion);
      const resize = () => {
        const bounds = host.getBoundingClientRect();
        if (bounds.width > 0 && bounds.height > 0) {
          orchestrator.resize({ width: bounds.width, height: bounds.height, pixelRatio: globalThis.devicePixelRatio || 1 });
        }
      };
      resize();
      if (typeof ResizeObserver !== "undefined") {
        observer = new ResizeObserver(resize);
        observer.observe(host);
      }
    }).catch(() => {
      if (active) setVisualState("Failed");
    });
    return () => {
      active = false;
      mountedRef.current = false;
      observer?.disconnect();
      orchestrator.dispose();
      orchestratorRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (!mountedRef.current) return;
    const reducedMotion = globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
    orchestratorRef.current?.update(model, reducedMotion);
  }, [model]);

  const changeZoom = (next: number) => {
    if (!mountedRef.current) return;
    orchestratorRef.current?.setZoomPercent(next);
    setZoomPercent(next);
  };
  const fit = () => {
    if (!mountedRef.current) return;
    orchestratorRef.current?.fit();
    setZoomPercent(100);
  };
  const reducedMotion = () => globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
  const resetLayout = () => {
    if (!mountedRef.current) return;
    orchestratorRef.current?.resetLayout(reducedMotion());
    setLayoutPaused(false);
    setZoomPercent(100);
  };
  const toggleLayout = () => {
    if (!mountedRef.current) return;
    if (layoutPaused) orchestratorRef.current?.resumeLayout(reducedMotion());
    else orchestratorRef.current?.pauseLayout();
    setLayoutPaused(!layoutPaused);
  };
  const moveSemanticFocus = (key: TopologySemanticFocusKey) => {
    const next = nextTopologySemanticFocus(semanticNodes.map((node) => node.identity), focusedKey, key);
    if (!next) return;
    setFocusedKey(next);
    globalThis.queueMicrotask(() => semanticButtonRefs.current.get(next)?.focus());
  };
  const selectedLabel = semanticNodes.find((node) => node.identity === selectedKey)?.label;
  const semanticLabels = new Map(semanticNodes.map((node) => [node.identity, node.label]));

  return e(
    "div",
    { className: `topology-visual-host state-${visualState.toLowerCase()}`, "data-topology-visual-state": visualState, "data-graph-camera-zoom": zoomPercent },
    e("div", { ref: hostRef, className: "topology-renderer-host", "data-topology-renderer-host": "accelerated", "aria-hidden": "true" }),
    visualState === "Failed"
      ? e("p", { className: "topology-renderer-failure", role: "alert" }, "가속 지도를 표시하지 못했습니다. 문서 목록으로 계속 탐색할 수 있습니다.")
      : null,
    e(
      "nav",
      { className: "topology-semantic-list", "data-topology-semantic-list": "available", "aria-label": "지도 문서 목록" },
      e("ul", null, semanticNodes.map((node) => e(
        "li",
        { key: node.identity },
        e("button", {
          type: "button",
          "data-action": "select-graph-node",
          "data-graph-node-id": node.identity,
          tabIndex: focusedKey === node.identity ? 0 : -1,
          "aria-current": selectedKey === node.identity ? "true" : undefined,
          ref: (element: HTMLButtonElement | null) => {
            if (element) semanticButtonRefs.current.set(node.identity, element);
            else semanticButtonRefs.current.delete(node.identity);
          },
          onFocus: () => setFocusedKey(node.identity),
          onClick: () => onNodeSelected(node.identity),
          onKeyDown: (event: React.KeyboardEvent<HTMLButtonElement>) => {
            if (["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
              event.preventDefault();
              moveSemanticFocus(event.key as TopologySemanticFocusKey);
              return;
            }
            if (event.key === "Enter" && node.canNavigate) {
              event.preventDefault();
              onNodeActivated(node.identity);
            }
          },
        }, e("strong", null, node.label), e("small", null, node.kindLabel)),
        node.canNavigate && (node.kind === "document" || node.kind === "attachment")
          ? e("button", {
              type: "button",
              className: "topology-semantic-open",
              "data-action": node.kind === "attachment" ? "open-graph-asset" : "open-graph-document",
              tabIndex: -1,
              "aria-label": `${node.label} 열기`,
              onClick: () => onNodeActivated(node.identity),
            }, "열기")
          : null,
      ))),
    ),
    e(
      "ul",
      {
        className: "topology-semantic-edge-list",
        "data-topology-semantic-edges": "available",
        "aria-label": "지도 연결 목록",
      },
      model.edges.map((edge) => e(
        "li",
        {
          key: edge.key,
          "data-edge-kind": edge.kind,
          "data-edge-source-id": edge.sourceKey,
          "data-edge-target-id": edge.targetKey,
        },
        `${semanticLabels.get(edge.sourceKey) ?? "알 수 없는 항목"} → ${semanticLabels.get(edge.targetKey) ?? "알 수 없는 항목"}`,
      )),
    ),
    e(
      "p",
      { className: "topology-accessibility-summary", role: "status", "aria-live": "polite" },
      `노드 ${semanticNodes.length}개, 연결 ${model.edges.length}개${selectedLabel ? `, 선택: ${selectedLabel}` : ""}`,
    ),
    e("div", { className: "graph-zoom-controls" },
      e("button", { type: "button", "data-action": "graph-zoom-in", disabled: zoomPercent >= 200 || visualState !== "Ready", "aria-label": `확대, 현재 ${zoomPercent}%`, title: "확대", onClick: () => changeZoom(Math.min(200, zoomPercent + 25)) }, e(ZoomIn, { size: 15, "aria-hidden": true })),
      e("button", { type: "button", "data-action": "graph-zoom-out", disabled: zoomPercent <= 50 || visualState !== "Ready", "aria-label": `축소, 현재 ${zoomPercent}%`, title: "축소", onClick: () => changeZoom(Math.max(50, zoomPercent - 25)) }, e(ZoomOut, { size: 15, "aria-hidden": true })),
      e("button", { type: "button", "data-action": "graph-fit-view", disabled: visualState !== "Ready", "aria-label": "화면에 맞춤", title: "화면에 맞춤", onClick: fit }, e(Maximize2, { size: 15, "aria-hidden": true })),
      e("button", { type: "button", "data-action": "graph-reset-layout", disabled: visualState !== "Ready", "aria-label": "배치 초기화", title: "배치 초기화", onClick: resetLayout }, e(RotateCcw, { size: 15, "aria-hidden": true })),
      e("button", { type: "button", "data-action": layoutPaused ? "graph-resume-layout" : "graph-pause-layout", disabled: visualState !== "Ready", "aria-label": layoutPaused ? "자동 배치 재개" : "자동 배치 일시정지", title: layoutPaused ? "자동 배치 재개" : "자동 배치 일시정지", "aria-pressed": layoutPaused, onClick: toggleLayout }, e(layoutPaused ? Play : Pause, { size: 15, "aria-hidden": true })),
    ),
  );
}
