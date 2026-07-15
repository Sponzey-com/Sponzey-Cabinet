use std::cell::Cell;

use cabinet_ports::ai::{
    AiProviderError, AiProviderPolicy, AiProviderPort, AiProviderRequest, AiProviderResponse,
};

#[derive(Debug)]
pub struct FakeAiProvider {
    response: Result<AiProviderResponse, AiProviderError>,
    call_count: Cell<usize>,
}

impl FakeAiProvider {
    pub const fn new(response: Result<AiProviderResponse, AiProviderError>) -> Self {
        Self {
            response,
            call_count: Cell::new(0),
        }
    }

    pub fn call_count(&self) -> usize {
        self.call_count.get()
    }
}

impl AiProviderPort for FakeAiProvider {
    fn generate_answer(
        &self,
        _request: &AiProviderRequest,
        _policy: &AiProviderPolicy,
    ) -> Result<AiProviderResponse, AiProviderError> {
        self.call_count.set(self.call_count.get() + 1);
        self.response.clone()
    }
}
