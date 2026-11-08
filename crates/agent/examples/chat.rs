//! Manual check against a real agent: `cargo run -p dresume-agent --example chat -- claude <dir> "<prompt>"`.
//! `acp:<program> [args…]` (one argument, space separated) runs any ACP agent instead.
//! With `DRESUME_BIN=<path to dresume>`, the agent also gets `dresume mcp` for `<dir>`.
//! `DRESUME_MODEL` / `DRESUME_EFFORT` pick the model and reasoning effort.
//! Approves every permission request and exits when the first turn ends.

use std::sync::Arc;

use dresume_agent::{AgentEvent, AgentKind, PermissionMode, SessionOptions};
use tokio::sync::mpsc;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [kind, dir, prompt] = &args[..] else {
        eprintln!("usage: chat <agent> <dir> <prompt>");
        return;
    };
    for info in dresume_agent::detect().await {
        eprintln!("{:?}: {:?} {:?}", info.kind, info.path, info.version);
    }
    let (tx, mut rx) = mpsc::unbounded_channel();
    let cwd = std::fs::canonicalize(dir).unwrap();
    let mcp = std::env::var_os("DRESUME_BIN").map(|bin| dresume_agent::McpServer {
        name: "dresume".into(),
        command: bin.into(),
        args: vec!["-w".into(), cwd.to_string_lossy().into_owned(), "mcp".into()],
    });
    let opts = SessionOptions {
        cwd,
        instructions: "Be brief.".into(),
        mode: PermissionMode::Ask,
        model: std::env::var("DRESUME_MODEL").ok(),
        effort: std::env::var("DRESUME_EFFORT").ok(),
        mcp,
    };
    let sink: dresume_agent::EventSink = Arc::new(move |e| {
        let _ = tx.send(e);
    });
    let session = match kind.strip_prefix("acp:") {
        Some(command) => {
            let mut parts = command.split(' ').map(String::from);
            dresume_agent::start_acp(parts.next().unwrap(), parts.collect(), opts, sink)
        }
        None => {
            let kind: AgentKind = serde_json::from_value(serde_json::json!(kind)).expect("unknown agent");
            dresume_agent::start(kind, opts, sink)
        }
    };
    session.send(prompt.clone());
    while let Some(event) = rx.recv().await {
        println!("{}", serde_json::to_string(&event).unwrap());
        match event {
            AgentEvent::Permission { id, .. } => session.respond(id, true),
            AgentEvent::TurnEnd { .. } | AgentEvent::Exited => break,
            _ => {}
        }
    }
}
