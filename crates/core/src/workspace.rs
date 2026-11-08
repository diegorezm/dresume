use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MANIFEST: &str = "dresume.yaml";
pub const PROFILE: &str = "profile.yaml";
pub const TEMPLATES_DIR: &str = "templates";
pub const RESUMES_DIR: &str = "resumes";

/// Guide for AI agents written into new workspaces.
const AGENTS_MD: &str = include_str!("../assets/AGENTS.md");

/// Template written into new workspaces.
const STARTER_TEMPLATE: &str = "basic";
/// `from` value of [`Workspace::new_template`] that copies the built-in starter.
pub const STARTER_SOURCE: &str = "@starter";
const STARTER_FILES: [(&str, &str); 4] = [
    ("template.typ", include_str!("../assets/starter/template.typ")),
    ("template.yaml", include_str!("../assets/starter/template.yaml")),
    ("schema.json", include_str!("../assets/starter/schema.json")),
    ("sample.yaml", include_str!("../assets/starter/sample.yaml")),
];

const STARTER_PROFILE: &str = r#"# Personal info shared by every resume.
name: Your Name
email: you@example.com
phone: "+55 11 90000-0000"
location: "São Paulo, BR"
website:
  label: "yoursite.dev"
  url: "https://yoursite.dev"
github:
  label: "github.com/you"
  url: "https://github.com/you"
linkedin:
  label: "linkedin.com/in/you"
  url: "https://linkedin.com/in/you"
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: u32,
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Ids of the resumes that use this template.
    #[serde(default)]
    pub resumes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeInfo {
    pub id: String,
    pub template: String,
    /// Workspace-relative path to the resume file.
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaError {
    /// JSON pointer to the offending value, e.g. `/experience/0/role`.
    pub path: String,
    pub message: String,
}

#[derive(Deserialize)]
struct TemplateMeta {
    name: String,
    #[serde(default)]
    description: String,
}

#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
    pub manifest: WorkspaceManifest,
}

impl Workspace {
    /// Opens the workspace containing `start`, searching parent directories.
    pub fn discover(start: &Path) -> Result<Self> {
        let start = dunce(start.canonicalize().with_context(|| start.display().to_string())?);
        for dir in start.ancestors() {
            if dir.join(MANIFEST).is_file() {
                return Self::open(dir);
            }
        }
        bail!("no {MANIFEST} found in {} or its parents", start.display())
    }

    pub fn open(root: &Path) -> Result<Self> {
        let manifest = read_yaml(&root.join(MANIFEST))?;
        Ok(Self { root: root.to_path_buf(), manifest })
    }

    /// Creates an empty workspace with a profile stub.
    pub fn init(root: &Path, name: &str) -> Result<Self> {
        if root.join(MANIFEST).exists() {
            bail!("{} already contains a workspace", root.display());
        }
        fs::create_dir_all(root.join(TEMPLATES_DIR))?;
        fs::create_dir_all(root.join(RESUMES_DIR))?;
        let manifest = WorkspaceManifest { name: name.into(), version: 1 };
        fs::write(root.join(MANIFEST), serde_yaml::to_string(&manifest)?)?;
        if !root.join("AGENTS.md").exists() {
            fs::write(root.join("AGENTS.md"), AGENTS_MD)?;
        }
        if !root.join(PROFILE).exists() {
            fs::write(root.join(PROFILE), STARTER_PROFILE)?;
        }
        let ws = Self { root: root.to_path_buf(), manifest };
        // Start with a working template and a resume, so there is something to edit right away.
        let dir = ws.template_dir(STARTER_TEMPLATE);
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
            for (name, text) in STARTER_FILES {
                fs::write(dir.join(name), text)?;
            }
            if ws.resumes()?.is_empty() {
                ws.new_resume("my-resume", STARTER_TEMPLATE)?;
            }
        }
        Ok(ws)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn templates(&self) -> Result<Vec<TemplateInfo>> {
        let resumes = self.resumes()?;
        let mut out = Vec::new();
        for id in subdirs(&self.root.join(TEMPLATES_DIR))? {
            let meta: TemplateMeta = read_yaml(&self.template_dir(&id).join("template.yaml"))?;
            let linked = resumes.iter().filter(|r| r.template == id).map(|r| r.id.clone());
            out.push(TemplateInfo {
                resumes: linked.collect(),
                id,
                name: meta.name,
                description: meta.description,
            });
        }
        Ok(out)
    }

    pub fn resumes(&self) -> Result<Vec<ResumeInfo>> {
        let dir = self.root.join(RESUMES_DIR);
        let mut out = Vec::new();
        if !dir.is_dir() {
            return Ok(out);
        }
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "yaml") {
                let id = path.file_stem().unwrap().to_string_lossy().into_owned();
                out.push(self.resume_info(&id)?);
            }
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    pub fn resume_info(&self, id: &str) -> Result<ResumeInfo> {
        let value = self.resume(id)?;
        let template = value
            .get("template")
            .and_then(Value::as_str)
            .with_context(|| format!("resume `{id}` has no `template` field"))?;
        Ok(ResumeInfo { id: id.into(), template: template.into(), path: resume_path(id) })
    }

    pub fn resume(&self, id: &str) -> Result<Value> {
        read_yaml(&self.root.join(resume_path(id)))
    }

    /// Writes a resume if it is valid against its template's schema.
    /// Returns the schema errors otherwise (nothing is written).
    pub fn save_resume(&self, id: &str, value: &Value) -> Result<Vec<SchemaError>> {
        check_id(id)?;
        let template = value
            .get("template")
            .and_then(Value::as_str)
            .context("resume needs a `template` field")?;
        let errors = validate_value(&self.schema(template)?, value);
        if errors.is_empty() {
            fs::write(self.root.join(resume_path(id)), serde_yaml::to_string(value)?)?;
        }
        Ok(errors)
    }

