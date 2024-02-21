use crate::jupyter::handlers::Handler;
use crate::jupyter::response::Response;

// dbg!'s all messages handled by an Action
// Primarily used in introspective click-testing
#[derive(Debug)]
pub struct DebugHandler;

impl Default for DebugHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugHandler {
    pub fn new() -> Self {
        DebugHandler {}
    }
}

#[async_trait::async_trait]
impl Handler for DebugHandler {
    async fn handle(&mut self, msg: &Response) {
        dbg!(msg);
    }
}
