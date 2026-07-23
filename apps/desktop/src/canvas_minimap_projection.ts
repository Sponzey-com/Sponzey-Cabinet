import type { DesktopCanvasNode, DesktopCanvasViewport } from "./tauri_canvas_transport.ts";

export const CANVAS_MINIMAP_POLICY = Object.freeze({
  width: 120,
  height: 72,
  padding: 4,
  surfaceWidth: 1_200,
  surfaceHeight: 720,
});

export interface CanvasMinimapRectangle {
  readonly left: number;
  readonly top: number;
  readonly width: number;
  readonly height: number;
}

export interface CanvasMinimapProjection {
  readonly nodes: readonly CanvasMinimapRectangle[];
  readonly viewport: CanvasMinimapRectangle;
}

interface WorldRectangle {
  readonly left: number;
  readonly top: number;
  readonly right: number;
  readonly bottom: number;
}

export function projectCanvasMinimap(
  nodes: readonly DesktopCanvasNode[],
  viewport: DesktopCanvasViewport,
): CanvasMinimapProjection {
  const visibleViewport = viewportWorldRectangle(viewport);
  const validNodes = nodes.filter(hasValidGeometry);
  const nodeBounds = validNodes.map((node) => ({
    left: node.x,
    top: node.y,
    right: node.x + node.width,
    bottom: node.y + node.height,
  }));
  const extent = paddedExtent([visibleViewport, ...nodeBounds]);
  const transform = createTransform(extent);

  return Object.freeze({
    nodes: Object.freeze(nodeBounds.map((bounds) => Object.freeze(transform(bounds)))),
    viewport: Object.freeze(transform(visibleViewport)),
  });
}

function viewportWorldRectangle(viewport: DesktopCanvasViewport): WorldRectangle {
  const zoom = Math.min(400, Math.max(25, viewport.zoomPercent)) / 100;
  const halfWidth = CANVAS_MINIMAP_POLICY.surfaceWidth / zoom / 2;
  const halfHeight = CANVAS_MINIMAP_POLICY.surfaceHeight / zoom / 2;
  return {
    left: viewport.centerX - halfWidth,
    top: viewport.centerY - halfHeight,
    right: viewport.centerX + halfWidth,
    bottom: viewport.centerY + halfHeight,
  };
}

function paddedExtent(rectangles: readonly WorldRectangle[]): WorldRectangle {
  const left = Math.min(...rectangles.map((rectangle) => rectangle.left));
  const top = Math.min(...rectangles.map((rectangle) => rectangle.top));
  const right = Math.max(...rectangles.map((rectangle) => rectangle.right));
  const bottom = Math.max(...rectangles.map((rectangle) => rectangle.bottom));
  const span = Math.max(right - left, bottom - top, 1);
  const worldPadding = Math.max(16, span * 0.04);
  return {
    left: left - worldPadding,
    top: top - worldPadding,
    right: right + worldPadding,
    bottom: bottom + worldPadding,
  };
}

function createTransform(extent: WorldRectangle): (rectangle: WorldRectangle) => CanvasMinimapRectangle {
  const availableWidth = CANVAS_MINIMAP_POLICY.width - CANVAS_MINIMAP_POLICY.padding * 2;
  const availableHeight = CANVAS_MINIMAP_POLICY.height - CANVAS_MINIMAP_POLICY.padding * 2;
  const worldWidth = Math.max(1, extent.right - extent.left);
  const worldHeight = Math.max(1, extent.bottom - extent.top);
  const scale = Math.min(availableWidth / worldWidth, availableHeight / worldHeight);
  const originX = (CANVAS_MINIMAP_POLICY.width - worldWidth * scale) / 2;
  const originY = (CANVAS_MINIMAP_POLICY.height - worldHeight * scale) / 2;

  return (rectangle) => {
    const left = originX + (rectangle.left - extent.left) * scale;
    const top = originY + (rectangle.top - extent.top) * scale;
    const right = originX + (rectangle.right - extent.left) * scale;
    const bottom = originY + (rectangle.bottom - extent.top) * scale;
    return {
      left: round(left),
      top: round(top),
      width: round(Math.max(1, right - left)),
      height: round(Math.max(1, bottom - top)),
    };
  };
}

function hasValidGeometry(node: DesktopCanvasNode): boolean {
  return Number.isFinite(node.x) && Number.isFinite(node.y)
    && Number.isFinite(node.width) && Number.isFinite(node.height)
    && node.width > 0 && node.height > 0;
}

function round(value: number): number {
  return Math.round(value * 100) / 100;
}
