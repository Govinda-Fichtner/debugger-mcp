/// Fake DAP Adapter for Integration Testing
///
/// This is a minimal DAP adapter implementation that responds to DAP protocol
/// requests for testing purposes. It doesn't actually debug anything, but
/// simulates the protocol correctly.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

#[derive(Debug, serde::Deserialize)]
struct Message {
    seq: Option<i32>,
    #[serde(rename = "type")]
    msg_type: String,
    command: Option<String>,
    arguments: Option<Value>,
}

struct FakeDapAdapter {
    seq: i32,
    breakpoints: HashMap<String, Vec<i32>>,
}

impl FakeDapAdapter {
    fn new() -> Self {
        Self {
            seq: 1,
            breakpoints: HashMap::new(),
        }
    }

    fn send_response(&mut self, request_seq: i32, command: &str, success: bool, body: Option<Value>) {
        let response = json!({
            "seq": self.seq,
            "type": "response",
            "request_seq": request_seq,
            "command": command,
            "success": success,
            "body": body
        });

        self.seq += 1;
        self.write_message(&response);
    }

    fn send_event(&mut self, event: &str, body: Option<Value>) {
        let event_msg = json!({
            "seq": self.seq,
            "type": "event",
            "event": event,
            "body": body
        });

        self.seq += 1;
        self.write_message(&event_msg);
    }

    fn write_message(&self, msg: &Value) {
        let content = serde_json::to_string(msg).unwrap();
        let headers = format!("Content-Length: {}\r\n\r\n", content.len());

        print!("{}{}", headers, content);
        io::stdout().flush().unwrap();
    }

    fn handle_initialize(&mut self, request_seq: i32) {
        let capabilities = json!({
            "supportsConfigurationDoneRequest": true,
            "supportsFunctionBreakpoints": false,
            "supportsConditionalBreakpoints": true,
            "supportsHitConditionalBreakpoints": false,
            "supportsEvaluateForHovers": true,
            "supportsStepBack": false,
            "supportsSetVariable": true,
            "supportsRestartFrame": false,
            "supportsGotoTargetsRequest": false,
            "supportsStepInTargetsRequest": false,
            "supportsCompletionsRequest": true,
            "supportsModulesRequest": false,
            "supportsRestartRequest": false,
            "supportsExceptionOptions": false,
            "supportsValueFormattingOptions": true,
            "supportsExceptionInfoRequest": true,
            "supportTerminateDebuggee": true,
            "supportsDelayedStackTraceLoading": true,
            "supportsLoadedSourcesRequest": false,
            "supportsLogPoints": true,
            "supportsTerminateThreadsRequest": false,
            "supportsSetExpression": true,
            "supportsTerminateRequest": true,
            "supportsDataBreakpoints": false,
            "supportsReadMemoryRequest": false,
            "supportsDisassembleRequest": false,
            "supportsCancelRequest": false,
            "supportsBreakpointLocationsRequest": false,
            "supportsClipboardContext": false
        });

        self.send_response(request_seq, "initialize", true, Some(capabilities));
        self.send_event("initialized", None);
    }

    fn handle_launch(&mut self, request_seq: i32, _args: Option<Value>) {
        self.send_response(request_seq, "launch", true, None);

        // Send a thread event to indicate the process started
        self.send_event("thread", Some(json!({
            "reason": "started",
            "threadId": 1
        })));
    }

