//! Core library for dresume: workspace model, validation and Typst compilation.
//!
//! A workspace is a plain directory that both humans and AI agents can edit:
//!
//! ```text
//! dresume.yaml              workspace marker
//! profile.yaml              personal info shared by every resume
//! templates/<id>/
//!   template.yaml           name, description, `profile.hide` (profile keys to leave out)
//!   template.typ            layout
//!   schema.json             JSON Schema for resume content
//!   sample.yaml             sample content used to preview the template
//! resumes/<id>.yaml         content only; `template: <id>` links it
//! fonts/                    optional extra fonts
//! memory.md                 facts agents remember about the user (see [`memory`])
//! ```
//!
//! Templates read their data through `sys.inputs.resume` and
//! `sys.inputs.profile`, which hold workspace-rooted paths to YAML files.

mod compile;
mod highlight;
pub mod memory;
mod profile;
mod world;
mod workspace;

pub use compile::{Diagnostic, Formats, Output, Severity, compile, compile_template_preview};
pub use highlight::{Token, highlight_typst};
pub use profile::{ProfileVisibility, TemplateVisibility};
pub use workspace::{
    ResumeInfo, SchemaError, TemplateInfo, Workspace, WorkspaceManifest, validate_value,
};
pub use world::Overlay;
