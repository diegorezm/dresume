//! Claude Code in headless streaming mode.
//!
//! `claude -p --input-format stream-json --output-format stream-json` reads user messages as JSON
//! lines on stdin and prints its whole transcript as JSON lines. With `--permission-prompt-tool
//! stdio`, approvals arrive as `control_request { subtype: "can_use_tool" }` and are answered with
//! a `control_response` on stdin; an interrupt is a `control_request` we send.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::Result;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::process::{self, Proc};
use crate::{AgentEvent, EventSink, Input, PermissionMode, SessionOptions, ToolStatus, tool_title};

pub(crate) async fn run(opts: SessionOptions, sink: EventSink, mut rx: mpsc::UnboundedReceiver<Input>) -> Result<()> {
    // The instructions go through a file: arguments can't carry long text through `.cmd` shims.
    let prompt_file = PromptFile::write(&opts.instructions)?;
    let mode = match opts.mode {
        PermissionMode::AutoEdit => "acceptEdits",
        PermissionMode::Ask | PermissionMode::ReadOnly => "default",
    };
    let prompt_path = prompt_file.0.to_string_lossy().into_owned();
    let mut args = vec![
        "-p",
        "--input-format", "stream-json",
        "--output-format", "stream-json",
        "--verbose",
        "--include-partial-messages",
        "--permission-prompt-tool", "stdio",
        "--permission-mode", mode,
        "--append-system-prompt-file", &prompt_path,
    ];
    if let Some(model) = &opts.model {
        args.extend(["--model", model]);
    }
    let mut proc = process::spawn("claude", &args, &opts.cwd)?;
    sink(AgentEvent::Ready);

    let mut state = State { opts: &opts, sink: &sink, pending: HashMap::new(), streamed: false };
    loop {
        tokio::select! {
            line = proc.lines.next_line() => {
                let Some(line) = line? else { break };
                let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
                if let Some(reply) = state.handle(&msg) {
                    proc.write(&reply).await?;
                }
            }
            input = rx.recv() => {
                let Some(input) = input else { break };
                state.input(&mut proc, input).await?;
            }
        }
    }
    Ok(())
}

struct State<'a> {
    opts: &'a SessionOptions,
    sink: &'a EventSink,
    /// Tool inputs of unanswered permission requests: an approval must echo them back.
    pending: HashMap<String, Value>,
    /// Whether text arrives as stream deltas; if not, full assistant messages carry it.
    streamed: bool,
}

impl State<'_> {
    /// Handles one line from Claude; returns a line to write back, if any.
    fn handle(&mut self, msg: &Value) -> Option<Value> {
        let sink = self.sink;
        match msg["type"].as_str()? {
            "stream_event" => {
                let event = &msg["event"];
                if event["type"] == "content_block_delta" && event["delta"]["type"] == "text_delta" {
                    self.streamed = true;
                    let text = event["delta"]["text"].as_str().unwrap_or_default();
                    sink(AgentEvent::Text { text: text.into() });
                }
            }
            "assistant" => {
                for block in msg["message"]["content"].as_array()? {
                    match block["type"].as_str() {
                        Some("tool_use") => sink(AgentEvent::Tool {
                            id: block["id"].as_str().unwrap_or_default().into(),
                            title: tool_title(block["name"].as_str().unwrap_or("tool"), &block["input"], &self.opts.cwd),
                            status: ToolStatus::Running,
                        }),
                        Some("text") if !self.streamed => sink(AgentEvent::Text {
                            text: block["text"].as_str().unwrap_or_default().into(),
                        }),
                        _ => {}
                    }
                }
            }
            "user" => {
                for block in msg["message"]["content"].as_array().into_iter().flatten() {
                    if block["type"] == "tool_result" {
                        let failed = block["is_error"].as_bool().unwrap_or(false);
                        sink(AgentEvent::Tool {
                            id: block["tool_use_id"].as_str().unwrap_or_default().into(),
                            title: String::new(),
                            status: if failed { ToolStatus::Failed } else { ToolStatus::Done },
                        });
                    }
                }
            }
            "control_request" => {
                let id = msg["request_id"].as_str()?.to_string();
                let req = &msg["request"];
                if req["subtype"] != "can_use_tool" {
                    return Some(control_error(&id, "unsupported request"));
                }
                let input = req["input"].clone();
                if self.opts.mode == PermissionMode::ReadOnly {
                    return Some(permission_reply(&id, false, &input));
                }
                let name = req["tool_name"].as_str().unwrap_or("tool");
                let title = tool_title(name, &input, &self.opts.cwd);
                let detail = edit_detail(&input);
                self.pending.insert(id.clone(), input);
                sink(AgentEvent::Permission { id, title, detail });
            }
            "result" => {
                let failed = msg["is_error"].as_bool().unwrap_or(false) || msg["subtype"] != "success";
                let error = failed.then(|| {
                    msg["result"].as_str().map(Into::into).unwrap_or_else(|| format!("{}", msg["subtype"]))
                });
                sink(AgentEvent::TurnEnd { error });
            }
            _ => {}
        }
        None
    }

    async fn input(&mut self, proc: &mut Proc, input: Input) -> Result<()> {
        match input {
            Input::Prompt(text) => {
                proc.write(&json!({ "type": "user", "message": { "role": "user", "content": text } })).await
            }
            Input::Permission { id, allow } => {
                let Some(tool_input) = self.pending.remove(&id) else { return Ok(()) };
                proc.write(&permission_reply(&id, allow, &tool_input)).await
            }
            Input::Cancel => {
                static NEXT: AtomicU64 = AtomicU64::new(1);
                let id = format!("dresume-{}", NEXT.fetch_add(1, Ordering::Relaxed));
                proc.write(&json!({ "type": "control_request", "request_id": id, "request": { "subtype": "interrupt" } }))
                    .await
            }
        }
    }
}

