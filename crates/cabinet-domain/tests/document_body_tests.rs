use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentError};

#[test]
fn document_body_normalizes_crlf_and_cr_line_endings_to_lf() {
    let policy = DocumentBodyPolicy::new(128).expect("policy should be valid");

    let body = DocumentBody::new("first\r\nsecond\rthird", policy).expect("body should be valid");

    assert_eq!(body.as_str(), "first\nsecond\nthird");
}

#[test]
fn document_body_rejects_content_larger_than_explicit_policy() {
    let policy = DocumentBodyPolicy::new(8).expect("policy should be valid");

    let error = DocumentBody::new("123456789", policy).expect_err("body should be too large");

    assert_eq!(error, DocumentError::BodyTooLarge { max_bytes: 8 });
}

#[test]
fn document_body_preserves_unicode_content_after_normalization() {
    let policy = DocumentBodyPolicy::new(128).expect("policy should be valid");

    let body =
        DocumentBody::new("# 안녕하세요\r\nEmoji: 🌐", policy).expect("body should be valid");

    assert_eq!(body.as_str(), "# 안녕하세요\nEmoji: 🌐");
}

#[test]
fn document_body_policy_rejects_zero_size_limit() {
    assert_eq!(
        DocumentBodyPolicy::new(0).expect_err("zero max size must fail"),
        DocumentError::InvalidBodyPolicy
    );
}
