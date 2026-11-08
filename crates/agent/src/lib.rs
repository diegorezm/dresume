//! Drives the AI coding agents already installed on the user's machine (Claude Code, Codex, …)
//! so the app can edit a workspace through them. No model API is called directly: every agent
//! is a CLI spawned as a child process and spoken to over its own stdio protocol.
//!
//! Each driver translates its protocol into the same [`AgentEvent`] stream, so the app never
//! special-cases an agent:
//!
//! ```text
//! claude   claude -p --input-format stream-json --output-format stream-json
//! codex    codex app-server                 (JSON-RPC)
//! ```

mod claude;
mod codex;
mod process;
mod rpc;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// The agents dresume knows how to drive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentKind {
    Claude,
    Codex,
}

impl AgentKind {
    pub const ALL: &[AgentKind] = &[AgentKind::Claude, AgentKind::Codex];

    pub fn name(self) -> &'static str {
        match self {
            AgentKind::Claude => "Claude Code",
            AgentKind::Codex => "Codex",
        }
    }

    /// Executable looked up on `PATH`.
    fn program(self) -> &'static str {
        match self {
            AgentKind::Claude => "claude",
            AgentKind::Codex => "codex",
        }
    }
}

/// An agent found (or not) on this machine.
#[derive(Clone, Debug, Serialize)]
pub struct AgentInfo {
    pub kind: AgentKind,
    pub name: &'static str,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
}

/// Looks up every known agent on `PATH` and asks each one for its version.
pub async fn detect() -> Vec<AgentInfo> {
    let mut out = Vec::new();
    for &kind in AgentKind::ALL {
        let path = which::which(kind.program()).ok();
        let version = match &path {
            Some(p) => process::version(p, Duration::from_secs(10)).await,
            None => None,
        };
        out.push(AgentInfo { kind, name: kind.name(), path, version });
    }
    out
}

/// How much the agent may do without asking.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Every write or command needs the user's approval.
    Ask,
    /// File edits inside the workspace go through; anything else asks.
    #[default]
    AutoEdit,
    /// Nothing is written: every approval request is refused.
    ReadOnly,
}

#[derive(Clone, Debug)]
pub struct SessionOptions {
    /// Workspace root; the agent runs here.
    pub cwd: PathBuf,
    /// Extra instructions appended to the agent's own system prompt.
    pub instructions: String,
    pub mode: PermissionMode,
    pub model: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolStatus {
    Running,
    Done,
    Failed,
}

/// What an agent did, normalized across agents.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// The agent process is up and accepts prompts.
    Ready,
    /// Streamed assistant text, appended to the current message.
    Text { text: String },
    /// A tool call started or changed status. `title` is a short human summary.
    Tool { id: String, title: String, status: ToolStatus },
    /// The agent wants approval; answer with [`Session::respond`].
    Permission { id: String, title: String, detail: Option<String> },
    /// The current turn finished, with an error message when it failed.
    TurnEnd { error: Option<String> },
    Error { message: String },
    /// The agent process is gone; the session is dead.
    Exited,
}

pub type EventSink = Arc<dyn Fn(AgentEvent) + Send + Sync>;

pub(crate) enum Input {
    Prompt(String),
    Permission { id: String, allow: bool },
    Cancel,
}

/// A running agent. Dropping it (or calling [`Session::close`]) stops the process.
pub struct Session {
    kind: AgentKind,
    tx: mpsc::UnboundedSender<Input>,
    task: tokio::task::JoinHandle<()>,
}

impl Session {
    pub fn kind(&self) -> AgentKind {
        self.kind
    }

    /// Sends a user message. Messages sent while a turn runs are queued by the agent.
    pub fn send(&self, text: impl Into<String>) {
        let _ = self.tx.send(Input::Prompt(text.into()));
    }

    pub fn respond(&self, id: impl Into<String>, allow: bool) {
        let _ = self.tx.send(Input::Permission { id: id.into(), allow });
    }

    /// Interrupts the current turn; the session stays usable.
    pub fn cancel(&self) {
        let _ = self.tx.send(Input::Cancel);
    }

    pub fn close(self) {}
}

impl Drop for Session {
    fn drop(&mut self) {
        // Aborting drops the driver future, and with it the child (spawned with kill_on_drop).
        self.task.abort();
    }
}

/// Starts an agent. Must be called inside a Tokio runtime. Startup failures arrive as
/// [`AgentEvent::Error`] followed by [`AgentEvent::Exited`].
pub fn start(kind: AgentKind, mut opts: SessionOptions, sink: EventSink) -> Session {
    opts.cwd = plain_path(&opts.cwd);
    let (tx, rx) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let result = match kind {
            AgentKind::Claude => claude::run(opts, sink.clone(), rx).await,
            AgentKind::Codex => codex::run(opts, sink.clone(), rx).await,
        };
        if let Err(e) = result {
            sink(AgentEvent::Error { message: format!("{e:#}") });
        }
        sink(AgentEvent::Exited);
    });
    Session { kind, tx, task }
}

/// Short label for a tool call: the tool plus its most telling argument.
pub(crate) fn tool_title(name: &str, input: &serde_json::Value, cwd: &std::path::Path) -> String {
    let arg = ["file_path", "path", "command", "pattern", "url", "query", "description"]
        .iter()
        .find_map(|k| input.get(*k).and_then(|v| v.as_str()));
    match arg {
        Some(a) => format!("{name} {}", relative(a, cwd)),
        None => name.to_string(),
    }
}

/// Drops Windows' `\\?\` verbatim prefix (from `canonicalize`), which CLIs and prefix
/// comparisons don't expect.
fn plain_path(path: &std::path::Path) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => path.to_path_buf(),
    }
}

/// Shows paths inside the workspace relative to it.
pub(crate) fn relative(text: &str, cwd: &std::path::Path) -> String {
    let path = std::path::Path::new(text);
    match path.strip_prefix(cwd) {
        Ok(rel) if path.is_absolute() => rel.to_string_lossy().replace('\\', "/"),
        _ => truncate(text, 120),
    }
}

fn truncate(text: &str, max: usize) -> String {
    let line = text.lines().next().unwrap_or("");
    if line.chars().count() > max || line.len() < text.len() {
        format!("{}…", line.chars().take(max).collect::<String>())
    } else {
        line.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tool_titles_use_relative_paths() {
        let cwd = if cfg!(windows) { PathBuf::from(r"C:\ws") } else { PathBuf::from("/ws") };
        let file = cwd.join("resumes").join("a.yaml");
        let input = json!({ "file_path": file.to_string_lossy() });
        assert_eq!(tool_title("Edit", &input, &cwd), "Edit resumes/a.yaml");
        assert_eq!(tool_title("Bash", &json!({ "command": "ls\nrm x" }), &cwd), "Bash ls…");
        assert_eq!(tool_title("Task", &json!({}), &cwd), "Task");
    }

    #[test]
    fn verbatim_prefix_is_dropped() {
        assert_eq!(plain_path(std::path::Path::new(r"\\?\C:\ws")), PathBuf::from(r"C:\ws"));
        assert_eq!(plain_path(std::path::Path::new("/ws")), PathBuf::from("/ws"));
    }
}
