//! Before/after view of what an agent changed in a workspace, with undo.
//!
//! A snapshot keeps the text files of the workspace in memory when the user sends a prompt;
//! comparing it with the disk later lists every file the agent created, edited or deleted.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::Serialize;
use similar::TextDiff;

/// Only text the agent is expected to touch; everything else is ignored.
const EXTENSIONS: &[&str] = &["yaml", "yml", "typ", "json", "md", "toml", "txt"];
const IGNORED_DIRS: &[&str] = &[".dresume", ".git", "output", "node_modules", "fonts"];
const MAX_SIZE: u64 = 1 << 20;

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    /// Workspace-relative path (with `/`) → content.
    files: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FileChange {
    pub path: String,
    /// `None`: the file didn't exist before / was deleted.
    pub before: Option<String>,
    pub after: Option<String>,
    /// Unified diff with a little context.
    pub diff: String,
}

impl Snapshot {
    pub fn take(root: &Path) -> Self {
        let mut files = BTreeMap::new();
        collect(root, root, &mut files);
        Self { files }
    }

    /// Files that differ between the snapshot and the disk now.
    pub fn changes(&self, root: &Path) -> Vec<FileChange> {
        let now = Self::take(root);
        let mut paths: Vec<&String> = self.files.keys().chain(now.files.keys()).collect();
        paths.sort();
        paths.dedup();
        paths
            .into_iter()
            .filter_map(|path| {
                let before = self.files.get(path);
                let after = now.files.get(path);
                if before == after {
                    return None;
                }
                let diff = TextDiff::from_lines(before.map_or("", |s| s), after.map_or("", |s| s))
                    .unified_diff()
                    .context_radius(2)
                    .header(path, path)
                    .to_string();
                Some(FileChange { path: path.clone(), before: before.cloned(), after: after.cloned(), diff })
            })
            .collect()
    }

    /// Puts every changed file back as it was: rewrites edited/deleted ones, removes created ones.
    /// Returns the restored paths.
    pub fn restore(&self, root: &Path) -> Result<Vec<String>> {
        let changes = self.changes(root);
        for change in &changes {
            let path = root.join(&change.path);
            match &change.before {
                Some(text) => {
                    if let Some(dir) = path.parent() {
                        fs::create_dir_all(dir)?;
                    }
                    fs::write(&path, text)?;
                }
                None => fs::remove_file(&path)?,
            }
        }
        Ok(changes.into_iter().map(|c| c.path).collect())
    }
}

fn collect(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else { continue };
        let name = entry.file_name();
        if kind.is_dir() {
            if dir == root && IGNORED_DIRS.iter().any(|d| name == *d) {
                continue;
            }
            collect(root, &path, out);
        } else if kind.is_file() {
            let text_ext = path.extension().and_then(|e| e.to_str()).is_some_and(|e| EXTENSIONS.contains(&e));
            let small = entry.metadata().is_ok_and(|m| m.len() <= MAX_SIZE);
            if !(text_ext && small) {
                continue;
            }
            if let (Ok(rel), Ok(text)) = (path.strip_prefix(root), fs::read_to_string(&path)) {
                out.insert(rel.to_string_lossy().replace('\\', "/"), text);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_and_undoes_changes() {
        let root = std::env::temp_dir().join(format!("dresume-snapshot-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("resumes")).unwrap();
        fs::create_dir_all(root.join("output")).unwrap();
        fs::write(root.join("resumes/a.yaml"), "name: x\n").unwrap();
        fs::write(root.join("profile.yaml"), "email: a@b.c\n").unwrap();

        let snap = Snapshot::take(&root);
        fs::write(root.join("resumes/a.yaml"), "name: y\n").unwrap();
        fs::write(root.join("resumes/b.yaml"), "new\n").unwrap();
        fs::remove_file(root.join("profile.yaml")).unwrap();
        fs::write(root.join("output/a.json"), "ignored").unwrap();

        let changes = snap.changes(&root);
        let paths: Vec<&str> = changes.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(paths, ["profile.yaml", "resumes/a.yaml", "resumes/b.yaml"]);
        assert!(changes[1].diff.contains("-name: x") && changes[1].diff.contains("+name: y"));

        snap.restore(&root).unwrap();
        assert!(snap.changes(&root).is_empty());
        assert!(!root.join("resumes/b.yaml").exists());
        fs::remove_dir_all(&root).unwrap();
    }
}
