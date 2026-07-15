import type { DesktopCanvasData, DesktopCanvasEdge, DesktopCanvasNode } from "./tauri_canvas_transport.ts";

export interface DesktopCanvasViewportProjectionOptions {
  readonly width: number;
  readonly height: number;
  readonly overscan: number;
  readonly nodeLimit: number;
  readonly edgeLimit: number;
}

export interface DesktopCanvasViewportProjection {
  readonly nodes: readonly DesktopCanvasNode[];
  readonly edges: readonly DesktopCanvasEdge[];
  readonly totalNodeCount: number;
  readonly totalEdgeCount: number;
  readonly matchingNodeCount: number;
  readonly matchingEdgeCount: number;
  readonly truncated: boolean;
}

export function createCanvasWorldTransform(viewport: DesktopCanvasData["viewport"]): string {
  return `translate(50%, 50%) scale(${viewport.zoomPercent / 100}) translate(${-viewport.centerX}px, ${-viewport.centerY}px)`;
}

export function projectDesktopCanvasViewport(
  canvas: DesktopCanvasData,
  options: DesktopCanvasViewportProjectionOptions,
): DesktopCanvasViewportProjection {
  const scale = Math.max(0.25, canvas.viewport.zoomPercent / 100);
  const halfWidth = options.width / scale / 2;
  const halfHeight = options.height / scale / 2;
  const left = canvas.viewport.centerX - halfWidth - options.overscan;
  const right = canvas.viewport.centerX + halfWidth + options.overscan;
  const top = canvas.viewport.centerY - halfHeight - options.overscan;
  const bottom = canvas.viewport.centerY + halfHeight + options.overscan;
  const nodes: DesktopCanvasNode[] = [];
  let matchingNodeCount = 0;
  for (const node of canvas.nodes) {
    if (node.x + node.width < left || node.x > right || node.y + node.height < top || node.y > bottom) continue;
    matchingNodeCount += 1;
    if (nodes.length < options.nodeLimit) nodes.push(node);
  }
  const visibleIds = new Set(nodes.map((node) => node.nodeId));
  const edges: DesktopCanvasEdge[] = [];
  let matchingEdgeCount = 0;
  for (const edge of canvas.edges) {
    if (!visibleIds.has(edge.sourceNodeId) || !visibleIds.has(edge.targetNodeId)) continue;
    matchingEdgeCount += 1;
    if (edges.length < options.edgeLimit) edges.push(edge);
  }
  return Object.freeze({
    nodes: Object.freeze(nodes),
    edges: Object.freeze(edges),
    totalNodeCount: canvas.totalNodeCount ?? canvas.nodes.length,
    totalEdgeCount: canvas.totalEdgeCount ?? canvas.edges.length,
    matchingNodeCount: canvas.matchingNodeCount ?? matchingNodeCount,
    matchingEdgeCount: canvas.matchingEdgeCount ?? matchingEdgeCount,
    truncated: canvas.truncated === true || matchingNodeCount > nodes.length || matchingEdgeCount > edges.length,
  });
}
