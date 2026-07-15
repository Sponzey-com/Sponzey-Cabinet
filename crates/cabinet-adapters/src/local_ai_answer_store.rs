use std::collections::HashMap;

use cabinet_domain::ai::{AiAnswerJobId, AiAnswerJobState, AiAnswerResult};
use cabinet_ports::ai::{AiAnswerResultStorePort, AiAnswerStoreError};

#[derive(Debug, Default)]
pub struct LocalAiAnswerStore {
    statuses: HashMap<String, AiAnswerJobState>,
    results: HashMap<String, AiAnswerResult>,
}

impl LocalAiAnswerStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AiAnswerResultStorePort for LocalAiAnswerStore {
    fn save_status(
        &mut self,
        job_id: &AiAnswerJobId,
        state: AiAnswerJobState,
    ) -> Result<(), AiAnswerStoreError> {
        self.statuses.insert(job_id.as_str().to_string(), state);
        Ok(())
    }

    fn save_result(
        &mut self,
        job_id: &AiAnswerJobId,
        result: AiAnswerResult,
    ) -> Result<(), AiAnswerStoreError> {
        self.results.insert(job_id.as_str().to_string(), result);
        Ok(())
    }

    fn get_status(
        &self,
        job_id: &AiAnswerJobId,
    ) -> Result<Option<AiAnswerJobState>, AiAnswerStoreError> {
        Ok(self.statuses.get(job_id.as_str()).copied())
    }

    fn get_result(
        &self,
        job_id: &AiAnswerJobId,
    ) -> Result<Option<AiAnswerResult>, AiAnswerStoreError> {
        Ok(self.results.get(job_id.as_str()).cloned())
    }
}
