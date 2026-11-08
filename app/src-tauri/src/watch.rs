//! Watches the open workspace so edits made outside the app (an agent, an editor, git) show up
//! live. Emits `workspace-changed` with workspace-relative paths, batched.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Directories whose changes the UI doesn't care about.
const IGNORED: &[&str] = &[".dresume", ".git", "output", "node_modules"];

#[derive(Clone, Serialize)]
struct Changed {
    paths: Vec<String>,
}

/// Starts watching `root`; watching stops when the returned watcher is dropped.
pub fn watch(app: AppHandle, root: PathBuf) -> notify::Result<RecommendedWatcher> {
    let (tx, rx) = mpsc::channel::<PathBuf>();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            if event.kind.is_access() {
                return;
            }
            for path in event.paths {
                let _ = tx.send(path);
            }
        }
    })?;
    watcher.watch(&root, RecursiveMode::Recursive)?;

    // Collects bursts (editors write several events per save) into one notification.
    // The thread ends when the watcher, and with it `tx`, is dropped.
    std::thread::spawn(move || {
        while let Ok(first) = rx.recv() {
            let mut paths = BTreeSet::new();
            paths.extend(relative(&root, &first));
            while let Ok(path) = rx.recv_timeout(Duration::from_millis(150)) {
                paths.extend(relative(&root, &path));
            }
            if !paths.is_empty() {
                let _ = app.emit("workspace-changed", Changed { paths: paths.into_iter().collect() });
            }
        }
    });
    Ok(watcher)
}

fn relative(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let first = rel.components().next()?.as_os_str().to_string_lossy();
    if IGNORED.contains(&first.as_ref()) {
        return None;
    }
    Some(rel.to_string_lossy().replace('\\', "/"))
}
