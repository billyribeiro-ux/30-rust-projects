use std::collections::HashMap;
use std::sync::Arc;

use crate::error::AppResult;

/// A job handler: pure async function over the payload. Returns Ok on
/// success; Err to signal "retry this".
pub type HandlerFn = Arc<
    dyn Fn(serde_json::Value) -> futures_core::future::BoxFuture<'static, AppResult<String>>
        + Send
        + Sync,
>;

#[derive(Default)]
pub struct Registry {
    handlers: HashMap<String, HandlerFn>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, kind: &str, h: HandlerFn) {
        self.handlers.insert(kind.to_string(), h);
    }

    pub fn get(&self, kind: &str) -> Option<HandlerFn> {
        self.handlers.get(kind).cloned()
    }
}
