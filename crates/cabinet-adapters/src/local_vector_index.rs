use std::cmp::Reverse;
use std::collections::HashMap;

use cabinet_ports::embedding::{
    EmbeddingVector, VectorIndexEntry, VectorIndexError, VectorIndexPort, VectorSearchQuery,
    VectorSearchResult,
};

#[derive(Debug, Default)]
pub struct LocalVectorIndex {
    entries: HashMap<String, EmbeddingVector>,
}

impl VectorIndexPort for LocalVectorIndex {
    fn upsert_vector(&mut self, entry: VectorIndexEntry) -> Result<(), VectorIndexError> {
        self.entries.insert(
            entry.vector().source_id().as_str().to_string(),
            entry.vector().clone(),
        );
        Ok(())
    }

    fn search_similar(
        &self,
        query: VectorSearchQuery,
    ) -> Result<Vec<VectorSearchResult>, VectorIndexError> {
        let mut results = self
            .entries
            .values()
            .filter(|vector| query.source_kinds().contains(&vector.source_kind()))
            .map(|vector| {
                VectorSearchResult::new(
                    vector.source_id().clone(),
                    vector.source_kind(),
                    vector.vector_reference().clone(),
                    dot(query.query_vector().values(), vector.values()),
                )
            })
            .collect::<Vec<_>>();

        results.sort_by_key(|result| {
            (
                Reverse(result.score()),
                result.source_kind() as u8,
                result.source_id().as_str().to_string(),
            )
        });
        results.truncate(query.limit());
        Ok(results)
    }
}

fn dot(left: &[i32], right: &[i32]) -> i64 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| i64::from(*left) * i64::from(*right))
        .sum()
}
