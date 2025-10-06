use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugState {
    NotStarted,
    Initializing,
    Initialized,
    Launching,
    Running,
    Stopped { thread_id: i32, reason: String },
    Terminated,
    Failed { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub source_path: String,
    pub line: i32,
    pub id: Option<i32>,
    pub verified: bool,
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub state: DebugState,
    pub breakpoints: HashMap<String, Vec<Breakpoint>>,
    pub threads: Vec<i32>,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            state: DebugState::NotStarted,
            breakpoints: HashMap::new(),
            threads: Vec::new(),
        }
    }

    pub fn set_state(&mut self, state: DebugState) {
        self.state = state;
    }

    pub fn add_breakpoint(&mut self, source: String, line: i32) {
        let bp = Breakpoint {
            source_path: source.clone(),
            line,
            id: None,
            verified: false,
        };
        
        self.breakpoints
            .entry(source)
            .or_insert_with(Vec::new)
            .push(bp);
    }

    pub fn update_breakpoint(&mut self, source: &str, line: i32, id: i32, verified: bool) {
        if let Some(bps) = self.breakpoints.get_mut(source) {
            if let Some(bp) = bps.iter_mut().find(|b| b.line == line) {
                bp.id = Some(id);
                bp.verified = verified;
            }
        }
    }

    pub fn get_breakpoints(&self, source: &str) -> Vec<Breakpoint> {
        self.breakpoints
            .get(source)
            .cloned()
            .unwrap_or_default()
    }

    pub fn add_thread(&mut self, thread_id: i32) {
        if !self.threads.contains(&thread_id) {
            self.threads.push(thread_id);
        }
    }
}
