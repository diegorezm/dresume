//! Agent Client Protocol (<https://agentclientprotocol.com>): JSON-RPC over stdio, spoken natively
//! by Gemini CLI (`gemini --acp`), OpenCode (`opencode acp`) and others.
//!
//! `initialize` → `session/new { cwd }` → one `session/prompt` per message, whose response marks
//! the end of the turn. Progress arrives as `session/update` notifications; approvals as
//! `session/request_permission` requests. ACP has no system prompt, so the instructions are
//! prepended to the first message.

use std::collections::{HashMap, VecDeque};

use anyhow::{Result, bail};
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::process;
use crate::rpc::{self, Message};
use crate::{AgentEvent, EventSink, Input, PermissionMode, SessionOptions, ToolStatus, relative};

enum Purpose {
    Initialize,
    NewSession,
    Prompt,
}

pub(crate) async fn run(
    program: &str,
    args: &[&str],
    opts: SessionOptions,
    sink: EventSink,
    mut rx: mpsc::UnboundedReceiver<Input>,
) -> Result<()> {
    let mut proc = process::spawn(program, args, &opts.cwd)?;
    let mut state = State::new(program, &opts, &sink);
    let init = state.rpc.request(
        Purpose::Initialize,
        "initialize",
        json!({
            "protocolVersion": 1,
            // The agent reads and writes files itself; we don't proxy fs or terminals.
            "clientCapabilities": { "fs": { "readTextFile": false, "writeTextFile": false }, "terminal": false },
            "clientInfo": { "name": "dresume", "version": env!("CARGO_PKG_VERSION") },
        }),
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

struct Permission {
    rpc_id: Value,
    /// Option ids offered by the agent for "allow once" and "reject once".
    allow: Option<String>,
    reject: Option<String>,
}

struct State<'a> {
    program: &'a str,
    opts: &'a SessionOptions,
    sink: &'a EventSink,
    rpc: rpc::Client<Purpose>,
    session: Option<String>,
    busy: bool,
    /// The instructions still have to be sent (with the first prompt).
    fresh: bool,
    queue: VecDeque<String>,
    pending: HashMap<String, Permission>,
    /// Tool kinds by call id (`edit`, `read`, `execute`, …), to auto-approve edits.
    kinds: HashMap<String, String>,
}

impl<'a> State<'a> {
    fn new(program: &'a str, opts: &'a SessionOptions, sink: &'a EventSink) -> Self {
        Self {
            program,
            opts,
            sink,
            rpc: rpc::Client::new(),
            session: None,
            busy: false,
            fresh: true,
            queue: VecDeque::new(),
            pending: HashMap::new(),
            kinds: HashMap::new(),
        }
    }

    fn handle(&mut self, msg: Message) -> Result<Vec<Value>> {
        let sink = self.sink;
        let mut out = Vec::new();
        match msg {
            Message::Response { id, result } => match (self.rpc.take(id), result) {
                (Some(Purpose::Initialize), Ok(_)) => {
                    let params = json!({ "cwd": self.opts.cwd, "mcpServers": [] });
                    out.push(self.rpc.request(Purpose::NewSession, "session/new", params));
                }
                (Some(Purpose::NewSession), Ok(result)) => {
                    let Some(id) = result["sessionId"].as_str() else { bail!("{}: no session id", self.program) };
                    self.session = Some(id.into());
                    sink(AgentEvent::Ready);
                    out.extend(self.next_prompt());
                }
                (Some(Purpose::Initialize), Err(e)) => bail!("{}: {e}", self.program),
                (Some(Purpose::NewSession), Err(e)) => {
                    bail!("{}: {e}. If it needs a login, run `{}` once in a terminal.", self.program, self.program)
                }
                (Some(Purpose::Prompt), result) => {
                    let error = match result {
                        Ok(r) if r["stopReason"] == "refusal" => Some("The agent refused this request.".into()),
                        Ok(_) => None,
                        Err(e) => Some(e),
                    };
                    self.busy = false;
                    sink(AgentEvent::TurnEnd { error });
                    out.extend(self.next_prompt());
                }
                (None, _) => {}
            },
            Message::Notification { method, params } if method == "session/update" => self.update(&params["update"]),
            Message::Notification { .. } => {}
            Message::Request { id, method, params } => out.extend(self.request(id, &method, &params)),
        }
        Ok(out)
    }

    fn update(&mut self, update: &Value) {
        let sink = self.sink;
        match update["sessionUpdate"].as_str() {
            Some("agent_message_chunk") => {
                if let Some(text) = update["content"]["text"].as_str() {
                    sink(AgentEvent::Text { text: text.into() });
                }
            }
            Some(kind @ ("tool_call" | "tool_call_update")) => {
                let id = update["toolCallId"].as_str().unwrap_or_default().to_string();
                if let Some(k) = update["kind"].as_str() {
                    self.kinds.insert(id.clone(), k.into());
                }
                let status = match update["status"].as_str() {
                    Some("completed") => ToolStatus::Done,
                    Some("failed") => ToolStatus::Failed,
                    Some(_) => ToolStatus::Running,
                    // An update without status only changes details.
                    None if kind == "tool_call_update" => return,
                    None => ToolStatus::Running,
                };
                let title = update["title"].as_str().map(|t| relative(t, &self.opts.cwd)).unwrap_or_default();
                sink(AgentEvent::Tool { id, title, status });
            }
            _ => {}
        }
    }

    fn request(&mut self, id: Value, method: &str, params: &Value) -> Option<Value> {
        if method != "session/request_permission" {
            // We declared no fs/terminal capabilities, so nothing else should arrive.
            return Some(rpc::error(&id, -32601, "not supported by dresume"));
        }
        let options = params["options"].as_array().cloned().unwrap_or_default();
        let pick = |kinds: &[&str]| {
            kinds.iter().find_map(|k| {
                options.iter().find(|o| o["kind"] == *k).and_then(|o| o["optionId"].as_str()).map(String::from)
            })
        };
        let permission = Permission {
            rpc_id: id,
            allow: pick(&["allow_once", "allow_always"]),
            reject: pick(&["reject_once", "reject_always"]),
        };
        let call = &params["toolCall"];
        let call_id = call["toolCallId"].as_str().unwrap_or_default();
        let kind = call["kind"].as_str().or(self.kinds.get(call_id).map(String::as_str));
        let auto = match self.opts.mode {
            PermissionMode::ReadOnly => Some(false),
            PermissionMode::AutoEdit if kind == Some("edit") => Some(true),
            _ => None,
        };
        if let Some(allow) = auto {
            return Some(permission_reply(&permission, allow));
        }
        let key = rpc::id_key(&permission.rpc_id);
        let title = call["title"].as_str().map(|t| relative(t, &self.opts.cwd)).unwrap_or_else(|| "Use a tool".into());
        let detail = diff_detail(call, &self.opts.cwd);
        self.pending.insert(key.clone(), permission);
        (self.sink)(AgentEvent::Permission { id: key, title, detail });
        None
    }

    fn input(&mut self, input: Input) -> Vec<Value> {
        match input {
            Input::Prompt(text) => {
                self.queue.push_back(text);
                self.next_prompt().into_iter().collect()
            }
            Input::Permission { id, allow } => match self.pending.remove(&id) {
                Some(p) => vec![permission_reply(&p, allow)],
                None => vec![],
            },
            Input::Cancel => {
                self.queue.clear();
                let Some(session) = &self.session else { return vec![] };
                // Open permission requests must be answered `cancelled` when a turn is cancelled.
                let mut out: Vec<Value> = self
                    .pending
                    .drain()
                    .map(|(_, p)| rpc::response(&p.rpc_id, json!({ "outcome": { "outcome": "cancelled" } })))
                    .collect();
                out.push(rpc::notification("session/cancel", json!({ "sessionId": session })));
                out
            }
        }
    }

    fn next_prompt(&mut self) -> Option<Value> {
        let session = self.session.clone()?;
        if self.busy {
            return None;
        }
        let mut text = self.queue.pop_front()?;
        if std::mem::take(&mut self.fresh) {
            text = format!("<dresume-context>\n{}</dresume-context>\n\n{text}", self.opts.instructions);
        }
        self.busy = true;
        let params = json!({ "sessionId": session, "prompt": [{ "type": "text", "text": text }] });
        Some(self.rpc.request(Purpose::Prompt, "session/prompt", params))
    }
}

fn permission_reply(p: &Permission, allow: bool) -> Value {
    let option = if allow { &p.allow } else { &p.reject };
    let outcome = match option {
        Some(id) => json!({ "outcome": "selected", "optionId": id }),
        // Nothing to pick (an agent offering no reject option): cancelling is the safe refusal.
        None => json!({ "outcome": "cancelled" }),
    };
    rpc::response(&p.rpc_id, json!({ "outcome": outcome }))
}

/// Diff contents of a tool call, as `- old` / `+ new` lines per file.
fn diff_detail(call: &Value, cwd: &std::path::Path) -> Option<String> {
    let parts: Vec<String> = call["content"]
        .as_array()?
        .iter()
        .filter(|c| c["type"] == "diff")
        .map(|c| {
            let path = relative(c["path"].as_str().unwrap_or_default(), cwd);
            let old = c["oldText"].as_str().unwrap_or_default().lines().map(|l| format!("- {l}"));
            let new = c["newText"].as_str().unwrap_or_default().lines().map(|l| format!("+ {l}"));
            std::iter::once(path).chain(old).chain(new).collect::<Vec<_>>().join("\n")
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn harness(mode: PermissionMode) -> (SessionOptions, Arc<Mutex<Vec<AgentEvent>>>, EventSink) {
        let opts = SessionOptions { cwd: std::env::temp_dir(), instructions: "rules\n".into(), mode, model: None };
        let events = Arc::new(Mutex::new(Vec::new()));
        let log = events.clone();
        (opts, events, Arc::new(move |e| log.lock().unwrap().push(e)))
    }

    fn line(v: Value) -> Message {
        rpc::parse(&v.to_string()).unwrap()
    }

    fn permission_request(kind: &str) -> Message {
        line(json!({ "id": 9, "method": "session/request_permission", "params": {
            "sessionId": "s",
            "toolCall": { "toolCallId": "c1", "title": "Write a.yaml", "kind": kind,
                "content": [{ "type": "diff", "path": "a.yaml", "oldText": "x", "newText": "y" }] },
            "options": [
                { "optionId": "yes", "name": "Allow", "kind": "allow_once" },
                { "optionId": "no", "name": "Reject", "kind": "reject_once" }
            ] } }))
    }

    #[test]
    fn first_prompt_carries_instructions() {
        let (opts, events, sink) = harness(PermissionMode::Ask);
        let mut s = State::new("agent", &opts, &sink);
        s.rpc.request(Purpose::Initialize, "initialize", json!({}));
        s.input(Input::Prompt("hello".into()));
        let out = s.handle(line(json!({ "id": 1, "result": { "protocolVersion": 1 } }))).unwrap();
        assert_eq!(out[0]["method"], "session/new");
        let out = s.handle(line(json!({ "id": 2, "result": { "sessionId": "s" } }))).unwrap();
        let text = out[0]["params"]["prompt"][0]["text"].as_str().unwrap();
        assert!(text.starts_with("<dresume-context>\nrules\n") && text.ends_with("hello"));
        s.handle(line(json!({ "id": 3, "result": { "stopReason": "end_turn" } }))).unwrap();
        let out = s.input(Input::Prompt("again".into()));
        assert_eq!(out[0]["params"]["prompt"][0]["text"], "again");
        assert!(matches!(events.lock().unwrap()[1], AgentEvent::TurnEnd { error: None }));
    }

    #[test]
    fn asks_then_selects_the_offered_option() {
        let (opts, events, sink) = harness(PermissionMode::Ask);
        let mut s = State::new("agent", &opts, &sink);
        assert!(s.handle(permission_request("edit")).unwrap().is_empty());
        let AgentEvent::Permission { id, detail, .. } = &events.lock().unwrap()[0] else { panic!() };
        assert_eq!(detail.as_deref(), Some("a.yaml\n- x\n+ y"));
        let out = s.input(Input::Permission { id: id.clone(), allow: false });
        assert_eq!(out[0]["result"]["outcome"]["optionId"], "no");
    }

    #[test]
    fn auto_edit_allows_edits_only() {
        let (opts, events, sink) = harness(PermissionMode::AutoEdit);
        let mut s = State::new("agent", &opts, &sink);
        let out = s.handle(permission_request("edit")).unwrap();
        assert_eq!(out[0]["result"]["outcome"]["optionId"], "yes");
        assert!(s.handle(permission_request("execute")).unwrap().is_empty());
        assert_eq!(events.lock().unwrap().len(), 1);
    }

    #[test]
    fn cancel_answers_open_requests() {
        let (opts, _, sink) = harness(PermissionMode::Ask);
        let mut s = State::new("agent", &opts, &sink);
        s.session = Some("s".into());
        s.handle(permission_request("execute")).unwrap();
        let out = s.input(Input::Cancel);
        assert_eq!(out[0]["result"]["outcome"]["outcome"], "cancelled");
        assert_eq!(out[1]["method"], "session/cancel");
    }
}