    fn handle_set_breakpoints(&mut self, request_seq: i32, args: Option<Value>) {
        if let Some(args) = args {
            let source_path = args.get("source")
                .and_then(|s| s.get("path"))
                .and_then(|p| p.as_str())
                .unwrap_or("unknown");

            let breakpoints = args.get("breakpoints")
                .and_then(|b| b.as_array())
                .map(|arr| {
                    arr.iter()
                        .enumerate()
                        .map(|(id, bp)| {
                            let line = bp.get("line").and_then(|l| l.as_i64()).unwrap_or(0) as i32;

                            // Store breakpoint
                            self.breakpoints
                                .entry(source_path.to_string())
                                .or_insert_with(Vec::new)
                                .push(line);

                            json!({
                                "id": id + 1,
                                "verified": true,
                                "line": line
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            self.send_response(request_seq, "setBreakpoints", true, Some(json!({
                "breakpoints": breakpoints
            })));
        } else {
            self.send_response(request_seq, "setBreakpoints", false, None);
        }
    }

    fn handle_configuration_done(&mut self, request_seq: i32) {
        self.send_response(request_seq, "configurationDone", true, None);
    }

    fn handle_continue(&mut self, request_seq: i32, _args: Option<Value>) {
        self.send_response(request_seq, "continue", true, Some(json!({
            "allThreadsContinued": true
        })));

        // Simulate hitting a breakpoint
        self.send_event("stopped", Some(json!({
            "reason": "breakpoint",
            "threadId": 1,
            "allThreadsStopped": true
        })));
    }

    fn handle_stack_trace(&mut self, request_seq: i32, _args: Option<Value>) {
        let stack_frames = vec![
            json!({
                "id": 1,
                "name": "main",
                "source": {
                    "name": "test.py",
                    "path": "/test/test.py"
                },
                "line": 10,
                "column": 1
            }),
            json!({
                "id": 2,
                "name": "<module>",
                "source": {
                    "name": "test.py",
                    "path": "/test/test.py"
                },
                "line": 1,
                "column": 1
            })
        ];

        self.send_response(request_seq, "stackTrace", true, Some(json!({
            "stackFrames": stack_frames,
            "totalFrames": 2
        })));
    }

    fn handle_evaluate(&mut self, request_seq: i32, args: Option<Value>) {
        if let Some(args) = args {
            let expression = args.get("expression")
                .and_then(|e| e.as_str())
                .unwrap_or("");

            // Simple evaluation - just return the expression with a fake result
            let result = match expression {
                "x" => "42",
                "y" => "10",
                _ => "None"
            };

            self.send_response(request_seq, "evaluate", true, Some(json!({
                "result": result,
                "type": "int",
                "variablesReference": 0
            })));
        } else {
            self.send_response(request_seq, "evaluate", false, None);
        }
    }

    fn handle_disconnect(&mut self, request_seq: i32, _args: Option<Value>) {
        self.send_response(request_seq, "disconnect", true, None);
        self.send_event("terminated", None);
        self.send_event("exited", Some(json!({
            "exitCode": 0
        })));
    }

    fn handle_request(&mut self, msg: Message) {
        let request_seq = msg.seq.unwrap_or(0);
        let command = msg.command.as_deref().unwrap_or("");

        match command {
            "initialize" => self.handle_initialize(request_seq),
            "launch" => self.handle_launch(request_seq, msg.arguments),
            "setBreakpoints" => self.handle_set_breakpoints(request_seq, msg.arguments),
            "configurationDone" => self.handle_configuration_done(request_seq),
            "continue" => self.handle_continue(request_seq, msg.arguments),
            "stackTrace" => self.handle_stack_trace(request_seq, msg.arguments),
            "evaluate" => self.handle_evaluate(request_seq, msg.arguments),
            "disconnect" | "terminate" => self.handle_disconnect(request_seq, msg.arguments),
            _ => {
                eprintln!("Unknown command: {}", command);
                self.send_response(request_seq, command, false, None);
            }
        }
    }

    fn run(&mut self) {
        let stdin = io::stdin();
        let mut reader = stdin.lock();

        loop {
            // Read Content-Length header
            let mut headers = String::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 {
                    return; // EOF
                }

                if line == "\r\n" || line == "\n" {
                    break;
                }

                headers.push_str(&line);
            }

            // Parse Content-Length
            let content_length: usize = headers
                .lines()
                .find(|line| line.starts_with("Content-Length:"))
                .and_then(|line| line.split(':').nth(1))
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);

            if content_length == 0 {
                continue;
            }

            // Read content
            let mut buffer = vec![0u8; content_length];
            io::Read::read_exact(&mut reader, &mut buffer).unwrap();

            let content = String::from_utf8(buffer).unwrap();

            // Parse message
            if let Ok(msg) = serde_json::from_str::<Message>(&content) {
                if msg.msg_type == "request" {
                    self.handle_request(msg);
                }
            }
        }
    }
}

fn main() {
    let mut adapter = FakeDapAdapter::new();
    adapter.run();
}
