//! Manual check against a real agent: `cargo run -p dresume-agent --example chat -- claude <dir> "<prompt>"`.
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
    let kind: AgentKind = serde_json::from_value(serde_json::json!(kind)).expect("unknown agent");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let opts = SessionOptions {
        cwd: std::fs::canonicalize(dir).unwrap(),
        instructions: "Be brief.".into(),
        mode: PermissionMode::Ask,
        model: None,
    };
    let session = dresume_agent::start(kind, opts, Arc::new(move |e| { let _ = tx.send(e); }));
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
