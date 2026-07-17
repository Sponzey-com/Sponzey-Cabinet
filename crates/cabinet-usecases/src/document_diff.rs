use cabinet_domain::document::DocumentTitle;
use similar::{Algorithm, ChangeTag, capture_diff_slices};

const DEFAULT_CONTEXT_LINES: usize = 3;
const DEFAULT_SYNC_MAX_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_SYNC_MAX_LINES: usize = 100_000;
const DEFAULT_MAX_HUNKS: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffPolicy {
    context_lines: usize,
    sync_max_bytes: usize,
    sync_max_lines: usize,
    max_hunks: usize,
}

impl DiffPolicy {
    pub fn new(
        context_lines: usize,
        sync_max_bytes: usize,
        sync_max_lines: usize,
        max_hunks: usize,
    ) -> Result<Self, DiffPolicyError> {
        if sync_max_bytes == 0 {
            return Err(DiffPolicyError::ZeroByteLimit);
        }
        if sync_max_lines == 0 {
            return Err(DiffPolicyError::ZeroLineLimit);
        }
        if max_hunks == 0 {
            return Err(DiffPolicyError::ZeroHunkLimit);
        }
        if context_lines > sync_max_lines {
            return Err(DiffPolicyError::ContextExceedsLineLimit);
        }
        Ok(Self {
            context_lines,
            sync_max_bytes,
            sync_max_lines,
            max_hunks,
        })
    }

    pub fn context_lines(self) -> usize {
        self.context_lines
    }

    pub fn sync_max_bytes(self) -> usize {
        self.sync_max_bytes
    }

    pub fn sync_max_lines(self) -> usize {
        self.sync_max_lines
    }

    pub fn max_hunks(self) -> usize {
        self.max_hunks
    }
}

