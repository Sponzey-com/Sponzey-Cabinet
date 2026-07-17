use cabinet_usecases::document_diff::{
    DiffComputation, DiffLimitReason, DiffPolicy, DiffPolicyError, DocumentLineDiffService,
    DocumentTitleDelta, LineDiffKind,
};

#[test]
fn diff_policy_rejects_zero_limits_and_context_larger_than_line_limit() {
    assert_eq!(
        DiffPolicy::new(0, 0, 1, 1),
        Err(DiffPolicyError::ZeroByteLimit)
    );
    assert_eq!(
        DiffPolicy::new(0, 1, 0, 1),
        Err(DiffPolicyError::ZeroLineLimit)
    );
    assert_eq!(
        DiffPolicy::new(0, 1, 1, 0),
        Err(DiffPolicyError::ZeroHunkLimit)
    );
    assert_eq!(
        DiffPolicy::new(3, 100, 2, 1),
        Err(DiffPolicyError::ContextExceedsLineLimit)
    );
    assert_eq!(
        DiffPolicyError::ZeroByteLimit.code(),
        "document.diff_policy_invalid"
    );

    let defaults = DiffPolicy::default();
    assert_eq!(defaults.context_lines(), 3);
    assert_eq!(defaults.sync_max_bytes(), 2 * 1024 * 1024);
    assert_eq!(defaults.sync_max_lines(), 100_000);
    assert_eq!(defaults.max_hunks(), 10_000);
}

#[test]
fn byte_and_line_limits_accept_exact_boundary_and_reject_one_over() {
    let byte_service =
        DocumentLineDiffService::with_policy(DiffPolicy::new(0, 4, 10, 10).expect("byte policy"));
    assert!(matches!(
        byte_service.compare("a\n", "b\n"),
        DiffComputation::Complete(_)
    ));
    assert_eq!(
        byte_service.compare("a\n", "bb\n"),
        DiffComputation::TooLarge(DiffLimitReason::Bytes)
    );

    let line_service =
        DocumentLineDiffService::with_policy(DiffPolicy::new(0, 100, 4, 10).expect("line policy"));
    assert!(matches!(
        line_service.compare("a\nb\n", "a\nc\n"),
        DiffComputation::Complete(_)
    ));
    assert_eq!(
        line_service.compare("a\nb\n", "a\nc\nd\n"),
        DiffComputation::TooLarge(DiffLimitReason::Lines)
    );
}

#[test]
fn hunk_limit_returns_typed_too_large_instead_of_empty_complete() {
    let limited = DocumentLineDiffService::with_policy(
        DiffPolicy::new(0, 1_000, 100, 1).expect("hunk policy"),
    );
    let exact = DocumentLineDiffService::with_policy(
        DiffPolicy::new(0, 1_000, 100, 2).expect("exact hunk policy"),
    );
    let left = "a\nold 1\nb\nold 2\nc\n";
    let right = "a\nnew 1\nb\nnew 2\nc\n";

    assert_eq!(
        limited.compare(left, right),
        DiffComputation::TooLarge(DiffLimitReason::Hunks)
    );
    assert!(matches!(
        exact.compare(left, right),
        DiffComputation::Complete(_)
    ));
}

#[test]
fn title_delta_uses_first_physical_markdown_line_and_is_separate_from_hunks() {
    let changed =
        complete(service(1).compare("# 이전 제목\n같은 본문\n", "# 새 제목\n같은 본문\n"));
    assert_eq!(
        changed.title_delta(),
        &DocumentTitleDelta::Changed {
            before: "이전 제목".to_string(),
            after: "새 제목".to_string(),
        }
    );
    assert_eq!(changed.hunks().len(), 1);

    let same_display = complete(service(1).compare("# 같은 제목 #\n본문\n", "같은 제목\n본문\n"));
    assert_eq!(same_display.title_delta(), &DocumentTitleDelta::Unchanged);
    assert_eq!(same_display.hunks().len(), 1);

    let fallback = complete(service(1).compare("# !!!\r\n본문\r\n", "\r\n본문\r\n"));
    assert_eq!(fallback.title_delta(), &DocumentTitleDelta::Unchanged);
}

