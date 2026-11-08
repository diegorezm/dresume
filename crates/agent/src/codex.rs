//! Codex through `codex app-server`: JSON-RPC over stdio.
//!
//! Handshake: `initialize` → `initialized` → `thread/start` (cwd, approval policy, sandbox,
//! developer instructions). Each prompt is a `turn/start`; progress arrives as `item/*`
//! notifications and the turn ends with `turn/completed`. Approvals are server requests
//! (`item/fileChange/requestApproval`, `item/commandExecution/requestApproval`).

use std::collections::{HashMap, VecDeque};

use anyhow::{Result, bail};
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::process;
use crate::rpc::{self, Message};
use crate::{AgentEvent, EventSink, Input, PermissionMode, SessionOptions, ToolStatus, relative};

enum Purpose {
    Initialize,
    StartThread,
    StartTurn,
    Interrupt,
}

pub(crate) async fn run(opts: SessionOptions, sink: EventSink, mut rx: mpsc::UnboundedReceiver<Input>) -> Result<()> {
    let mut proc = process::spawn("codex", &["app-server"], &opts.cwd)?;
    let mut state = State::new(&opts, &sink);
    let init = state.rpc.request(
        Purpose::Initialize,
        "initialize",
        json!({ "clientInfo": { "name": "dresume", "title": "dresume", "version": env!("CARGO_PKG_VERSION") } }),
    );
    proc.write(&init).await?;
    loop {
        tokio::select! {
            line = proc.lines.next_line() => {
                let Some(line) = line? else { break };
                let Some(msg) = rpc::parse(&line) else { continue };
                for out in state.handle(msg)? {
                    proc.write(&out).await?;
                }
            }
            input = rx.recv() => {
                let Some(input) = input else { break };
                for out in state.input(input) {
                    proc.write(&out).await?;
                }
            }
        }
    }
    Ok(())
}

struct State<'a> {
    opts: &'a SessionOptions,
    sink: &'a EventSink,
    rpc: rpc::Client<Purpose>,
    thread: Option<String>,
    turn: Option<String>,
    /// A turn was requested and hasn't completed yet.
    busy: bool,
    /// Prompts sent before the thread exists or while a turn runs.
    queue: VecDeque<String>,
    /// Unanswered approvals: UI id → (JSON-RPC id, how to answer).
    pending: HashMap<String, (Value, Reply)>,
    /// Diffs of file-change items, shown when Codex asks to apply them.
    diffs: HashMap<String, String>,
    /// Last agent message that produced text, to separate consecutive messages.
    text_item: Option<String>,
}

impl<'a> State<'a> {
    fn new(opts: &'a SessionOptions, sink: &'a EventSink) -> Self {
        Self {
            opts,
            sink,
            rpc: rpc::Client::new(),
            thread: None,
            turn: None,
            busy: false,
            queue: VecDeque::new(),
            pending: HashMap::new(),
            diffs: HashMap::new(),
            text_item: None,
        }
    }

    fn handle(&mut self, msg: Message) -> Result<Vec<Value>> {
        let sink = self.sink;
        let mut out = Vec::new();
        match msg {
            Message::Response { id, result } => match (self.rpc.take(id), result) {
                (Some(Purpose::Initialize), Ok(_)) => {
                    out.push(rpc::notification("initialized", json!({})));
                    let (approval, sandbox) = match self.opts.mode {
                        PermissionMode::Ask => ("untrusted", "workspace-write"),
                        PermissionMode::AutoEdit => ("on-request", "workspace-write"),
                        PermissionMode::ReadOnly => ("never", "read-only"),
                    };
                    let mut params = json!({
                        "cwd": self.opts.cwd,
                        "approvalPolicy": approval,
                        "sandbox": sandbox,
                        "developerInstructions": self.opts.instructions,
                    });
                    if let Some(model) = &self.opts.model {
                        params["model"] = json!(model);
                    }
                    if let Some(server) = &self.opts.mcp {
                        let entry = json!({ "command": server.command, "args": server.args });
                        params["config"] = json!({ "mcp_servers": { &server.name: entry } });
                    }
                    out.push(self.rpc.request(Purpose::StartThread, "thread/start", params));
                }
                (Some(Purpose::StartThread), Ok(result)) => {
                    let Some(id) = result["thread"]["id"].as_str() else { bail!("codex returned no thread id") };
                    self.thread = Some(id.into());
                    sink(AgentEvent::Ready);
                    out.extend(self.next_turn());
                }
                (Some(Purpose::StartTurn), Ok(result)) => {
                    self.turn = result["turn"]["id"].as_str().map(Into::into);
                }
                (Some(Purpose::Initialize | Purpose::StartThread), Err(e)) => bail!("codex: {e}"),
                (Some(Purpose::StartTurn), Err(e)) => {
                    self.busy = false;
                    sink(AgentEvent::TurnEnd { error: Some(e) });
                    out.extend(self.next_turn());
                }
                (Some(Purpose::Interrupt), Err(e)) => sink(AgentEvent::Error { message: e }),
                _ => {}
            },
            Message::Notification { method, params } => self.notification(&method, &params, &mut out),
            Message::Request { id, method, params } => out.extend(self.request(id, &method, &params)),
        }
        Ok(out)
    }

