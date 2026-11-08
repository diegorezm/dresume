use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use dresume_core::{
    Formats, Output, Overlay, Severity, Workspace, compile, compile_template_preview,
};
use serde_json::json;

/// Resume workspaces powered by Typst.
#[derive(Parser)]
#[command(name = "dresume", version)]
struct Cli {
    /// Workspace directory (defaults to the nearest parent with dresume.yaml).
    #[arg(short = 'w', long, global = true)]
    workspace: Option<PathBuf>,

    /// Print machine-readable JSON instead of text.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Pdf,
    Svg,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new workspace in the given directory.
    Init { dir: PathBuf, #[arg(long, default_value = "My resumes")] name: String },
    /// List templates and the resumes linked to each.
    List,
    /// Compile resumes to PDF (all when no id is given).
    Build {
        ids: Vec<String>,
        /// Output directory.
        #[arg(short, long, default_value = "output")]
        out: PathBuf,
        /// `svg` writes one `<id>-<page>.svg` per page.
        #[arg(short, long, value_enum, default_value_t = Format::Pdf)]
        format: Format,
    },
    /// Print a resume's content as JSON.
    Show { id: String },
    /// Compile a template with its own sample.yaml, to check it while editing.
    PreviewTemplate {
        template: String,
        /// Output PDF path.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Validate resumes against their template schema (all when no id is given).
    Validate { ids: Vec<String> },
    /// Create a resume from a template's sample content.
    New { id: String, #[arg(short, long)] template: String },
    /// Create a resume with a copy of another resume's content.
    Duplicate { id: String, #[arg(long)] from: String },
    /// Create a template, optionally copying an existing one (`--from @starter` copies the built-in Basic).
    NewTemplate { id: String, #[arg(long)] name: String, #[arg(long)] from: Option<String> },
    /// Show which profile fields each template shows; with --hide, set them for one template.
    ProfileFields {
        template: Option<String>,
        /// Comma-separated profile keys to hide (empty string shows everything).
        #[arg(long)]
        hide: Option<String>,
    },
    /// Print a template's JSON Schema.
    Schema { template: String },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            if cli.json {
                println!("{}", json!({ "error": format!("{e:#}") }));
            } else {
                eprintln!("error: {e:#}");
            }
            ExitCode::FAILURE
        }
    }
}

fn workspace(cli: &Cli) -> Result<Workspace> {
    let start = cli.workspace.clone().unwrap_or_else(|| PathBuf::from("."));
    Workspace::discover(&start)
}

/// Returns `Ok(false)` when the command ran but found problems.
fn run(cli: &Cli) -> Result<bool> {
    match &cli.command {
        Command::Init { dir, name } => {
            let ws = Workspace::init(dir, name)?;
            emit(cli, json!({ "root": ws.root() }), || println!("created {}", dir.display()));
            Ok(true)
        }
        Command::List => {
            let ws = workspace(cli)?;
            let templates = ws.templates()?;
            emit(cli, json!({ "templates": templates }), || {
                for t in &templates {
                    println!("{} ({})", t.name, t.id);
                    for r in &t.resumes {
                        println!("  - {r}");
                    }
                }
            });
            Ok(true)
        }
        Command::Build { ids, out, format } => {
            let ws = workspace(cli)?;
            let ids = ids_or_all(&ws, ids)?;
            let out = if out.is_absolute() { out.clone() } else { ws.root().join(out) };
            fs::create_dir_all(&out)?;
            let mut results = Vec::new();
            let mut ok = true;
            for id in &ids {
                let formats = match format {
                    Format::Pdf => Formats::PDF,
                    Format::Svg => Formats::SVG,
                };
                let output = compile(&ws, id, &Overlay::new(), formats)?;
                let path = match format {
                    Format::Pdf => out.join(format!("{id}.pdf")),
                    Format::Svg => out.join(format!("{id}-1.svg")),
                };
                if let Some(pdf) = &output.pdf {
                    fs::write(&path, pdf).with_context(|| path.display().to_string())?;
                }
                for (i, page) in output.svg.iter().enumerate() {
                    let page_path = out.join(format!("{id}-{}.svg", i + 1));
                    fs::write(&page_path, page).with_context(|| page_path.display().to_string())?;
                }
                let written = output.pdf.is_some() || !output.svg.is_empty();
                ok &= output.ok();
                if !cli.json {
                    report(id, &output, &path);
                }
                results.push(json!({
                    "id": id,
                    "ok": output.ok(),
                    "output": written.then(|| path.clone()),
                    "pages": output.svg.len().max(output.pdf.is_some() as usize),
                    "diagnostics": output.diagnostics,
                }));
            }
            if cli.json {
                println!("{}", json!({ "results": results }));
            }
            Ok(ok)
        }
        Command::PreviewTemplate { template, out } => {
            let ws = workspace(cli)?;
            let output = compile_template_preview(&ws, template, &Overlay::new(), Formats::PDF)?;
            let path = match out {
                Some(p) => p.clone(),
                None => {
                    fs::create_dir_all(ws.root().join("output"))?;
                    ws.root().join("output").join(format!("template-{template}.pdf"))
                }
            };
            if let Some(pdf) = &output.pdf {
                fs::write(&path, pdf).with_context(|| path.display().to_string())?;
            }
            if cli.json {
                println!(
                    "{}",
                    json!({
                        "template": template,
                        "ok": output.ok(),
                        "pdf": output.pdf.is_some().then(|| path.clone()),
                        "diagnostics": output.diagnostics,
                    })
                );
            } else {
                report(template, &output, &path);
            }
            Ok(output.ok())
        }
        Command::Validate { ids } => {
            let ws = workspace(cli)?;
            let mut results = Vec::new();
            let mut ok = true;
            for id in ids_or_all(&ws, ids)? {
                let errors = ws.validate_resume(&id)?;
                ok &= errors.is_empty();
                if !cli.json {
                    if errors.is_empty() {
                        println!("ok    {id}");
                    }
                    for e in &errors {
                        println!("error {id}{}: {}", e.path, e.message);
                    }
                }
                results.push(json!({ "id": id, "ok": errors.is_empty(), "errors": errors }));
            }
            if cli.json {
                println!("{}", json!({ "results": results }));
            }
            Ok(ok)
        }
        Command::New { id, template } => {
            let info = workspace(cli)?.new_resume(id, template)?;
            emit(cli, json!(info), || println!("created {}", info.path));
            Ok(true)
        }
        Command::Duplicate { id, from } => {
            let info = workspace(cli)?.duplicate_resume(id, from)?;
            emit(cli, json!(info), || println!("created {}", info.path));
            Ok(true)
        }
        Command::NewTemplate { id, name, from } => {
            let info = workspace(cli)?.new_template(id, name, from.as_deref())?;
            emit(cli, json!(info), || println!("created templates/{}", info.id));
            Ok(true)
        }
        Command::Show { id } => {
            let value = workspace(cli)?.resume(id)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
            Ok(true)
        }
        Command::ProfileFields { template, hide } => {
            let ws = workspace(cli)?;
            if let (Some(template), Some(hide)) = (template, hide) {
                let keys: Vec<String> =
                    hide.split(',').map(str::trim).filter(|k| !k.is_empty()).map(Into::into).collect();
                ws.set_hidden_profile_fields(template, &keys)?;
            }
            let vis = ws.profile_visibility()?;
            emit(cli, json!(vis), || {
                println!("fields: {}", vis.fields.join(", "));
                for t in &vis.templates {
                    let hidden = if t.hidden.is_empty() { "-".into() } else { t.hidden.join(", ") };
                    println!("{} ({}): hides {hidden}", t.name, t.id);
                }
            });
            Ok(true)
        }
        Command::Schema { template } => {
            let schema = workspace(cli)?.schema(template)?;
            println!("{}", serde_json::to_string_pretty(&schema)?);
            Ok(true)
        }
    }
}

fn ids_or_all(ws: &Workspace, ids: &[String]) -> Result<Vec<String>> {
    if !ids.is_empty() {
        return Ok(ids.to_vec());
    }
    Ok(ws.resumes()?.into_iter().map(|r| r.id).collect())
}

fn emit(cli: &Cli, value: serde_json::Value, text: impl FnOnce()) {
    if cli.json {
        println!("{value}");
    } else {
        text();
    }
}

fn report(id: &str, output: &Output, path: &std::path::Path) {
    if output.ok() {
        println!("ok    {id} -> {}", path.display());
    } else {
        println!("error {id}");
    }
    for d in &output.diagnostics {
        let level = match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let at = match (&d.file, d.line, d.column) {
            (Some(f), Some(l), Some(c)) => format!("{f}:{l}:{c}: "),
            (Some(f), _, _) => format!("{f}: "),
            _ => String::new(),
        };
        println!("  {level}: {at}{}", d.message);
        for h in &d.hints {
            println!("    hint: {h}");
        }
    }
}
