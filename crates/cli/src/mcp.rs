//! `dresume mcp`: a Model Context Protocol server over stdio, so agents get typed tools
//! (validate, build, schema…) instead of parsing shell output.

use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use dresume_core::{Formats, Overlay, Workspace, compile, compile_template_preview};
use serde_json::{Value, json};

const PROTOCOL_VERSION: &str = "2025-06-18";

/// Serves JSON-RPC lines on stdin/stdout until stdin closes.
pub fn serve(start: &Path) -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
        // Notifications (no id) need no answer.
        let Some(id) = msg.get("id").cloned() else { continue };
        let method = msg["method"].as_str().unwrap_or_default();
        let reply = match handle(start, method, &msg["params"]) {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err(e) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": format!("{e:#}") } }),
        };
        writeln!(stdout, "{reply}")?;
        stdout.flush()?;
    }
    Ok(())
}

fn handle(start: &Path, method: &str, params: &Value) -> Result<Value> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": params["protocolVersion"].as_str().unwrap_or(PROTOCOL_VERSION),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "dresume", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "Tools for a dresume workspace (resumes in YAML, templates in Typst). \
                Read AGENTS.md in the workspace for the file layout.",
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => {
            let name = params["name"].as_str().unwrap_or_default();
            let args = &params["arguments"];
            // Tool failures are results with isError, so the agent can read and fix them.
            Ok(match call(start, name, args) {
                Ok((value, ok)) => json!({
                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&value)? }],
                    "structuredContent": value,
                    "isError": !ok,
                }),
                Err(e) => json!({ "content": [{ "type": "text", "text": format!("{e:#}") }], "isError": true }),
            })
        }
        _ => bail!("method not found: {method}"),
    }
}

fn tools() -> Value {
    let id = json!({ "type": "string", "description": "Resume id (file name in resumes/ without .yaml)" });
    let template = json!({ "type": "string", "description": "Template id (folder in templates/)" });
    json!([
        {
            "name": "list",
            "description": "List templates and the resumes linked to each.",
            "inputSchema": { "type": "object", "properties": {} },
        },
        {
            "name": "schema",
            "description": "JSON Schema a template accepts for resume content.",
            "inputSchema": { "type": "object", "properties": { "template": template }, "required": ["template"] },
        },
        {
            "name": "show",
            "description": "A resume's content as JSON.",
            "inputSchema": { "type": "object", "properties": { "id": id }, "required": ["id"] },
        },
        {
            "name": "validate",
            "description": "Validate resumes against their template schema (all when ids is empty).",
            "inputSchema": { "type": "object", "properties": { "ids": { "type": "array", "items": { "type": "string" } } } },
        },
        {
            "name": "build",
            "description": "Compile a resume to output/<id>.pdf and report Typst errors with file and line.",
            "inputSchema": { "type": "object", "properties": { "id": id }, "required": ["id"] },
        },
        {
            "name": "preview_template",
            "description": "Compile a template with its sample.yaml to check it for errors.",
            "inputSchema": { "type": "object", "properties": { "template": template }, "required": ["template"] },
        },
    ])
}

/// Runs a tool; the bool is false when it ran but found problems.
fn call(start: &Path, name: &str, args: &Value) -> Result<(Value, bool)> {
    let ws = Workspace::discover(start)?;
    let str_arg = |key: &str| args[key].as_str().with_context(|| format!("missing `{key}`"));
    match name {
        "list" => Ok((json!({ "templates": ws.templates()? }), true)),
        "schema" => Ok((ws.schema(str_arg("template")?)?, true)),
        "show" => Ok((ws.resume(str_arg("id")?)?, true)),
        "validate" => {
            let mut ids: Vec<String> =
                args["ids"].as_array().into_iter().flatten().filter_map(Value::as_str).map(Into::into).collect();
            if ids.is_empty() {
                ids = ws.resumes()?.into_iter().map(|r| r.id).collect();
            }
            let mut ok = true;
            let mut results = Vec::new();
            for id in ids {
                let errors = ws.validate_resume(&id)?;
                ok &= errors.is_empty();
                results.push(json!({ "id": id, "ok": errors.is_empty(), "errors": errors }));
            }
            Ok((json!({ "results": results }), ok))
        }
        "build" => {
            let id = str_arg("id")?;
            let output = compile(&ws, id, &Overlay::new(), Formats::PDF)?;
            let path: Option<PathBuf> = match &output.pdf {
                Some(pdf) => {
                    let dir = ws.root().join("output");
                    fs::create_dir_all(&dir)?;
                    let path = dir.join(format!("{id}.pdf"));
                    fs::write(&path, pdf)?;
                    Some(path)
                }
                None => None,
            };
            Ok((json!({ "id": id, "ok": output.ok(), "pdf": path, "diagnostics": output.diagnostics }), output.ok()))
        }
        "preview_template" => {
            let template = str_arg("template")?;
            let output = compile_template_preview(&ws, template, &Overlay::new(), Formats::PDF)?;
            Ok((json!({ "template": template, "ok": output.ok(), "diagnostics": output.diagnostics }), output.ok()))
        }
        _ => bail!("unknown tool `{name}`"),
    }
}
