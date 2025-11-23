use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::client::{LlmClient, LlmError, Message};

#[derive(Clone, Default)]
pub struct MockLlmClient {
    responses: Arc<Mutex<VecDeque<String>>>,
}

impl MockLlmClient {
    pub fn push_response(&self, response: impl Into<String>) {
        let mut guard = self.responses.lock().unwrap();
        guard.push_back(response.into());
    }
}

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn chat(&self, _role: &str, _messages: &[Message]) -> Result<String, LlmError> {
        let mut guard = self.responses.lock().unwrap();
        Ok(guard.pop_front().unwrap_or_else(|| "mock-response".into()))
    }
}
