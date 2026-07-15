import type { DocumentRevisionMetadataGenerator } from "./desktop_document_authoring_controller.ts";

export function createDesktopRevisionMetadataGenerator(
  nextId: () => string,
): DocumentRevisionMetadataGenerator {
  return {
    next() {
      const id = nextId();
      return {
        versionId: `version-${id}`,
        snapshotRef: `snapshot-${id}`,
      };
    },
  };
}
