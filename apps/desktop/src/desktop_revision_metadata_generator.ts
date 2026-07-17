export interface DesktopRevisionMetadataGenerator {
  next(): { readonly versionId: string; readonly snapshotRef: string };
}

export function createDesktopRevisionMetadataGenerator(
  nextId: () => string,
): DesktopRevisionMetadataGenerator {
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
