use std::collections::HashMap;

use crate::jupyter::response::Response;

use crate::jupyter::handlers::Handler;

// Returns a hashmap of {msg_type: count} for all messages handled by an Action
// Primarily used in tests and introspective click-testing
#[derive(Debug, Clone)]
pub struct MessageCountHandler {
    pub counts: HashMap<String, usize>,
}

impl Default for MessageCountHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageCountHandler {
    pub fn new() -> Self {
        MessageCountHandler {
            counts: HashMap::new(),
        }
    }
}

#[async_trait::async_trait]
impl Handler for MessageCountHandler {
    async fn handle(&mut self, msg: &Response) {
        let msg_type = msg.msg_type();
        let count = self.counts.entry(msg_type).or_insert(0);
        *count += 1;
    }
}
