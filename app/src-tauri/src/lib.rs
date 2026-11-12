mod agent;
mod watch;

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use dresume_core::{
    Diagnostic, Formats, Overlay, ProfileVisibility, ResumeInfo, SchemaError, TemplateInfo,
    Workspace, compile, compile_template_preview,
    memory::{self, MemoryStatus, Remembered},
};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Manager, State};

#[derive(Default)]
struct AppState(Mutex<Option<Workspace>>);

/// Watcher of the open workspace; replaced when another one is opened.
#[derive(Default)]
struct WatchState(Mutex<Option<notify::RecommendedWatcher>>);

/// Errors cross the IPC boundary as plain strings.
type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    format!("{e:#}")
}

fn with_ws<T>(state: &State<AppState>, f: impl FnOnce(&Workspace) -> anyhow::Result<T>) -> CmdResult<T> {
    let guard = state.0.lock().unwrap();
    let ws = guard.as_ref().ok_or("no workspace open")?;
    f(ws).map_err(err)
}

#[derive(Serialize)]
struct Summary {
    name: String,
    root: PathBuf,
    templates: Vec<TemplateInfo>,
    resumes: Vec<ResumeInfo>,
}

fn summary(ws: &Workspace) -> anyhow::Result<Summary> {
    Ok(Summary {
        name: ws.manifest.name.clone(),
        root: ws.root().to_path_buf(),
        templates: ws.templates()?,
        resumes: ws.resumes()?,
    })
}

