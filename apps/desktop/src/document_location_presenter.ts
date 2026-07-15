export function presentDocumentLocation(path: string): string {
  const segments = path
    .split(/[\\/]+/)
    .map((segment) => segment.trim())
    .filter((segment) => segment.length > 0 && segment !== "." && segment !== "..");
  if (segments.length <= 1) return "문서";
  const folders = segments.slice(0, -1);
  return folders.length > 0 ? folders.join(" / ") : "문서";
}
