use std::cmp::Reverse;
use std::collections::HashMap;

use cabinet_domain::retrieval::{RetrievalSourceId, RetrievalSourceKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    source_id: RetrievalSourceId,
    source_kind: RetrievalSourceKind,
    score: u32,
}

impl SearchMatch {
    pub fn new(
        source_id: RetrievalSourceId,
        source_kind: RetrievalSourceKind,
        score: u32,
    ) -> Result<Self, HybridSearchError> {
        if score == 0 {
            return Err(HybridSearchError::InvalidScore);
        }
        Ok(Self {
            source_id,
            source_kind,
            score,
        })
    }

    pub fn source_id(&self) -> &RetrievalSourceId {
        &self.source_id
    }

    pub const fn source_kind(&self) -> RetrievalSourceKind {
        self.source_kind
    }

    pub const fn score(&self) -> u32 {
        self.score
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridSearchInput {
    keyword_matches: Vec<SearchMatch>,
    semantic_matches: Vec<SearchMatch>,
    limit: usize,
}

impl HybridSearchInput {
    pub fn new(
        keyword_matches: Vec<SearchMatch>,
        semantic_matches: Vec<SearchMatch>,
        limit: usize,
    ) -> Result<Self, HybridSearchError> {
        if keyword_matches.is_empty() && semantic_matches.is_empty() {
            return Err(HybridSearchError::EmptyInput);
        }
        if limit == 0 {
            return Err(HybridSearchError::InvalidLimit);
        }
        Ok(Self {
            keyword_matches,
            semantic_matches,
            limit,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridSearchOutput {
    results: Vec<HybridSearchResult>,
}

impl HybridSearchOutput {
    pub fn results(&self) -> &[HybridSearchResult] {
        &self.results
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridSearchResult {
    source_id: RetrievalSourceId,
    source_kind: RetrievalSourceKind,
    combined_score: u32,
    keyword_hit: bool,
    semantic_hit: bool,
}

impl HybridSearchResult {
    pub fn source_id(&self) -> &RetrievalSourceId {
        &self.source_id
    }

    pub const fn source_kind(&self) -> RetrievalSourceKind {
        self.source_kind
    }

    pub const fn combined_score(&self) -> u32 {
        self.combined_score
    }

    pub const fn keyword_hit(&self) -> bool {
        self.keyword_hit
    }

    pub const fn semantic_hit(&self) -> bool {
        self.semantic_hit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergeHybridSearchUsecase;

impl MergeHybridSearchUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: HybridSearchInput,
    ) -> Result<HybridSearchOutput, HybridSearchError> {
        let mut results: HashMap<String, HybridSearchResult> = HashMap::new();
        for keyword_match in input.keyword_matches {
            merge_match(&mut results, keyword_match, true);
        }
        for semantic_match in input.semantic_matches {
            merge_match(&mut results, semantic_match, false);
        }

        let mut results = results.into_values().collect::<Vec<_>>();
        results.sort_by_key(|result| {
            (
                Reverse(result.combined_score()),
                result.source_kind() as u8,
                result.source_id().as_str().to_string(),
            )
        });
        results.truncate(input.limit);
        Ok(HybridSearchOutput { results })
    }
}

impl Default for MergeHybridSearchUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridSearchError {
    EmptyInput,
    InvalidLimit,
    InvalidScore,
}

impl HybridSearchError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptyInput => "hybrid_search.empty_input",
            Self::InvalidLimit => "hybrid_search.invalid_limit",
            Self::InvalidScore => "hybrid_search.invalid_score",
        }
    }
}

fn merge_match(
    results: &mut HashMap<String, HybridSearchResult>,
    entry: SearchMatch,
    keyword: bool,
) {
    let key = entry.source_id().as_str().to_string();
    results
        .entry(key)
        .and_modify(|result| {
            result.combined_score = result.combined_score.saturating_add(entry.score());
            if keyword {
                result.keyword_hit = true;
            } else {
                result.semantic_hit = true;
            }
        })
        .or_insert_with(|| HybridSearchResult {
            source_id: entry.source_id,
            source_kind: entry.source_kind,
            combined_score: entry.score,
            keyword_hit: keyword,
            semantic_hit: !keyword,
        });
}