#[test]
fn middle_insertion_preserves_sequence_and_one_based_coordinates() {
    let result = complete(service(1).compare(
        "첫 줄\n둘째 줄\n셋째 줄\n",
        "첫 줄\n새 줄\n둘째 줄\n셋째 줄\n",
    ));

    assert_eq!(result.added_count(), 1);
    assert_eq!(result.removed_count(), 0);
    assert_eq!(result.hunks().len(), 1);
    assert_eq!(
        result
            .lines()
            .iter()
            .map(|line| {
                (
                    line.kind(),
                    line.text(),
                    line.old_line_number(),
                    line.new_line_number(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (LineDiffKind::Equal, "첫 줄", Some(1), Some(1)),
            (LineDiffKind::Added, "새 줄", None, Some(2)),
            (LineDiffKind::Equal, "둘째 줄", Some(2), Some(3)),
            (LineDiffKind::Equal, "셋째 줄", Some(3), Some(4)),
        ]
    );

    let hunk = &result.hunks()[0];
    assert_eq!(hunk.old_start_line(), 1);
    assert_eq!(hunk.new_start_line(), 1);
    assert_eq!(hunk.added_count(), 1);
    assert_eq!(hunk.removed_count(), 0);
    assert_eq!(hunk.lines().len(), 3);
}

#[test]
fn deletion_and_replacement_create_deterministic_separate_hunks() {
    let service = service(1);
    let left = "head\ndelete me\nstable 1\nstable 2\nstable 3\nold tail\nend\n";
    let right = "head\nstable 1\nstable 2\nstable 3\nnew tail\nend\n";

    let first = complete(service.compare(left, right));
    let second = complete(service.compare(left, right));

    assert_eq!(first, second);
    assert_eq!(first.added_count(), 1);
    assert_eq!(first.removed_count(), 2);
    assert_eq!(first.hunks().len(), 2);
    assert!(first.lines().iter().any(|line| {
        line.kind() == LineDiffKind::Removed
            && line.text() == "delete me"
            && line.old_line_number() == Some(2)
            && line.new_line_number().is_none()
    }));
    assert!(first.lines().iter().any(|line| {
        line.kind() == LineDiffKind::Added
            && line.text() == "new tail"
            && line.old_line_number().is_none()
            && line.new_line_number() == Some(5)
    }));
}

#[test]
fn line_endings_are_normalized_and_trailing_newline_is_explicit_metadata() {
    let service = service(2);
    let normalized = complete(service.compare("제목\r\n본문\r\n", "제목\n본문\n"));

    assert_eq!(normalized.added_count(), 0);
    assert_eq!(normalized.removed_count(), 0);
    assert!(normalized.hunks().is_empty());
    assert!(normalized.old_ends_with_newline());
    assert!(normalized.new_ends_with_newline());

    let newline_only = complete(service.compare("제목\n본문", "제목\n본문\n"));
    assert_eq!(newline_only.lines().len(), 2);
    assert!(newline_only.hunks().is_empty());
    assert!(!newline_only.old_ends_with_newline());
    assert!(newline_only.new_ends_with_newline());

    let empty = complete(service.compare("", ""));
    assert!(empty.lines().is_empty());
    assert!(empty.hunks().is_empty());
    assert!(!empty.old_ends_with_newline());
    assert!(!empty.new_ends_with_newline());
}

#[test]
fn context_size_controls_hunk_content_without_changing_full_sequence() {
    let left = "a\nb\nc\nd\ne\n";
    let right = "a\nb changed\nc\nd\ne\n";
    let no_context = complete(service(0).compare(left, right));
    let one_context = complete(service(1).compare(left, right));

    assert_eq!(no_context.lines(), one_context.lines());
    assert_eq!(no_context.hunks()[0].lines().len(), 2);
    assert_eq!(one_context.hunks()[0].lines().len(), 4);
}

#[test]
fn applying_diff_operations_reconstructs_every_small_target_sequence() {
    let service = service(1);
    let sequences = small_sequences();

    for left in &sequences {
        for right in &sequences {
            let left_body = body(left, true);
            let right_body = body(right, true);
            let result = complete(service.compare(&left_body, &right_body));
            let mut reconstructed = result
                .lines()
                .iter()
                .filter(|line| line.kind() != LineDiffKind::Removed)
                .map(|line| line.text())
                .collect::<Vec<_>>()
                .join("\n");
            if result.new_ends_with_newline() && !reconstructed.is_empty() {
                reconstructed.push('\n');
            }

            assert_eq!(reconstructed, right_body, "left={left:?} right={right:?}");
        }
    }
}

fn service(context_lines: usize) -> DocumentLineDiffService {
    DocumentLineDiffService::with_policy(
        DiffPolicy::new(context_lines, 10_000, 1_000, 100).expect("test policy"),
    )
}

fn complete(computation: DiffComputation) -> cabinet_usecases::document_diff::DocumentDiffResult {
    match computation {
        DiffComputation::Complete(result) => result,
        DiffComputation::TooLarge(reason) => panic!("unexpected limit: {reason:?}"),
    }
}

fn small_sequences() -> Vec<Vec<&'static str>> {
    let mut sequences = vec![Vec::new()];
    for length in 1..=3 {
        for mask in 0..(1 << length) {
            sequences.push(
                (0..length)
                    .map(|index| {
                        if mask & (1 << index) == 0 {
                            "가"
                        } else {
                            "나"
                        }
                    })
                    .collect(),
            );
        }
    }
    sequences
}

fn body(lines: &[&str], trailing_newline: bool) -> String {
    let mut body = lines.join("\n");
    if trailing_newline && !lines.is_empty() {
        body.push('\n');
    }
    body
}
