use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Mutable state accumulated while walking an NDJSON run trace.
pub struct RenderState {
    pub lines: Vec<String>,
    pub seen: HashSet<String>,
    pub turn_requests: HashMap<String, Value>,
    pub raw_blocks: Vec<String>,
    pub run_id: String,
}

impl RenderState {
    pub fn new(run_id: String) -> Self {
        Self {
            lines: Vec::new(),
            seen: HashSet::new(),
            turn_requests: HashMap::new(),
            raw_blocks: Vec::new(),
            run_id,
        }
    }

    /// Push `text` only if it has not already been rendered. Keeps the
    /// transcript free of duplicate lines when the same event repeats.
    pub fn push_unique(&mut self, text: String) {
        if self.seen.insert(text.clone()) {
            self.lines.push(text);
        }
    }
}