#[derive(Serialize)]
struct Preview {
    svg: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

#[tauri::command]
fn open_workspace(
    app: AppHandle,
    state: State<AppState>,
    watcher: State<WatchState>,
    path: PathBuf,
    create: bool,
) -> CmdResult<Summary> {
    let ws = if create {
        let name = path.file_name().map_or("My resumes".into(), |n| n.to_string_lossy());
        Workspace::init(&path, &name)
    } else {
        Workspace::discover(&path)
    }
    .map_err(err)?;
    let out = summary(&ws).map_err(err)?;
    // Live reload is a convenience: the app works without it.
    *watcher.0.lock().unwrap() = watch::watch(app, ws.root().to_path_buf()).ok();
    *state.0.lock().unwrap() = Some(ws);
    Ok(out)
}

#[tauri::command]
fn workspace_summary(state: State<AppState>) -> CmdResult<Summary> {
    with_ws(&state, summary)
}

#[tauri::command]
fn get_resume(state: State<AppState>, id: String) -> CmdResult<Value> {
    with_ws(&state, |ws| ws.resume(&id))
}

#[tauri::command]
fn get_schema(state: State<AppState>, template: String) -> CmdResult<Value> {
    with_ws(&state, |ws| ws.schema(&template))
}

#[tauri::command]
fn get_profile(state: State<AppState>) -> CmdResult<String> {
    with_ws(&state, |ws| Ok(fs::read_to_string(ws.root().join("profile.yaml"))?))
}

#[tauri::command]
fn save_profile(state: State<AppState>, text: String) -> CmdResult<()> {
    with_ws(&state, |ws| {
        serde_yaml::from_str::<Value>(&text)?;
        Ok(fs::write(ws.root().join("profile.yaml"), text)?)
    })
}

#[tauri::command]
fn profile_visibility(state: State<AppState>) -> CmdResult<ProfileVisibility> {
    with_ws(&state, |ws| ws.profile_visibility())
}

#[tauri::command]
fn set_profile_hidden(state: State<AppState>, template: String, hidden: Vec<String>) -> CmdResult<()> {
    with_ws(&state, |ws| ws.set_hidden_profile_fields(&template, &hidden))
}

#[tauri::command]
fn save_resume(state: State<AppState>, id: String, value: Value) -> CmdResult<Vec<SchemaError>> {
    with_ws(&state, |ws| ws.save_resume(&id, &value))
}

#[tauri::command]
fn new_resume(state: State<AppState>, id: String, template: String) -> CmdResult<ResumeInfo> {
    with_ws(&state, |ws| ws.new_resume(&id, &template))
}

#[tauri::command]
fn duplicate_resume(state: State<AppState>, id: String, from: String) -> CmdResult<ResumeInfo> {
    with_ws(&state, |ws| ws.duplicate_resume(&id, &from))
}

#[tauri::command]
fn delete_resume(state: State<AppState>, id: String) -> CmdResult<()> {
    with_ws(&state, |ws| ws.delete_resume(&id))
}

/// Renders a resume with unsaved content.
#[tauri::command]
fn preview_resume(state: State<AppState>, id: String, value: Value) -> CmdResult<Preview> {
    with_ws(&state, |ws| {
        let overlay = Overlay::from([(format!("resumes/{id}.yaml"), serde_yaml::to_string(&value)?)]);
        let out = compile(ws, &id, &overlay, Formats::SVG)?;
        Ok(Preview { svg: out.svg, diagnostics: out.diagnostics })
    })
}

#[tauri::command]
fn export_pdf(state: State<AppState>, id: String, value: Value, path: PathBuf) -> CmdResult<Vec<Diagnostic>> {
    with_ws(&state, |ws| {
        let overlay = Overlay::from([(format!("resumes/{id}.yaml"), serde_yaml::to_string(&value)?)]);
        let out = compile(ws, &id, &overlay, Formats::PDF)?;
        if let Some(pdf) = &out.pdf {
            fs::write(&path, pdf)?;
        }
        Ok(out.diagnostics)
    })
}

/// Saves the PDF to the default `pdfs/<template>/<id>.pdf`; returns its absolute path.
#[tauri::command]
fn save_pdf(state: State<AppState>, id: String, value: Value) -> CmdResult<(Vec<Diagnostic>, Option<PathBuf>)> {
    with_ws(&state, |ws| {
        let overlay = Overlay::from([(format!("resumes/{id}.yaml"), serde_yaml::to_string(&value)?)]);
        let out = compile(ws, &id, &overlay, Formats::PDF)?;
        let Some(pdf) = &out.pdf else { return Ok((out.diagnostics, None)) };
        let path = ws.root().join(ws.pdf_path(&id)?);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(&path, pdf)?;
        Ok((out.diagnostics, Some(path)))
    })
}

#[tauri::command]
fn rename_resume(state: State<AppState>, id: String, new_id: String) -> CmdResult<ResumeInfo> {
    with_ws(&state, |ws| ws.rename_resume(&id, &new_id))
}

#[tauri::command]
fn update_template(
    state: State<AppState>,
    id: String,
    new_id: String,
    name: String,
    description: String,
) -> CmdResult<TemplateInfo> {
    with_ws(&state, |ws| ws.update_template(&id, &new_id, &name, &description))
}

#[tauri::command]
fn delete_template(state: State<AppState>, id: String) -> CmdResult<Vec<String>> {
    with_ws(&state, |ws| ws.delete_template(&id))
}

#[tauri::command]
fn get_memory(state: State<AppState>) -> CmdResult<(String, MemoryStatus)> {
    with_ws(&state, |ws| {
        let text = ws.memory();
        let status = memory::status(&text);
        Ok((text, status))
    })
}

#[tauri::command]
fn save_memory(state: State<AppState>, text: String) -> CmdResult<MemoryStatus> {
    with_ws(&state, |ws| ws.save_memory(&text))
}

#[tauri::command]
fn compact_memory(state: State<AppState>) -> CmdResult<Remembered> {
    with_ws(&state, |ws| ws.compact_memory())
}

#[tauri::command]
fn new_template(state: State<AppState>, id: String, name: String, from: Option<String>) -> CmdResult<TemplateInfo> {
    with_ws(&state, |ws| ws.new_template(&id, &name, from.as_deref()))
}

/// Text files in a template directory (relative names).
#[tauri::command]
fn template_files(state: State<AppState>, template: String) -> CmdResult<Vec<String>> {
    with_ws(&state, |ws| {
        let mut names = Vec::new();
        for entry in fs::read_dir(ws.template_dir(&template))? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let text = [".typ", ".yaml", ".json"].iter().any(|ext| name.ends_with(ext));
            if entry.file_type()?.is_file() && text {
                names.push(name);
            }
        }
        // template.typ first, then the rest alphabetically.
        names.sort_by_key(|n| (n != "template.typ", n.clone()));
        Ok(names)
    })
}

fn template_file(ws: &Workspace, template: &str, name: &str) -> anyhow::Result<PathBuf> {
    if name.contains(['/', '\\']) || name.starts_with('.') {
        anyhow::bail!("invalid file name `{name}`");
    }
    Ok(ws.template_dir(template).join(name))
}