    /// Creates a resume from a template's sample content.
    pub fn new_resume(&self, id: &str, template: &str) -> Result<ResumeInfo> {
        check_id(id)?;
        let path = self.root.join(resume_path(id));
        if path.exists() {
            bail!("resume `{id}` already exists");
        }
        if !self.template_dir(template).is_dir() {
            bail!("template `{template}` not found");
        }
        let sample: Value = read_yaml(&self.template_dir(template).join("sample.yaml"))
            .unwrap_or_else(|_| Value::Object(Default::default()));
        let sample = sample.as_object().context("sample.yaml must be a mapping")?;
        // Keep `template` as the first key so the link is obvious when reading the file.
        let mut value = serde_json::Map::new();
        value.insert("template".into(), template.into());
        value.extend(sample.clone().into_iter().filter(|(k, _)| k != "template"));
        fs::write(&path, serde_yaml::to_string(&Value::Object(value))?)?;
        self.resume_info(id)
    }

    /// Creates a resume with a copy of another resume's content (same template).
    pub fn duplicate_resume(&self, id: &str, from: &str) -> Result<ResumeInfo> {
        check_id(id)?;
        let path = self.root.join(resume_path(id));
        if path.exists() {
            bail!("resume `{id}` already exists");
        }
        fs::copy(self.root.join(resume_path(from)), &path)
            .with_context(|| format!("copying resume `{from}`"))?;
        self.resume_info(id)
    }

    pub fn delete_resume(&self, id: &str) -> Result<()> {
        check_id(id)?;
        fs::remove_file(self.root.join(resume_path(id)))?;
        Ok(())
    }

    pub fn profile(&self) -> Result<Value> {
        read_yaml(&self.root.join(PROFILE))
    }

    pub fn template_dir(&self, id: &str) -> PathBuf {
        self.root.join(TEMPLATES_DIR).join(id)
    }

    pub fn schema(&self, template: &str) -> Result<Value> {
        let path = self.template_dir(template).join("schema.json");
        let text = fs::read_to_string(&path)
            .with_context(|| format!("template `{template}` has no schema.json"))?;
        Ok(serde_json::from_str(&text)?)
    }

    /// Validates a resume on disk against its template's schema.
    pub fn validate_resume(&self, id: &str) -> Result<Vec<SchemaError>> {
        let info = self.resume_info(id)?;
        Ok(validate_value(&self.schema(&info.template)?, &self.resume(id)?))
    }

    /// Creates a new template: a copy of template `from`, the built-in starter
    /// when `from` is `"@starter"`, or a minimal blank one.
    pub fn new_template(&self, id: &str, name: &str, from: Option<&str>) -> Result<TemplateInfo> {
        check_id(id)?;
        let dir = self.template_dir(id);
        if dir.exists() {
            bail!("template `{id}` already exists");
        }
        match from {
            Some(STARTER_SOURCE) => {
                fs::create_dir_all(&dir)?;
                for (file, text) in STARTER_FILES {
                    fs::write(dir.join(file), text)?;
                }
            }
            Some(src) => copy_dir(&self.template_dir(src), &dir)?,
            None => {
                fs::create_dir_all(&dir)?;
                fs::write(dir.join("template.typ"), BLANK_TEMPLATE)?;
                fs::write(dir.join("schema.json"), BLANK_SCHEMA)?;
                fs::write(dir.join("sample.yaml"), "summary: Write something about yourself.\n")?;
            }
        }
        let meta = format!("name: {}\ndescription: \"\"\n", serde_json::to_string(name)?);
        fs::write(dir.join("template.yaml"), meta)?;
        Ok(TemplateInfo {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            resumes: vec![],
        })
    }
}

pub(crate) fn resume_path(id: &str) -> String {
    format!("{RESUMES_DIR}/{id}.yaml")
}

pub fn validate_value(schema: &Value, value: &Value) -> Vec<SchemaError> {
    let validator = match jsonschema::validator_for(schema) {
        Ok(v) => v,
        Err(e) => {
            let message = format!("invalid schema: {e}");
            return vec![SchemaError { path: String::new(), message }];
        }
    };
    validator
        .iter_errors(value)
        .map(|e| SchemaError { path: e.instance_path.to_string(), message: e.to_string() })
        .collect()
}

fn check_id(id: &str) -> Result<()> {
    let ok = !id.is_empty()
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        bail!("invalid id `{id}`: use letters, digits, `-` or `_`");
    }
    Ok(())
}

/// Strips the `\\?\` prefix Windows adds to canonicalized paths.
fn dunce(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(s) => PathBuf::from(s),
        None => path,
    }
}

fn read_yaml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_yaml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn subdirs(dir: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                out.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    out.sort();
    Ok(out)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    if !src.is_dir() {
        bail!("template directory {} not found", src.display());
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

const BLANK_TEMPLATE: &str = r#"#let profile = yaml(sys.inputs.profile)
#let resume = yaml(sys.inputs.resume)

#set page(paper: "a4", margin: 1.5cm)
#set text(size: 10pt)

#align(center, text(20pt, weight: "bold", profile.name))

#resume.summary
"#;

const BLANK_SCHEMA: &str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["template", "summary"],
  "properties": {
    "template": { "type": "string" },
    "summary": { "type": "string", "title": "Summary", "format": "textarea" }
  }
}
"#;