    fn notification(&mut self, method: &str, params: &Value, out: &mut Vec<Value>) {
        let sink = self.sink;
        match method {
            "item/agentMessage/delta" => {
                let item = params["itemId"].as_str().map(String::from);
                if self.text_item.is_some() && self.text_item != item {
                    sink(AgentEvent::Text { text: "\n\n".into() });
                }
                self.text_item = item;
                sink(AgentEvent::Text { text: params["delta"].as_str().unwrap_or_default().into() });
            }
            "item/started" | "item/completed" => {
                let item = &params["item"];
                let Some(title) = self.item_title(item) else { return };
                let status = match item["status"].as_str() {
                    Some("inProgress") | None if method == "item/started" => ToolStatus::Running,
                    Some("failed" | "declined") => ToolStatus::Failed,
                    _ => ToolStatus::Done,
                };
                // Any tool call ends the current text block.
                self.text_item = None;
                sink(AgentEvent::Tool { id: item["id"].as_str().unwrap_or_default().into(), title, status });
            }
            "turn/completed" => {
                let turn = &params["turn"];
                let error = match turn["status"].as_str() {
                    Some("failed") => Some(turn["error"]["message"].as_str().unwrap_or("turn failed").to_string()),
                    _ => None,
                };
                self.busy = false;
                self.turn = None;
                self.text_item = None;
                sink(AgentEvent::TurnEnd { error });
                out.extend(self.next_turn());
            }
            "error" if !params["willRetry"].as_bool().unwrap_or(false) => {
                let message = params["error"]["message"].as_str().unwrap_or("codex error");
                sink(AgentEvent::Error { message: message.into() });
            }
            _ => {}
        }
    }

    /// A short title for tool-like items; `None` for messages and reasoning.
    fn item_title(&mut self, item: &Value) -> Option<String> {
        let cwd = &self.opts.cwd;
        Some(match item["type"].as_str()? {
            "commandExecution" => format!("Run {}", relative(&unwrap_shell(item["command"].as_str().unwrap_or_default()), cwd)),
            "fileChange" => {
                let changes = item["changes"].as_array().cloned().unwrap_or_default();
                let diff: Vec<String> = changes
                    .iter()
                    .map(|c| format!("{}\n{}", relative(c["path"].as_str().unwrap_or_default(), cwd), c["diff"].as_str().unwrap_or_default()))
                    .collect();
                if let Some(id) = item["id"].as_str() {
                    self.diffs.insert(id.into(), diff.join("\n"));
                }
                let paths: Vec<String> =
                    changes.iter().map(|c| relative(c["path"].as_str().unwrap_or_default(), cwd)).collect();
                format!("Edit {}", paths.join(", "))
            }
            "mcpToolCall" => format!("{}.{}", item["server"].as_str().unwrap_or("mcp"), item["tool"].as_str().unwrap_or("tool")),
            "dynamicToolCall" => item["tool"].as_str().unwrap_or("tool").to_string(),
            "webSearch" => format!("Search {}", item["query"].as_str().unwrap_or_default()),
            _ => return None,
        })
    }

