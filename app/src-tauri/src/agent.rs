//! Tauri commands for the agent panel: thin wrappers over `dresume-agent`, plus the chat history
//! kept in `<config>/chats/` (per user, never in the workspace).

use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::{Arc, Mutex};

use dresume_agent::{AgentEvent, AgentInfo, AgentKind, PermissionMode, Session, SessionOptions};
use serde_json::Value;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

use crate::{AppState, CmdResult, config_dir, err, with_ws};

#[derive(Default)]
pub struct AgentState {
    sessions: Mutex<HashMap<u32, Session>>,
    next: Mutex<u32>,
}

impl AgentState {
    fn with<T>(&self, id: u32, f: impl FnOnce(&Session) -> T) -> CmdResult<T> {
        let sessions = self.sessions.lock().unwrap();
        sessions.get(&id).map(f).ok_or_else(|| "agent session not found".into())
    }
}

#[tauri::command]
pub async fn list_agents() -> Vec<AgentInfo> {
    dresume_agent::detect().await
}

/// Starts an agent in the workspace root. Events stream through `on_event` until `Exited`.
#[tauri::command]
pub async fn agent_start(
    state: State<'_, AppState>,
    agents: State<'_, AgentState>,
    kind: AgentKind,
    mode: PermissionMode,
    resume: Option<String>,
    on_event: Channel<AgentEvent>,
) -> CmdResult<u32> {
    let (cwd, instructions) = with_ws(&state, |ws| {
        let template = match &resume {
            Some(id) => Some(ws.resume_info(id)?.template),
            None => None,
        };
        Ok((ws.root().to_path_buf(), instructions(ws.root(), resume.as_deref(), template.as_deref())))
    })?;
    let opts = SessionOptions { cwd, instructions, mode, model: None };
    let session = dresume_agent::start(kind, opts, Arc::new(move |e| {
        let _ = on_event.send(e);
    }));
    let id = {
        let mut next = agents.next.lock().unwrap();
        *next += 1;
        *next
    };
    agents.sessions.lock().unwrap().insert(id, session);
    Ok(id)
}

#[tauri::command]
pub fn agent_send(agents: State<AgentState>, id: u32, text: String) -> CmdResult<()> {
    agents.with(id, |s| s.send(text))
}

#[tauri::command]
pub fn agent_respond(agents: State<AgentState>, id: u32, request: String, allow: bool) -> CmdResult<()> {
    agents.with(id, |s| s.respond(request, allow))
}

#[tauri::command]
pub fn agent_cancel(agents: State<AgentState>, id: u32) -> CmdResult<()> {
    agents.with(id, |s| s.cancel())
}

#[tauri::command]
pub fn agent_stop(agents: State<AgentState>, id: u32) {
    agents.sessions.lock().unwrap().remove(&id);
}

/// What the agent is told about dresume and about what the user has open.
fn instructions(root: &Path, resume: Option<&str>, template: Option<&str>) -> String {
    let mut text = format!(
        "You are running inside dresume, a resume manager built on Typst. The current directory \
         ({}) is a dresume workspace: read AGENTS.md first, it explains the file layout and rules.\n\
         - Resume content lives in resumes/<id>.yaml and must match templates/<template>/schema.json.\n\
         - Edit the files directly; the app's live preview reloads when they change.\n\
         - If the `dresume` CLI is installed, check your work with `dresume validate <id>` and `dresume build <id>`.\n\
         - Keep the user's facts: never invent experience, dates, employers or numbers.\n",
        root.display()
    );
    if let (Some(id), Some(template)) = (resume, template) {
        text.push_str(&format!(
            "\nThe user has resumes/{id}.yaml open (template `{template}`). Unless they say otherwise, \
             their requests are about that resume.\n"
        ));
    }
    text
}

/// Chat transcripts are keyed by workspace root and resume.
fn chat_path(app: &AppHandle, root: &Path, key: &str) -> CmdResult<std::path::PathBuf> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    root.hash(&mut hasher);
    key.hash(&mut hasher);
    let dir = config_dir(app)?.join("chats");
    fs::create_dir_all(&dir).map_err(err)?;
    Ok(dir.join(format!("{:016x}.json", hasher.finish())))
}

#[tauri::command]
pub fn load_chat(app: AppHandle, state: State<AppState>, key: String) -> CmdResult<Value> {
    let root = with_ws(&state, |ws| Ok(ws.root().to_path_buf()))?;
    let path = chat_path(&app, &root, &key)?;
    Ok(fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Array(vec![])))
}

#[tauri::command]
pub fn save_chat(app: AppHandle, state: State<AppState>, key: String, messages: Value) -> CmdResult<()> {
    let root = with_ws(&state, |ws| Ok(ws.root().to_path_buf()))?;
    let path = chat_path(&app, &root, &key)?;
    fs::write(path, serde_json::to_string(&messages).map_err(err)?).map_err(err)
}