#[tauri::command]
fn read_template_file(state: State<AppState>, template: String, name: String) -> CmdResult<String> {
    with_ws(&state, |ws| Ok(fs::read_to_string(template_file(ws, &template, &name)?)?))
}

#[tauri::command]
fn write_template_file(state: State<AppState>, template: String, name: String, text: String) -> CmdResult<()> {
    with_ws(&state, |ws| Ok(fs::write(template_file(ws, &template, &name)?, text)?))
}

/// Renders a template against its sample data, with unsaved file contents.
#[tauri::command]
fn preview_template(
    state: State<AppState>,
    template: String,
    files: std::collections::HashMap<String, String>,
) -> CmdResult<Preview> {
    with_ws(&state, |ws| {
        let overlay: Overlay = files
            .into_iter()
            .map(|(name, text)| (format!("templates/{template}/{name}"), text))
            .collect();
        let out = compile_template_preview(ws, &template, &overlay, Formats::SVG)?;
        Ok(Preview { svg: out.svg, diagnostics: out.diagnostics })
    })
}

/// App settings live in `<config>/settings.json`; user themes are `<config>/themes/*.json`.
fn config_dir(app: &AppHandle) -> CmdResult<PathBuf> {
    let dir = app.path().app_config_dir().map_err(err)?;
    fs::create_dir_all(dir.join("themes")).map_err(err)?;
    Ok(dir)
}

#[derive(Serialize)]
struct Settings {
    settings: Value,
    /// Custom themes; files that fail to parse are skipped.
    themes: Vec<Value>,
}

#[tauri::command]
fn load_settings(app: AppHandle) -> CmdResult<Settings> {
    let dir = config_dir(&app)?;
    let settings = fs::read_to_string(dir.join("settings.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(Value::Object(Default::default()));
    let mut themes = Vec::new();
    for entry in fs::read_dir(dir.join("themes")).map_err(err)? {
        let path = entry.map_err(err)?.path();
        if path.extension().is_some_and(|e| e == "json") {
            if let Some(theme) = fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()) {
                themes.push(theme);
            }
        }
    }
    Ok(Settings { settings, themes })
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Value) -> CmdResult<()> {
    let text = serde_json::to_string_pretty(&settings).map_err(err)?;
    fs::write(config_dir(&app)?.join("settings.json"), text).map_err(err)
}

/// Writes a custom theme to `<config>/themes/<id>.json`.
#[tauri::command]
fn save_theme(app: AppHandle, theme: Value) -> CmdResult<()> {
    let id = theme.get("id").and_then(Value::as_str).ok_or("theme needs an id")?;
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(format!("invalid theme id `{id}`"));
    }
    let text = serde_json::to_string_pretty(&theme).map_err(err)?;
    fs::write(config_dir(&app)?.join("themes").join(format!("{id}.json")), text).map_err(err)
}

#[tauri::command]
fn open_themes_dir(app: AppHandle) -> CmdResult<()> {
    let dir = config_dir(&app)?.join("themes");
    tauri_plugin_opener::open_path(dir, None::<&str>).map_err(err)
}

/// Syntax highlighting for the template editor, using Typst's own parser.
#[tauri::command]
fn highlight_typst(text: String) -> Vec<dresume_core::Token> {
    dresume_core::highlight_typst(&text)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::default())
        .manage(WatchState::default())
        .manage(agent::AgentState::default())
        .invoke_handler(tauri::generate_handler![
            open_workspace,
            workspace_summary,
            get_resume,
            get_schema,
            get_profile,
            save_profile,
            profile_visibility,
            set_profile_hidden,
            save_resume,
            new_resume,
            duplicate_resume,
            delete_resume,
            preview_resume,
            export_pdf,
            save_pdf,
            rename_resume,
            update_template,
            delete_template,
            get_memory,
            save_memory,
            compact_memory,
            new_template,
            template_files,
            read_template_file,
            write_template_file,
            preview_template,
            load_settings,
            save_settings,
            save_theme,
            open_themes_dir,
            highlight_typst,
            agent::list_agents,
            agent::agent_models,
            agent::agent_start,
            agent::agent_send,
            agent::agent_respond,
            agent::agent_cancel,
            agent::agent_stop,
            agent::agent_changes,
            agent::agent_keep,
            agent::agent_undo,
            agent::load_chat,
            agent::save_chat,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