    fn request(&mut self, id: Value, method: &str, params: &Value) -> Option<Value> {
        let cwd = &self.opts.cwd;
        let (title, detail, kind) = match method {
            "item/fileChange/requestApproval" => {
                let item = params["itemId"].as_str().unwrap_or_default();
                ("Edit files".to_string(), self.diffs.get(item).cloned(), Reply::Decision)
            }
            "item/commandExecution/requestApproval" => {
                let command = unwrap_shell(params["command"].as_str().unwrap_or("a command"));
                (format!("Run {}", relative(&command, cwd)), Some(command), Reply::Decision)
            }
            "applyPatchApproval" => ("Edit files".to_string(), None, Reply::Legacy),
            "execCommandApproval" => {
                let command = params["command"].as_array().map(|parts| {
                    parts.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" ")
                });
                let command = command.unwrap_or_else(|| "a command".into());
                (format!("Run {}", relative(&command, cwd)), Some(command), Reply::Legacy)
            }
            // Codex asks before MCP tool calls through a form elicitation. dresume's own server
            // (validate, build, …) only reads the workspace or writes output/, so it's approved.
            "mcpServer/elicitation/request" if params["mode"] == "form" => {
                let message = params["message"].as_str().unwrap_or("Use an MCP tool").to_string();
                let ours = self.opts.mcp.as_ref().is_some_and(|s| params["serverName"] == s.name.as_str() || message.contains(&s.name));
                if ours {
                    return Some(reply(&id, true, Reply::Elicitation));
                }
                (message, None, Reply::Elicitation)
            }
            // Questions to the user, URL elicitations, auth refreshes: not supported here.
            _ => return Some(rpc::error(&id, -32601, "not supported by dresume")),
        };
        if self.opts.mode == PermissionMode::ReadOnly {
            return Some(reply(&id, false, kind));
        }
        let key = rpc::id_key(&id);
        self.pending.insert(key.clone(), (id, kind));
        let detail = detail.or_else(|| params["reason"].as_str().map(Into::into));
        (self.sink)(AgentEvent::Permission { id: key, title, detail });
        None
    }

    fn input(&mut self, input: Input) -> Vec<Value> {
        match input {
            Input::Prompt(text) => {
                self.queue.push_back(text);
                self.next_turn().into_iter().collect()
            }
            Input::Permission { id, allow } => match self.pending.remove(&id) {
                Some((rpc_id, kind)) => vec![reply(&rpc_id, allow, kind)],
                None => vec![],
            },
            Input::Cancel => {
                self.queue.clear();
                match (&self.thread, &self.turn) {
                    (Some(thread), Some(turn)) => {
                        let params = json!({ "threadId": thread, "turnId": turn });
                        vec![self.rpc.request(Purpose::Interrupt, "turn/interrupt", params)]
                    }
                    _ => vec![],
                }
            }
        }
    }

    /// Starts the next queued prompt when the thread is ready and idle.
    fn next_turn(&mut self) -> Option<Value> {
        let thread = self.thread.clone()?;
        if self.busy {
            return None;
        }
        let text = self.queue.pop_front()?;
        self.busy = true;
        let params = json!({ "threadId": thread, "input": [{ "type": "text", "text": text, "text_elements": [] }] });
        Some(self.rpc.request(Purpose::StartTurn, "turn/start", params))
    }
}

/// Codex wraps commands in a shell (`"…pwsh.exe" -Command '…'`, `bash -lc '…'`); show the inner one.
fn unwrap_shell(command: &str) -> String {
    for flag in [" -Command ", " -lc ", " -c "] {
        if let Some((_, inner)) = command.split_once(flag) {
            let inner = inner.trim();
            let quoted = inner.len() >= 2
                && ((inner.starts_with('\'') && inner.ends_with('\'')) || (inner.starts_with('"') && inner.ends_with('"')));
            return if quoted { inner[1..inner.len() - 1].to_string() } else { inner.to_string() };
        }
    }
    command.to_string()
}

/// How an approval request is answered.
#[derive(Clone, Copy)]
enum Reply {
    /// `{ decision: accept | decline }`
    Decision,
    /// `applyPatchApproval` / `execCommandApproval`: `{ decision: approved | denied }`
    Legacy,
    /// MCP form elicitation: `{ action: accept | decline, content }`
    Elicitation,
}

