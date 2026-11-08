use anyhow::Result;
use serde::Serialize;
use typst::WorldExt;
use typst::diag::{self, SourceDiagnostic};
use typst::World;
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;
use typst_svg::SvgOptions;

use crate::workspace::{PROFILE, Workspace, resume_path};
use crate::world::{Overlay, ResumeWorld};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// A compiler message, located in a workspace file when possible.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub hints: Vec<String>,
    /// Workspace-relative file path (absent for package or detached errors).
    pub file: Option<String>,
    /// 1-based line and column.
    pub line: Option<usize>,
    pub column: Option<usize>,
}

/// The result of a compilation. `svg` has one entry per page.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Output {
    #[serde(skip)]
    pub pdf: Option<Vec<u8>>,
    pub svg: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Output {
    pub fn ok(&self) -> bool {
        !self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Formats {
    pub pdf: bool,
    pub svg: bool,
}

impl Formats {
    pub const PDF: Self = Self { pdf: true, svg: false };
    pub const SVG: Self = Self { pdf: false, svg: true };
}

/// Compiles a resume with its template.
pub fn compile(ws: &Workspace, resume: &str, overlay: &Overlay, formats: Formats) -> Result<Output> {
    let template = match overlay.get(&resume_path(resume)) {
        Some(text) => template_of(&serde_yaml::from_str(text)?)?,
        None => ws.resume_info(resume)?.template,
    };
    run(ws, &template, &resume_path(resume), overlay, formats)
}

/// Compiles a template against its own `sample.yaml`, for template editing.
pub fn compile_template_preview(
    ws: &Workspace,
    template: &str,
    overlay: &Overlay,
    formats: Formats,
) -> Result<Output> {
    run(ws, template, &format!("templates/{template}/sample.yaml"), overlay, formats)
}

fn template_of(value: &serde_json::Value) -> Result<String> {
    value
        .get("template")
        .and_then(|v| v.as_str())
        .map(Into::into)
        .ok_or_else(|| anyhow::anyhow!("resume has no `template` field"))
}

fn run(
    ws: &Workspace,
    template: &str,
    data: &str,
    overlay: &Overlay,
    formats: Formats,
) -> Result<Output> {
    let main = format!("templates/{template}/template.typ");
    let inputs = [("resume", &*format!("/{data}")), ("profile", &*format!("/{PROFILE}"))];
    let world = ResumeWorld::new(ws.root(), &main, &inputs, overlay)?;

    let warned = typst::compile::<PagedDocument>(&world);
    let mut out = Output::default();
    out.diagnostics.extend(warned.warnings.iter().map(|d| convert(&world, d)));
    match warned.output {
        Ok(doc) => {
            if formats.svg {
                let opts = SvgOptions::default();
                out.svg = doc.pages().iter().map(|p| typst_svg::svg(p, &opts)).collect();
            }
            if formats.pdf {
                match typst_pdf::pdf(&doc, &PdfOptions::default()) {
                    Ok(bytes) => out.pdf = Some(bytes),
                    Err(errs) => out.diagnostics.extend(errs.iter().map(|d| convert(&world, d))),
                }
            }
        }
        Err(errs) => out.diagnostics.extend(errs.iter().map(|d| convert(&world, d))),
    }
    Ok(out)
}

fn convert(world: &ResumeWorld, d: &SourceDiagnostic) -> Diagnostic {
    let severity = match d.severity {
        diag::Severity::Error => Severity::Error,
        diag::Severity::Warning => Severity::Warning,
    };
    let id = d.span.id();
    let (mut line, mut column) = (None, None);
    if let (Some(id), Some(range)) = (id, world.range(d.span)) {
        if let Ok(source) = world.source(id) {
            if let Some((l, c)) = source.lines().byte_to_line_column(range.start) {
                (line, column) = (Some(l + 1), Some(c + 1));
            }
        }
    }
    Diagnostic {
        severity,
        message: d.message.to_string(),
        hints: d.hints.iter().map(|h| h.v.to_string()).collect(),
        file: id.and_then(|id| world.relative_path(id)),
        line,
        column,
    }
}
