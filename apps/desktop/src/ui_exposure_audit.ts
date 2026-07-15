export type UiExposureIssueCode =
  | "IDENTITY_EXPOSED"
  | "ERROR_CODE_EXPOSED"
  | "ABSOLUTE_PATH_EXPOSED"
  | "ENGLISH_COPY_EXPOSED";

export interface UiExposureIssue {
  readonly code: UiExposureIssueCode;
}

const RULES: readonly { readonly code: UiExposureIssueCode; readonly pattern: RegExp }[] = Object.freeze([
  { code: "IDENTITY_EXPOSED", pattern: /\b(?:workspace|document|doc|canvas|asset|version|operation|package)-[a-z0-9][a-z0-9_-]*\b/i },
  { code: "ERROR_CODE_EXPOSED", pattern: /\b[A-Z][A-Z0-9]+(?:_[A-Z0-9]+){2,}\b/ },
  { code: "ABSOLUTE_PATH_EXPOSED", pattern: /(?:\/(?:Users|home|private|var)\/|[A-Z]:\\|file:\/\/)/i },
  { code: "ENGLISH_COPY_EXPOSED", pattern: /\b(?:Retry|Save failed|Read-only recovery|Unsaved changes|Loading documents|Confirm restore|Cancel restore|Create backup|Load history)\b/i },
]);

export function auditUserExposedMarkup(markup: string): readonly UiExposureIssue[] {
  const text = decodeEntities(markup.replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, " ").replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, " ").replace(/<[^>]+>/g, " "));
  const accessible = [...markup.matchAll(/\b(?:aria-label|title|placeholder)=(?:"([^"]*)"|'([^']*)')/gi)]
    .map((match) => match[1] ?? match[2] ?? "")
    .join(" ");
  const surface = `${text} ${decodeEntities(accessible)}`;
  return Object.freeze(RULES.filter((rule) => rule.pattern.test(surface)).map((rule) => Object.freeze({ code: rule.code })));
}

function decodeEntities(value: string): string {
  return value.replace(/&quot;/g, '"').replace(/&#x27;|&#39;/g, "'").replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&");
}
