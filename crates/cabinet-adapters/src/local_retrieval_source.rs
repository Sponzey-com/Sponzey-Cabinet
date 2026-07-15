use cabinet_domain::retrieval::{RetrievalCandidate, RetrievalQuery, RetrievalScope};
use cabinet_ports::retrieval::{RetrievalPortError, RetrievalSourcePort};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRetrievalSourceRecord {
    searchable_text: String,
    candidate: RetrievalCandidate,
}

impl LocalRetrievalSourceRecord {
    pub fn new(
        searchable_text: &str,
        candidate: RetrievalCandidate,
    ) -> Result<Self, LocalRetrievalSourceError> {
        let searchable_text = searchable_text.trim();
        if searchable_text.is_empty() || searchable_text.chars().any(char::is_control) {
            return Err(LocalRetrievalSourceError::InvalidSearchText);
        }
        Ok(Self {
            searchable_text: searchable_text.to_ascii_lowercase(),
            candidate,
        })
    }

    pub fn candidate(&self) -> &RetrievalCandidate {
        &self.candidate
    }
}

#[derive(Debug, Default)]
pub struct LocalRetrievalSource {
    records: Vec<LocalRetrievalSourceRecord>,
}

impl LocalRetrievalSource {
    pub fn new(records: Vec<LocalRetrievalSourceRecord>) -> Self {
        Self { records }
    }
}

impl RetrievalSourcePort for LocalRetrievalSource {
    fn query_candidates(
        &self,
        query: &RetrievalQuery,
        scope: &RetrievalScope,
    ) -> Result<Vec<RetrievalCandidate>, RetrievalPortError> {
        let query_text = query.as_str().to_ascii_lowercase();
        let candidates = self
            .records
            .iter()
            .filter(|record| record.searchable_text.contains(&query_text))
            .filter(|record| {
                scope
                    .source_kinds()
                    .contains(&record.candidate().source_kind())
            })
            .map(|record| record.candidate().clone())
            .collect();
        Ok(candidates)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalRetrievalSourceError {
    InvalidSearchText,
}

impl LocalRetrievalSourceError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidSearchText => "local_retrieval_source.invalid_search_text",
        }
    }
}