fn permission_reply(id: &str, allow: bool, input: &Value) -> Value {
    let response = if allow {
        json!({ "behavior": "allow", "updatedInput": input })
    } else {
        json!({ "behavior": "deny", "message": "The user denied this action." })
    };
    json!({ "type": "control_response", "response": { "subtype": "success", "request_id": id, "response": response } })
}

fn control_error(id: &str, error: &str) -> Value {
    json!({ "type": "control_response", "response": { "subtype": "error", "request_id": id, "error": error } })
}

/// For edits, the text being written, so the user sees what they approve.
fn edit_detail(input: &Value) -> Option<String> {
    if let (Some(old), Some(new)) = (input["old_string"].as_str(), input["new_string"].as_str()) {
        let minus = old.lines().map(|l| format!("- {l}"));
        let plus = new.lines().map(|l| format!("+ {l}"));
        return Some(minus.chain(plus).collect::<Vec<_>>().join("\n"));
    }
    input["content"].as_str().or(input["command"].as_str()).map(Into::into)
}

/// A temp file removed when dropped.
struct PromptFile(std::path::PathBuf);

impl PromptFile {
    fn write(text: &str) -> Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("dresume-agent-{}-{n}.md", std::process::id()));
        std::fs::write(&path, text)?;
        Ok(Self(path))
    }
}

impl Drop for PromptFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn state_with(mode: PermissionMode) -> (SessionOptions, Arc<Mutex<Vec<AgentEvent>>>, EventSink) {
        let opts = SessionOptions { cwd: std::env::temp_dir(), instructions: String::new(), mode, model: None };
        let events = Arc::new(Mutex::new(Vec::new()));
        let log = events.clone();
        let sink: EventSink = Arc::new(move |e| log.lock().unwrap().push(e));
        (opts, events, sink)
    }

    #[test]
    fn permission_requests_round_trip() {
        let (opts, events, sink) = state_with(PermissionMode::Ask);
        let mut state = State { opts: &opts, sink: &sink, pending: HashMap::new(), streamed: false };
        let req = json!({
            "type": "control_request", "request_id": "r1",
            "request": { "subtype": "can_use_tool", "tool_name": "Edit", "input": { "old_string": "a", "new_string": "b" } }
        });
        assert!(state.handle(&req).is_none());
        assert!(matches!(&events.lock().unwrap()[0], AgentEvent::Permission { id, .. } if id == "r1"));
        let input = state.pending.remove("r1").unwrap();
        let reply = permission_reply("r1", true, &input);
        assert_eq!(reply["response"]["response"]["updatedInput"]["new_string"], "b");
    }

    #[test]
    fn read_only_denies_without_asking() {
        let (opts, events, sink) = state_with(PermissionMode::ReadOnly);
        let mut state = State { opts: &opts, sink: &sink, pending: HashMap::new(), streamed: false };
        let req = json!({ "type": "control_request", "request_id": "r1", "request": { "subtype": "can_use_tool", "tool_name": "Write", "input": {} } });
        let reply = state.handle(&req).unwrap();
        assert_eq!(reply["response"]["response"]["behavior"], "deny");
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn deltas_replace_full_text() {
        let (opts, events, sink) = state_with(PermissionMode::AutoEdit);
        let mut state = State { opts: &opts, sink: &sink, pending: HashMap::new(), streamed: false };
        state.handle(&json!({ "type": "stream_event", "event": { "type": "content_block_delta", "delta": { "type": "text_delta", "text": "Hi" } } }));
        state.handle(&json!({ "type": "assistant", "message": { "content": [{ "type": "text", "text": "Hi" }] } }));
        state.handle(&json!({ "type": "result", "subtype": "success", "is_error": false }));
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[1], AgentEvent::TurnEnd { error: None }));
    }
}
