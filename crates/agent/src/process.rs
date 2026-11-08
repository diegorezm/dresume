//! Spawning agent CLIs: PATH lookup, Windows `.cmd` shims, no console windows, line I/O.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

/// Windows: don't flash a console window for each child.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A command for `program`, going through `cmd /C` for npm-style `.cmd`/`.bat` shims,
/// which can't be executed directly. Keep arguments short and quote-free for those
/// (cmd re-parses them); pass long text through stdin or files instead.
pub fn command(program: &Path) -> Command {
    let shim = program
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat"));
    let mut cmd = if shim {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(program);
        c
    } else {
        Command::new(program)
    };
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.kill_on_drop(true);
    cmd
}

/// A running agent with piped stdio, read line by line.
pub struct Proc {
    pub child: Child,
    pub stdin: ChildStdin,
    pub lines: Lines<BufReader<ChildStdout>>,
}

pub fn spawn(program: &str, args: &[&str], cwd: &Path) -> Result<Proc> {
    let path = which::which(program).with_context(|| format!("`{program}` not found on PATH"))?;
    let mut child = command(&path)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("failed to start {}", path.display()))?;
    let stdin = child.stdin.take().context("no stdin")?;
    let stdout = child.stdout.take().context("no stdout")?;
    Ok(Proc { child, stdin, lines: BufReader::new(stdout).lines() })
}

impl Proc {
    /// Writes one JSON value as a line.
    pub async fn write(&mut self, value: &serde_json::Value) -> Result<()> {
        let mut line = serde_json::to_vec(value)?;
        line.push(b'\n');
        self.stdin.write_all(&line).await?;
        self.stdin.flush().await?;
        Ok(())
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        // `cmd /C` shims leave the real agent running when only cmd is killed.
        #[cfg(windows)]
        if let Some(pid) = self.child.id() {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        let _ = self.child.start_kill();
    }
}

#[cfg(windows)]
use std::os::windows::process::CommandExt as _;

/// First line of `<program> --version`, or `None` if it fails or hangs.
pub async fn version(program: &Path, timeout: Duration) -> Option<String> {
    let out = command(program)
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    let out = tokio::time::timeout(timeout, out).await.ok()?.ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().map(str::trim).find(|l| !l.is_empty()).map(Into::into)
}
