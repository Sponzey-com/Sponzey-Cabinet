use cabinet_domain::document_revision::{
    DocumentExpectedCurrentVersion, DocumentMutationFingerprint, DocumentMutationKind,
};
use cabinet_ports::document_mutation_fingerprint::{
    DocumentMutationFingerprintInput, DocumentMutationFingerprintPort,
    DocumentMutationFingerprintPortError,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalDocumentMutationFingerprint;

impl LocalDocumentMutationFingerprint {
    pub const fn new() -> Self {
        Self
    }
}

impl DocumentMutationFingerprintPort for LocalDocumentMutationFingerprint {
    fn fingerprint(
        &self,
        input: &DocumentMutationFingerprintInput,
    ) -> Result<DocumentMutationFingerprint, DocumentMutationFingerprintPortError> {
        let canonical = CanonicalMutation::from_input(input);
        let encoded = serde_json::to_vec(&canonical)
            .map_err(|_| DocumentMutationFingerprintPortError::GenerationUnavailable)?;
        let digest = Sha256::digest(encoded);
        let mut value = String::with_capacity(71);
        value.push_str("sha256:");
        for byte in digest {
            value.push(hex_digit(byte >> 4));
            value.push(hex_digit(byte & 0x0f));
        }
        DocumentMutationFingerprint::new(&value)
            .map_err(|_| DocumentMutationFingerprintPortError::GenerationUnavailable)
    }
}

#[derive(Serialize)]
struct CanonicalMutation {
    schema: u8,
    mutation_kind: &'static str,
    workspace_id: String,
    document_id: String,
    expected_current_kind: &'static str,
    expected_current_version: Option<String>,
    body: String,
    author: String,
    summary: String,
    attachment_state: &'static str,
    attachments: Vec<CanonicalAttachment>,
}

#[derive(Serialize)]
struct CanonicalAttachment {
    asset_id: String,
    label: String,
}

impl CanonicalMutation {
    fn from_input(input: &DocumentMutationFingerprintInput) -> Self {
        let (expected_current_kind, expected_current_version) = match input.expected_current() {
            DocumentExpectedCurrentVersion::MustNotExist => ("must_not_exist", None),
            DocumentExpectedCurrentVersion::MustMatch(version_id) => {
                ("must_match", Some(version_id.as_str().to_string()))
            }
        };
        let (attachment_state, attachments) = match input.attachment_state().references() {
            Some(references) => (
                "known",
                references
                    .iter()
                    .map(|reference| CanonicalAttachment {
                        asset_id: reference.asset_id().as_str().to_string(),
                        label: reference.label().to_string(),
                    })
                    .collect(),
            ),
            None => ("legacy_unknown", Vec::new()),
        };
        Self {
            schema: 1,
            mutation_kind: mutation_kind_name(input.kind()),
            workspace_id: input.workspace_id().as_str().to_string(),
            document_id: input.document_id().as_str().to_string(),
            expected_current_kind,
            expected_current_version,
            body: input.body().as_str().to_string(),
            author: input.author().as_str().to_string(),
            summary: input.summary().as_str().to_string(),
            attachment_state,
            attachments,
        }
    }
}

const fn mutation_kind_name(kind: DocumentMutationKind) -> &'static str {
    match kind {
        DocumentMutationKind::Create => "create",
        DocumentMutationKind::Update => "update",
        DocumentMutationKind::AttachAsset => "attach_asset",
        DocumentMutationKind::LinkAsset => "link_asset",
        DocumentMutationKind::UnlinkAsset => "unlink_asset",
        DocumentMutationKind::Restore => "restore",
    }
}

const fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        _ => (b'a' + value - 10) as char,
    }
}
