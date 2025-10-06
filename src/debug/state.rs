use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_new() {
        let state = SessionState::new();
        assert!(matches!(state.state, DebugState::NotStarted));
        assert!(state.breakpoints.is_empty());
        assert!(state.threads.is_empty());
    }

    #[test]
    fn test_set_state() {
        let mut state = SessionState::new();
        state.set_state(DebugState::Running);
        assert!(matches!(state.state, DebugState::Running));
    }

    #[test]
    fn test_add_breakpoint() {
        let mut state = SessionState::new();
        state.add_breakpoint("test.py".to_string(), 10);
        
        let bps = state.get_breakpoints("test.py");
        assert_eq!(bps.len(), 1);
        assert_eq!(bps[0].line, 10);
        assert!(!bps[0].verified);
    }

    #[test]
    fn test_update_breakpoint() {
        let mut state = SessionState::new();
        state.add_breakpoint("test.py".to_string(), 10);
        state.update_breakpoint("test.py", 10, 1, true);
        
        let bps = state.get_breakpoints("test.py");
        assert_eq!(bps[0].id, Some(1));
        assert!(bps[0].verified);
    }

    #[test]
    fn test_add_thread() {
        let mut state = SessionState::new();
        state.add_thread(1);
        state.add_thread(2);
        state.add_thread(1); // Duplicate should not be added
        
        assert_eq!(state.threads.len(), 2);
        assert!(state.threads.contains(&1));
        assert!(state.threads.contains(&2));
    }

    #[test]
    fn test_get_breakpoints_empty() {
        let state = SessionState::new();
        let bps = state.get_breakpoints("nonexistent.py");
        assert!(bps.is_empty());
    }

    #[test]
    fn test_debug_state_stopped() {
        let state = DebugState::Stopped {
            thread_id: 1,
            reason: "breakpoint".to_string(),
        };
        
        if let DebugState::Stopped { thread_id, reason } = state {
            assert_eq!(thread_id, 1);
            assert_eq!(reason, "breakpoint");
        } else {
            panic!("Expected Stopped state");
        }
    }
}