impl Default for DiffPolicy {
    fn default() -> Self {
        Self::new(
            DEFAULT_CONTEXT_LINES,
            DEFAULT_SYNC_MAX_BYTES,
            DEFAULT_SYNC_MAX_LINES,
            DEFAULT_MAX_HUNKS,
        )
        .expect("default diff policy must be valid")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffPolicyError {
    ZeroByteLimit,
    ZeroLineLimit,
    ZeroHunkLimit,
    ContextExceedsLineLimit,
}

impl DiffPolicyError {
    pub const fn code(self) -> &'static str {
        "document.diff_policy_invalid"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLimitReason {
    Bytes,
    Lines,
    Hunks,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffComputation {
    Complete(DocumentDiffResult),
    TooLarge(DiffLimitReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentTitleDelta {
    Unchanged,
    Changed { before: String, after: String },
}

impl DiffComputation {
    pub fn complete(&self) -> Option<&DocumentDiffResult> {
        match self {
            Self::Complete(result) => Some(result),
            Self::TooLarge(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineDiffKind {
    Equal,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineDiff {
    kind: LineDiffKind,
    text: String,
    old_line_number: Option<usize>,
    new_line_number: Option<usize>,
}

impl LineDiff {
    pub fn kind(&self) -> LineDiffKind {
        self.kind
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn old_line_number(&self) -> Option<usize> {
        self.old_line_number
    }

    pub fn new_line_number(&self) -> Option<usize> {
        self.new_line_number
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    old_start_line: usize,
    new_start_line: usize,
    lines: Vec<LineDiff>,
    added_count: usize,
    removed_count: usize,
}

impl DiffHunk {
    pub fn old_start_line(&self) -> usize {
        self.old_start_line
    }

    pub fn new_start_line(&self) -> usize {
        self.new_start_line
    }

    pub fn lines(&self) -> &[LineDiff] {
        &self.lines
    }

    pub fn added_count(&self) -> usize {
        self.added_count
    }

    pub fn removed_count(&self) -> usize {
        self.removed_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentDiffResult {
    lines: Vec<LineDiff>,
    hunks: Vec<DiffHunk>,
    added_count: usize,
    removed_count: usize,
    old_ends_with_newline: bool,
    new_ends_with_newline: bool,
    title_delta: DocumentTitleDelta,
}

impl DocumentDiffResult {
    pub fn lines(&self) -> &[LineDiff] {
        &self.lines
    }

    pub fn hunks(&self) -> &[DiffHunk] {
        &self.hunks
    }

    pub fn added_count(&self) -> usize {
        self.added_count
    }

    pub fn removed_count(&self) -> usize {
        self.removed_count
    }

    pub fn old_ends_with_newline(&self) -> bool {
        self.old_ends_with_newline
    }

    pub fn new_ends_with_newline(&self) -> bool {
        self.new_ends_with_newline
    }

    pub fn title_delta(&self) -> &DocumentTitleDelta {
        &self.title_delta
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentLineDiffService {
    policy: DiffPolicy,
}

impl DocumentLineDiffService {
    pub const fn with_policy(policy: DiffPolicy) -> Self {
        Self { policy }
    }

    pub const fn policy(&self) -> DiffPolicy {
        self.policy
    }

    pub fn compare(&self, left: &str, right: &str) -> DiffComputation {
        if exceeds_limit(left.len(), right.len(), self.policy.sync_max_bytes) {
            return DiffComputation::TooLarge(DiffLimitReason::Bytes);
        }
        let title_delta = title_delta(left, right);
        let left = normalize_line_endings(left);
        let right = normalize_line_endings(right);
        let old_ends_with_newline = left.ends_with('\n');
        let new_ends_with_newline = right.ends_with('\n');
        let old_lines = physical_lines(&left);
        let new_lines = physical_lines(&right);
        if exceeds_limit(old_lines.len(), new_lines.len(), self.policy.sync_max_lines) {
            return DiffComputation::TooLarge(DiffLimitReason::Lines);
        }
        let operations = capture_diff_slices(Algorithm::Myers, &old_lines, &new_lines);
        let mut old_line_number = 1;
        let mut new_line_number = 1;
        let mut lines = Vec::new();

        for change in operations
            .iter()
            .flat_map(|operation| operation.iter_changes(&old_lines, &new_lines))
        {
            let (kind, old_number, new_number) = match change.tag() {
                ChangeTag::Equal => {
                    let coordinates = (Some(old_line_number), Some(new_line_number));
                    old_line_number += 1;
                    new_line_number += 1;
                    (LineDiffKind::Equal, coordinates.0, coordinates.1)
                }
                ChangeTag::Delete => {
                    let coordinate = Some(old_line_number);
                    old_line_number += 1;
                    (LineDiffKind::Removed, coordinate, None)
                }
                ChangeTag::Insert => {
                    let coordinate = Some(new_line_number);
                    new_line_number += 1;
                    (LineDiffKind::Added, None, coordinate)
                }
            };
            lines.push(LineDiff {
                kind,
                text: change.value().to_string(),
                old_line_number: old_number,
                new_line_number: new_number,
            });
        }

        let added_count = count_kind(&lines, LineDiffKind::Added);
        let removed_count = count_kind(&lines, LineDiffKind::Removed);
        let hunks = build_hunks(&lines, self.policy.context_lines);
        if hunks.len() > self.policy.max_hunks {
            return DiffComputation::TooLarge(DiffLimitReason::Hunks);
        }

        DiffComputation::Complete(DocumentDiffResult {
            lines,
            hunks,
            added_count,
            removed_count,
            old_ends_with_newline,
            new_ends_with_newline,
            title_delta,
        })
    }
}

impl Default for DocumentLineDiffService {
    fn default() -> Self {
        Self::with_policy(DiffPolicy::default())
    }
}

fn exceeds_limit(left: usize, right: usize, limit: usize) -> bool {
    left.checked_add(right).is_none_or(|total| total > limit)
}

fn title_delta(left: &str, right: &str) -> DocumentTitleDelta {
    let before = DocumentTitle::from_markdown_text(left);
    let after = DocumentTitle::from_markdown_text(right);
    if before == after {
        DocumentTitleDelta::Unchanged
    } else {
        DocumentTitleDelta::Changed {
            before: before.as_str().to_string(),
            after: after.as_str().to_string(),
        }
    }
}

fn normalize_line_endings(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

fn physical_lines(value: &str) -> Vec<&str> {
    if value.is_empty() {
        return Vec::new();
    }
    let mut lines = value.split('\n').collect::<Vec<_>>();
    if value.ends_with('\n') {
        lines.pop();
    }
    lines
}

fn count_kind(lines: &[LineDiff], kind: LineDiffKind) -> usize {
    lines.iter().filter(|line| line.kind == kind).count()
}

fn build_hunks(lines: &[LineDiff], context_lines: usize) -> Vec<DiffHunk> {
    let changed_indexes = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (line.kind != LineDiffKind::Equal).then_some(index))
        .collect::<Vec<_>>();
    let Some(&first_change) = changed_indexes.first() else {
        return Vec::new();
    };

    let mut ranges = Vec::new();
    let mut start = first_change.saturating_sub(context_lines);
    let mut end = usize::min(first_change + context_lines + 1, lines.len());
    for &changed_index in changed_indexes.iter().skip(1) {
        let candidate_start = changed_index.saturating_sub(context_lines);
        let candidate_end = usize::min(changed_index + context_lines + 1, lines.len());
        if candidate_start <= end {
            end = usize::max(end, candidate_end);
        } else {
            ranges.push((start, end));
            start = candidate_start;
            end = candidate_end;
        }
    }
    ranges.push((start, end));

    ranges
        .into_iter()
        .map(|(start, end)| {
            let hunk_lines = lines[start..end].to_vec();
            DiffHunk {
                old_start_line: hunk_start_line(lines, start, CoordinateSide::Old),
                new_start_line: hunk_start_line(lines, start, CoordinateSide::New),
                added_count: count_kind(&hunk_lines, LineDiffKind::Added),
                removed_count: count_kind(&hunk_lines, LineDiffKind::Removed),
                lines: hunk_lines,
            }
        })
        .collect()
}

#[derive(Clone, Copy)]
enum CoordinateSide {
    Old,
    New,
}

fn hunk_start_line(lines: &[LineDiff], start: usize, side: CoordinateSide) -> usize {
    let coordinate = |line: &LineDiff| match side {
        CoordinateSide::Old => line.old_line_number,
        CoordinateSide::New => line.new_line_number,
    };
    coordinate(&lines[start]).unwrap_or_else(|| {
        lines[..start]
            .iter()
            .rev()
            .find_map(coordinate)
            .map_or(1, |line_number| line_number + 1)
    })
}
