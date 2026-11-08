//! The profile as templates see it: normalized, minus the fields a template hides.

use std::fs;

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::workspace::Workspace;

/// Workspace-relative path of the per-compile profile handed to templates.
/// It only exists in the compile overlay, never on disk.
pub(crate) const RENDERED_PROFILE: &str = ".dresume/profile.yaml";

/// Fields that are always passed to templates, whatever `hide` says.
const ALWAYS_SHOWN: &[&str] = &["name"];

/// Which profile fields each template shows.
#[derive(Debug, Clone, Serialize)]
pub struct ProfileVisibility {
    /// Profile keys, in file order.
    pub fields: Vec<String>,
    pub templates: Vec<TemplateVisibility>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateVisibility {
    pub id: String,
    pub name: String,
    pub hidden: Vec<String>,
}

impl Workspace {
    /// Profile keys hidden by a template (`profile.hide` in its template.yaml).
    pub fn hidden_profile_fields(&self, template: &str) -> Result<Vec<String>> {
        let meta = self.template_meta(template)?;
        let hide = meta.pointer("/profile/hide").and_then(Value::as_array);
        Ok(hide.into_iter().flatten().filter_map(|v| v.as_str().map(Into::into)).collect())
    }

    /// Sets the profile keys a template hides, keeping the rest of template.yaml.
    pub fn set_hidden_profile_fields(&self, template: &str, hidden: &[String]) -> Result<()> {
        let mut meta = self.template_meta(template)?;
        let obj = meta.as_object_mut().context("template.yaml must be a mapping")?;
        let hidden: Vec<Value> = hidden
            .iter()
            .filter(|k| !ALWAYS_SHOWN.contains(&k.as_str()))
            .map(|k| Value::String(k.clone()))
            .collect();
        let profile = obj.entry("profile").or_insert_with(|| Value::Object(Map::new()));
        let profile = profile.as_object_mut().context("`profile` in template.yaml must be a mapping")?;
        if hidden.is_empty() {
            profile.remove("hide");
        } else {
            profile.insert("hide".into(), Value::Array(hidden));
        }
        if profile.is_empty() {
            obj.remove("profile");
        }
        let path = self.template_dir(template).join("template.yaml");
        fs::write(path, serde_yaml::to_string(&meta)?)?;
        Ok(())
    }

    pub fn profile_visibility(&self) -> Result<ProfileVisibility> {
        let profile = self.profile()?;
        let fields = profile.as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default();
        let templates = self
            .templates()?
            .into_iter()
            .map(|t| {
                let hidden = self.hidden_profile_fields(&t.id)?;
                Ok(TemplateVisibility { id: t.id, name: t.name, hidden })
            })
            .collect::<Result<_>>()?;
        Ok(ProfileVisibility { fields, templates })
    }

    /// The profile YAML a template receives. `source` overrides profile.yaml (unsaved edits).
    pub(crate) fn rendered_profile(&self, template: &str, source: Option<&str>) -> Result<String> {
        let mut profile: Value = match source {
            Some(text) => serde_yaml::from_str(text).context("parsing profile.yaml")?,
            None => self.profile()?,
        };
        let hidden = self.hidden_profile_fields(template)?;
        if let Some(obj) = profile.as_object_mut() {
            obj.retain(|k, _| ALWAYS_SHOWN.contains(&k.as_str()) || !hidden.contains(k));
            for value in obj.values_mut() {
                normalize_link(value);
            }
        }
        Ok(serde_yaml::to_string(&profile)?)
    }

    fn template_meta(&self, template: &str) -> Result<Value> {
        let path = self.template_dir(template).join("template.yaml");
        let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        Ok(serde_yaml::from_str(&text)?)
    }
}

/// Links may be written as a plain URL or as `{label, url}`; templates always get the latter.
fn normalize_link(value: &mut Value) {
    match value {
        Value::String(s) if is_url(s) => {
            let url = s.trim().to_string();
            *value = serde_json::json!({ "label": pretty_url(&url), "url": url });
        }
        Value::String(s) if is_bare_domain(s) => {
            let label = s.trim().trim_end_matches('/').to_string();
            *value = serde_json::json!({ "url": format!("https://{label}"), "label": label });
        }
        Value::Object(obj) => {
            if let Some(url) = obj.get("url").and_then(Value::as_str).map(str::to_string) {
                obj.entry("label").or_insert_with(|| Value::String(pretty_url(&url)));
            }
        }
        _ => {}
    }
}

fn is_url(s: &str) -> bool {
    let s = s.trim();
    (s.starts_with("https://") || s.starts_with("http://")) && !s.contains(char::is_whitespace)
}

/// A URL written without a scheme, like `you.dev` or `github.com/you`.
fn is_bare_domain(s: &str) -> bool {
    let s = s.trim();
    let host = s.split('/').next().unwrap_or("");
    let Some((name, tld)) = host.rsplit_once('.') else { return false };
    !s.contains(char::is_whitespace)
        && !s.contains('@')
        && !name.is_empty()
        && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        && tld.len() >= 2
        && tld.chars().all(|c| c.is_ascii_alphabetic())
}

/// "https://www.github.com/you/" -> "github.com/you"
fn pretty_url(url: &str) -> String {
    let s = url.trim();
    let s = s.strip_prefix("https://").or_else(|| s.strip_prefix("http://")).unwrap_or(s);
    let s = s.strip_prefix("www.").unwrap_or(s);
    s.trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_urls_become_links() {
        let mut v = Value::String("https://www.you.dev/".into());
        normalize_link(&mut v);
        assert_eq!(v["label"], "you.dev");
        assert_eq!(v["url"], "https://www.you.dev/");

        let mut v = serde_json::json!({ "url": "https://github.com/you" });
        normalize_link(&mut v);
        assert_eq!(v["label"], "github.com/you");

        let mut v = Value::String("diegorezm.xyz".into());
        normalize_link(&mut v);
        assert_eq!(v["url"], "https://diegorezm.xyz");
        assert_eq!(v["label"], "diegorezm.xyz");

        let mut v = Value::String("github.com/you/".into());
        normalize_link(&mut v);
        assert_eq!(v["url"], "https://github.com/you");

        for text in ["São Paulo, BR", "+55 11 90000-0000", "@you", "v1.2", "Jr. Dev"] {
            let mut v = Value::String(text.into());
            normalize_link(&mut v);
            assert_eq!(v, text, "{text} should stay text");
        }

        let mut v = Value::String("you@example.com".into());
        normalize_link(&mut v);
        assert_eq!(v, "you@example.com");
    }
}
