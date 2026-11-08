//! Minimal JSON-RPC 2.0 over JSON lines, shared by the Codex app-server and ACP drivers.
//! Drivers own the read loop; this only builds messages and remembers what each request was for.

use std::collections::HashMap;

use serde_json::{Value, json};

pub enum Message {
    /// Reply to one of our requests.
    Response { id: u64, result: Result<Value, String> },
    /// The agent asks us something (approvals); answer with [`response`] or [`error`].
    Request { id: Value, method: String, params: Value },
    Notification { method: String, params: Value },
}

pub fn parse(line: &str) -> Option<Message> {
    let msg: Value = serde_json::from_str(line).ok()?;
    let method = msg.get("method").and_then(Value::as_str).map(String::from);
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    match (msg.get("id"), method) {
        (Some(id), Some(method)) => Some(Message::Request { id: id.clone(), method, params }),
        (None, Some(method)) => Some(Message::Notification { method, params }),
        (Some(id), None) => {
            let result = match msg.get("error") {
                Some(e) => Err(error_text(e)),
                None => Ok(msg.get("result").cloned().unwrap_or(Value::Null)),
            };
            Some(Message::Response { id: id.as_u64()?, result })
        }
        (None, None) => None,
    }
}

/// The error message plus any details the agent attached (`data.details`, `data.message`).
fn error_text(e: &Value) -> String {
    let message = e.get("message").and_then(Value::as_str).unwrap_or("request failed");
    let data = &e["data"];
    let details = data.as_str().or(data["details"].as_str()).or(data["message"].as_str());
    match details {
        Some(d) if d != message => format!("{message}: {d}"),
        _ => message.to_string(),
    }
}

/// Outgoing requests, each tagged with what the driver needs to do with the reply.
pub struct Client<P> {
    next: u64,
    pending: HashMap<u64, P>,
}

impl<P> Client<P> {
    pub fn new() -> Self {
        Self { next: 1, pending: HashMap::new() }
    }

    pub fn request(&mut self, purpose: P, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        self.pending.insert(id, purpose);
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
    }

    /// What a response was for; `None` for unknown or already handled ids.
    pub fn take(&mut self, id: u64) -> Option<P> {
        self.pending.remove(&id)
    }
}

pub fn notification(method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "method": method, "params": params })
}

pub fn response(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

pub fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Request ids are strings or numbers; permissions need a string id for the UI.
pub fn id_key(id: &Value) -> String {
    match id {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_messages() {
        assert!(matches!(parse(r#"{"id":3,"result":{"ok":true}}"#), Some(Message::Response { id: 3, result: Ok(_) })));
        assert!(matches!(parse(r#"{"id":3,"error":{"message":"no"}}"#), Some(Message::Response { result: Err(e), .. }) if e == "no"));
        let detailed = r#"{"id":3,"error":{"message":"Internal error","data":{"details":"boom"}}}"#;
        assert!(matches!(parse(detailed), Some(Message::Response { result: Err(e), .. }) if e == "Internal error: boom"));
        assert!(matches!(parse(r#"{"id":0,"method":"x","params":{}}"#), Some(Message::Request { .. })));
        assert!(matches!(parse(r#"{"method":"x"}"#), Some(Message::Notification { .. })));
        assert!(parse("not json").is_none());
    }

    #[test]
    fn remembers_purposes() {
        let mut client = Client::new();
        let req = client.request("init", "initialize", json!({}));
        assert_eq!(req["id"], 1);
        assert_eq!(client.take(1), Some("init"));
        assert_eq!(client.take(1), None);
    }
}