fn reply(id: &Value, allow: bool, kind: Reply) -> Value {
    let result = match (kind, allow) {
        (Reply::Decision, true) => json!({ "decision": "accept" }),
        (Reply::Decision, false) => json!({ "decision": "decline" }),
        (Reply::Legacy, true) => json!({ "decision": "approved" }),
        (Reply::Legacy, false) => json!({ "decision": "denied" }),
        (Reply::Elicitation, true) => json!({ "action": "accept", "content": {} }),
        (Reply::Elicitation, false) => json!({ "action": "decline", "content": null }),
    };
    rpc::response(id, result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn harness(mode: PermissionMode) -> (SessionOptions, Arc<Mutex<Vec<AgentEvent>>>, EventSink) {
        let opts = SessionOptions { cwd: std::env::temp_dir(), instructions: "x".into(), mode, model: None, mcp: None };
        let events = Arc::new(Mutex::new(Vec::new()));
        let log = events.clone();
        (opts, events, Arc::new(move |e| log.lock().unwrap().push(e)))
    }

    fn line(v: Value) -> Message {
        rpc::parse(&v.to_string()).unwrap()
    }

    #[test]
    fn handshake_then_queued_prompt() {
        let (opts, events, sink) = harness(PermissionMode::AutoEdit);
        let mut s = State::new(&opts, &sink);
        s.rpc.request(Purpose::Initialize, "initialize", json!({}));
        assert!(s.input(Input::Prompt("hi".into())).is_empty(), "no thread yet");
        let out = s.handle(line(json!({ "id": 1, "result": {} }))).unwrap();
        assert_eq!(out[0]["method"], "initialized");
        assert_eq!(out[1]["method"], "thread/start");
        assert_eq!(out[1]["params"]["sandbox"], "workspace-write");
        let out = s.handle(line(json!({ "id": 2, "result": { "thread": { "id": "t1" } } }))).unwrap();
        assert_eq!(out[0]["method"], "turn/start");
        assert_eq!(out[0]["params"]["input"][0]["text"], "hi");
        assert!(matches!(events.lock().unwrap()[0], AgentEvent::Ready));
        // A second prompt waits for the turn to complete.
        assert!(s.input(Input::Prompt("again".into())).is_empty());
        let out = s.handle(line(json!({ "method": "turn/completed", "params": { "turn": { "status": "completed" } } }))).unwrap();
        assert_eq!(out[0]["params"]["input"][0]["text"], "again");
    }

    #[test]
    fn approvals_round_trip() {
        let (opts, events, sink) = harness(PermissionMode::Ask);
        let mut s = State::new(&opts, &sink);
        s.handle(line(json!({ "method": "item/started", "params": { "item": {
            "type": "fileChange", "id": "f1", "status": "inProgress",
            "changes": [{ "path": "a.yaml", "kind": { "type": "update" }, "diff": "-x\n+y" }] } } }))).unwrap();
        let out = s.handle(line(json!({ "id": 0, "method": "item/fileChange/requestApproval", "params": { "itemId": "f1" } }))).unwrap();
        assert!(out.is_empty());
        let events = events.lock().unwrap();
        let AgentEvent::Permission { id, detail, .. } = &events[1] else { panic!("expected permission") };
        assert!(detail.as_deref().unwrap().contains("+y"));
        let reply = s.input(Input::Permission { id: id.clone(), allow: true });
        assert_eq!(reply[0]["result"]["decision"], "accept");
        assert_eq!(reply[0]["id"], 0);
    }

    #[test]
    fn shell_wrappers_are_unwrapped() {
        assert_eq!(unwrap_shell(r#""C:\pwsh.exe" -Command 'Get-Content a.yaml'"#), "Get-Content a.yaml");
        assert_eq!(unwrap_shell("/bin/bash -lc 'ls -la'"), "ls -la");
        assert_eq!(unwrap_shell("ls"), "ls");
    }

    #[test]
    fn read_only_declines() {
        let (opts, _, sink) = harness(PermissionMode::ReadOnly);
        let mut s = State::new(&opts, &sink);
        let out = s.handle(line(json!({ "id": 5, "method": "execCommandApproval", "params": { "command": ["rm", "x"] } }))).unwrap();
        assert_eq!(out[0]["result"]["decision"], "denied");
    }
}
