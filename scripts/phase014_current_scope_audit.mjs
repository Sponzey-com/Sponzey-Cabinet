const DOMAIN_EXTERNAL_IO = /\b(?:std::(?:fs|env|net)|rusqlite|reqwest|sqlx|diesel|tauri|tokio|axum|actix)\b/;
const USECASE_EXTERNAL_IO = /\b(?:std::(?:fs|net)|rusqlite|reqwest|sqlx|diesel|tauri|tokio::net|TcpStream|OpenOptions|File::open)\b/;
const ENVIRONMENT_MUTATION = /\b(?:std::env::(?:set_var|remove_var)|process\.env\s*[.[][^\n=]*=)/;
const ENVIRONMENT_REQUERY = /\b(?:std::env::(?:var|var_os|vars|vars_os)|process\.env\b)/;
const RELEASE_SENSITIVE = /(?:\/(?:Users|home|private|tmp|var)\/|[A-Za-z]:\\Users\\|\b(?:documentBody|assetBytes|sessionToken|provider_api_key|apiKey|secret|password|token|body|content|absolutePath)=|# Packaged Workflow|Durable readback marker)/i;
const DESKTOP_FIELD_DEBUG_ACTIVATION = /\b(?:field_debug|fieldDebug)(?:Session|Enabled|Active|activate|approve)/;

export function auditCurrentScopeSources({ domainSources, usecaseSources, runtimeSources, releaseTexts }) {
  const findingIds = [];
  for (const source of domainSources) {
    if (DOMAIN_EXTERNAL_IO.test(source.text)) findingIds.push("domain_external_io");
  }
  for (const source of usecaseSources) {
    if (USECASE_EXTERNAL_IO.test(source.text)) findingIds.push("usecase_external_io");
  }
  for (const source of runtimeSources) {
    if (ENVIRONMENT_MUTATION.test(source.text)) findingIds.push("runtime_environment_mutation");
    if (ENVIRONMENT_REQUERY.test(source.text)) findingIds.push("runtime_environment_requery");
    if (DESKTOP_FIELD_DEBUG_ACTIVATION.test(source.text)) findingIds.push("desktop_field_debug_activation");
  }
  for (const source of releaseTexts) {
    if (RELEASE_SENSITIVE.test(source.text)) findingIds.push("release_sensitive_content");
  }
  const unique = [...new Set(findingIds)];
  return Object.freeze({
    passed: unique.length === 0,
    findingIds: Object.freeze(unique),
    scannedFileCount: domainSources.length + usecaseSources.length + runtimeSources.length + releaseTexts.length,
  });
}
