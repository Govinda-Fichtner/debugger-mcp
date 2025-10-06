use serde::{Deserialize, Serialize};
use serde_json::Value;

/// DAP Protocol Message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    #[serde(rename = "request")]
    Request(Request),
    #[serde(rename = "response")]
    Response(Response),
    #[serde(rename = "event")]
    Event(Event),
}

/// DAP Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub seq: i32,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

/// DAP Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub seq: i32,
    pub request_seq: i32,
    pub command: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

/// DAP Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub seq: i32,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

/// Initialize Request Arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequestArguments {
    #[serde(rename = "clientID")]
    pub client_id: Option<String>,
    pub client_name: Option<String>,
    #[serde(rename = "adapterID")]
    pub adapter_id: String,
    pub locale: Option<String>,
    pub lines_start_at_1: Option<bool>,
    pub columns_start_at_1: Option<bool>,
    pub path_format: Option<String>,
}

/// Capabilities returned by initialize
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub supports_configuration_done_request: Option<bool>,
    pub supports_function_breakpoints: Option<bool>,
    pub supports_conditional_breakpoints: Option<bool>,
    pub supports_hit_conditional_breakpoints: Option<bool>,
    pub supports_evaluate_for_hovers: Option<bool>,
    pub supports_set_variable: Option<bool>,
    pub supports_restart_frame: Option<bool>,
    pub supports_step_in_targets_request: Option<bool>,
}

/// Launch Request Arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequestArguments {
    pub no_debug: Option<bool>,
    #[serde(flatten)]
    pub additional: Value,
}

/// SetBreakpoints Request Arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBreakpointsArguments {
    pub source: Source,
    pub breakpoints: Option<Vec<SourceBreakpoint>>,
    pub source_modified: Option<bool>,
}

/// Source reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub name: Option<String>,
    pub path: Option<String>,
    pub source_reference: Option<i32>,
}

/// Source breakpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBreakpoint {
    pub line: i32,
    pub column: Option<i32>,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
}

/// Breakpoint response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Breakpoint {
    pub id: Option<i32>,
    pub verified: bool,
    pub message: Option<String>,
    pub source: Option<Source>,
    pub line: Option<i32>,
    pub column: Option<i32>,
}

/// StackTrace Request Arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTraceArguments {
    pub thread_id: i32,
    pub start_frame: Option<i32>,
    pub levels: Option<i32>,
}

/// Stack Frame
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
    pub id: i32,
    pub name: String,
    pub source: Option<Source>,
    pub line: i32,
    pub column: i32,
    pub end_line: Option<i32>,
    pub end_column: Option<i32>,
}

/// Thread info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: i32,
    pub name: String,
}

/// Evaluate Request Arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateArguments {
    pub expression: String,
    pub frame_id: Option<i32>,
    pub context: Option<String>,
}

/// Variable
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub name: String,
    pub value: String,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub variables_reference: i32,
}

/// Scopes Request Arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopesArguments {
    pub frame_id: i32,
}

/// Scope
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub name: String,
    pub variables_reference: i32,
    pub expensive: bool,
}

/// Continue Request Arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueArguments {
    pub thread_id: i32,
}

/// Next (Step Over) Request Arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextArguments {
    pub thread_id: i32,
}

/// StepIn (Step Into) Request Arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepInArguments {
    pub thread_id: i32,
}

/// StepOut (Step Out) Request Arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepOutArguments {
    pub thread_id: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_serialization() {
        let req = Request {
            seq: 1,
            command: "initialize".to_string(),
            arguments: Some(json!({"clientID": "test"})),
        };
        
        let serialized = serde_json::to_string(&req).unwrap();
        assert!(serialized.contains("initialize"));
        assert!(serialized.contains("\"seq\":1"));
    }

    #[test]
    fn test_response_serialization() {
        let resp = Response {
            seq: 2,
            request_seq: 1,
            command: "initialize".to_string(),
            success: true,
            message: None,
            body: Some(json!({"capabilities": {}})),
        };
        
        let serialized = serde_json::to_string(&resp).unwrap();
        assert!(serialized.contains("\"success\":true"));
    }

    #[test]
    fn test_source_breakpoint() {
        let bp = SourceBreakpoint {
            line: 10,
            column: Some(5),
            condition: Some("x > 0".to_string()),
            hit_condition: None,
        };
        
        assert_eq!(bp.line, 10);
        assert_eq!(bp.column, Some(5));
    }

    #[test]
    fn test_stack_frame() {
        let frame = StackFrame {
            id: 1,
            name: "main".to_string(),
            source: Some(Source {
                name: Some("test.py".to_string()),
                path: Some("/path/to/test.py".to_string()),
                source_reference: None,
            }),
            line: 42,
            column: 10,
            end_line: None,
            end_column: None,
        };
        
        assert_eq!(frame.name, "main");
        assert_eq!(frame.line, 42);
    }
}
