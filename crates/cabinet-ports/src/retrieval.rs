use cabinet_domain::retrieval::{RetrievalCandidate, RetrievalQuery, RetrievalScope};

pub trait RetrievalSourcePort {
    fn query_candidates(
        &self,
        query: &RetrievalQuery,
        scope: &RetrievalScope,
    ) -> Result<Vec<RetrievalCandidate>, RetrievalPortError>;
}

pub trait RetrievalPermissionPort {
    fn allows_candidate(
        &self,
        scope: &RetrievalScope,
        candidate: &RetrievalCandidate,
    ) -> Result<bool, RetrievalPortError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalPortError {
    SourceUnavailable,
    PermissionUnavailable,
}

impl RetrievalPortError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::SourceUnavailable => "retrieval_port.source_unavailable",
            Self::PermissionUnavailable => "retrieval_port.permission_unavailable",
        }
    }
}
