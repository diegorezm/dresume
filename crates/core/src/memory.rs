//! `memory.md`: durable facts about the user that agents carry between chats
//! ("already did X", "prefers Y"). A plain Markdown file with fixed sections, one fact per
//! `- ` line, dated `(YYYY-MM-DD)` at the end. Kept under [`LIMIT`] bytes by a cheap,
//! deterministic compaction (dedupe + dropping the oldest "Done" items); anything that
//! needs judgment is left to the agent or the user.

use std::collections::HashSet;
use std::fs;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::Workspace;

pub const MEMORY: &str = "memory.md";
/// Size budget in bytes (~1k tokens): the whole file rides along with every chat.
pub const LIMIT: usize = 4096;
/// "Done" items kept even when over budget.
const MIN_DONE: usize = 3;
/// Same fact when word overlap is at least this (and no name/number differs).
const SAME: f64 = 0.8;
/// Possibly the same fact: flagged for the agent, never merged automatically.
const MAYBE: f64 = 0.5;
/// Items with this mark are never dropped.
const PIN: &str = "📌";

/// The fixed sections, in file order: (key used by tools, heading).
pub const SECTIONS: [(&str, &str); 3] =
    [("about", "About the user"), ("preferences", "Preferences"), ("done", "Done")];

const HEADER: &str = "# Memory\n\n\
<!-- Facts agents remember about you between chats. Edit freely: one fact per `- ` line. \
Mark a line with 📌 to keep it forever. -->\n";

#[derive(Debug, Clone, Serialize)]
pub struct MemoryStatus {
    pub size: usize,
    pub limit: usize,
    /// Pairs of facts that may say the same thing (for the agent to merge).
    pub similar: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Remembered {
    /// False when the fact was already there.
    pub added: bool,
    /// Older facts replaced or dropped to stay under the limit.
    pub removed: Vec<String>,
    pub status: MemoryStatus,
}

#[derive(Debug)]
enum Line {
    Item { text: String, date: Option<String> },
    Other(String),
}

#[derive(Debug)]
struct Section {
    /// Heading text without `## `; `None` for lines before the first heading.
    heading: Option<String>,
    lines: Vec<Line>,
}

struct Doc(Vec<Section>);

impl Doc {
    fn parse(text: &str) -> Self {
        let mut sections = vec![Section { heading: None, lines: vec![] }];
        for raw in text.lines() {
            if let Some(h) = raw.strip_prefix("## ") {
                sections.push(Section { heading: Some(h.trim().into()), lines: vec![] });
            } else if let Some(item) = raw.strip_prefix("- ") {
                let (text, date) = split_date(item.trim());
                sections.last_mut().unwrap().lines.push(Line::Item { text, date });
            } else {
                sections.last_mut().unwrap().lines.push(Line::Other(raw.into()));
            }
        }
        Doc(sections)
    }

    fn render(&self) -> String {
        let mut out = String::new();
        for s in &self.0 {
            if let Some(h) = &s.heading {
                out.push_str(&format!("## {h}\n"));
            }
            for line in &s.lines {
                match line {
                    Line::Item { text, date: Some(d) } => out.push_str(&format!("- {text} ({d})\n")),
                    Line::Item { text, date: None } => out.push_str(&format!("- {text}\n")),
                    Line::Other(l) => out.push_str(&format!("{l}\n")),
                }
            }
        }
        out
    }

    /// The section with this heading, created (in the canonical order) when missing.
    fn section(&mut self, heading: &str) -> &mut Section {
        if let Some(i) = self.0.iter().position(|s| s.heading.as_deref() == Some(heading)) {
            return &mut self.0[i];
        }
        if self.0.iter().all(|s| s.heading.is_none() && s.lines.iter().all(|l| matches!(l, Line::Other(t) if t.trim().is_empty()))) {
            // New file: every section, in order, so they never end up shuffled.
            let skeleton: String = SECTIONS.iter().map(|(_, h)| format!("
## {h}
")).collect();
            self.0 = Doc::parse(&format!("{HEADER}{skeleton}")).0;
            return self.section(heading);
        }
        // Blank line before a new heading, unless the file already ends with one.
        let last = self.0.last_mut().unwrap();
        if !matches!(last.lines.last(), Some(Line::Other(t)) if t.trim().is_empty()) {
            last.lines.push(Line::Other(String::new()));
        }
        self.0.push(Section { heading: Some(heading.into()), lines: vec![] });
        self.0.last_mut().unwrap()
    }

    /// Removes exact and near-exact duplicates inside each section (the newer one stays).
    fn dedupe(&mut self, removed: &mut Vec<String>) {
        for s in &mut self.0 {
            let mut i = 0;
            while i < s.lines.len() {
                let Line::Item { text: a, date: da } = &s.lines[i] else { i += 1; continue };
                let dup = (i + 1..s.lines.len()).find(|&j| {
                    matches!(&s.lines[j], Line::Item { text: b, .. } if same_fact(a, b))
                });
                match dup {
                    Some(j) => {
                        let Line::Item { date: db, .. } = &s.lines[j] else { unreachable!() };
                        // Drop the older one; on a tie, the earlier line.
                        let drop = if da > db { j } else { i };
                        if let Line::Item { text, .. } = s.lines.remove(drop) {
                            removed.push(text);
                        }
                    }
                    None => i += 1,
                }
            }
        }
    }

    /// Drops the oldest unpinned "Done" items until the file fits (keeping [`MIN_DONE`]).
    fn trim(&mut self, removed: &mut Vec<String>) {
        let done = SECTIONS[2].1;
        while self.render().len() > LIMIT {
            let Some(s) = self.0.iter_mut().find(|s| s.heading.as_deref() == Some(done)) else { return };
            let items = s.lines.iter().filter(|l| matches!(l, Line::Item { .. })).count();
            if items <= MIN_DONE {
                return;
            }
            // Undated counts as oldest.
            let oldest = s
                .lines
                .iter()
                .enumerate()
                .filter_map(|(i, l)| match l {
                    Line::Item { text, date } if !text.contains(PIN) => Some((date.clone(), i)),
                    _ => None,
                })
                .min();
            let Some((_, i)) = oldest else { return };
            if let Line::Item { text, .. } = s.lines.remove(i) {
                removed.push(text);
            }
        }
    }

    fn similar(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for s in &self.0 {
            let items: Vec<&str> = s
                .lines
                .iter()
                .filter_map(|l| match l {
                    Line::Item { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            for (i, a) in items.iter().enumerate() {
                for b in &items[i + 1..] {
                    if jaccard(a, b) >= MAYBE {
                        out.push((a.to_string(), b.to_string()));
                    }
                }
            }
        }
        out
    }
}

pub fn status(text: &str) -> MemoryStatus {
    MemoryStatus { size: text.len(), limit: LIMIT, similar: Doc::parse(text).similar() }
}

/// Deterministic compaction: dedupe, then drop the oldest "Done" items over the limit.
/// Returns the new text and the removed facts.
pub fn compact(text: &str) -> (String, Vec<String>) {
    let mut doc = Doc::parse(text);
    let mut removed = Vec::new();
    doc.dedupe(&mut removed);
    doc.trim(&mut removed);
    (doc.render(), removed)
}

impl Workspace {
    /// `memory.md`, or an empty string when there is none yet.
    pub fn memory(&self) -> String {
        fs::read_to_string(self.root().join(MEMORY)).unwrap_or_default()
    }

    pub fn save_memory(&self, text: &str) -> Result<MemoryStatus> {
        fs::write(self.root().join(MEMORY), text)?;
        Ok(status(text))
    }

    /// Compacts `memory.md` in place.
    pub fn compact_memory(&self) -> Result<Remembered> {
        let (text, removed) = compact(&self.memory());
        let status = self.save_memory(&text)?;
        Ok(Remembered { added: false, removed, status })
    }

    /// Adds a dated fact to a section (`about`, `preferences` or `done`), replacing an older
    /// version of the same fact, then compacts.
    pub fn remember(&self, section: &str, fact: &str) -> Result<Remembered> {
        let Some((_, heading)) = SECTIONS.iter().find(|(k, _)| *k == section) else {
            bail!("unknown section `{section}` (use about, preferences or done)");
        };
        let fact = fact.trim().trim_start_matches("- ").replace('\n', " ");
        if fact.is_empty() {
            bail!("empty fact");
        }
        let mut doc = Doc::parse(&self.memory());
        let s = doc.section(heading);
        let exists = s.lines.iter().any(|l| matches!(l, Line::Item { text, .. } if key(text) == key(&fact)));
        // Insert after the last item so the blank line before the next heading stays.
        let at = s.lines.iter().rposition(|l| matches!(l, Line::Item { .. })).map_or(0, |i| i + 1);
        s.lines.insert(at, Line::Item { text: fact, date: Some(today()) });
        let mut removed = Vec::new();
        doc.dedupe(&mut removed);
        doc.trim(&mut removed);
        let status = self.save_memory(&doc.render())?;
        Ok(Remembered { added: !exists, removed, status })
    }
}

/// `"text (2026-11-10)"` → (`"text"`, `Some("2026-11-10")`).
fn split_date(item: &str) -> (String, Option<String>) {
    if let Some(rest) = item.strip_suffix(')')
        && let Some(i) = rest.rfind(" (")
    {
        let date = &rest[i + 2..];
        let b = date.as_bytes();
        if b.len() == 10 && b[4] == b'-' && b[7] == b'-' && date.chars().filter(char::is_ascii_digit).count() == 8 {
            return (rest[..i].trim_end().into(), Some(date.into()));
        }
    }
    (item.into(), None)
}

/// Lowercase, no accents, no punctuation, collapsed spaces.
fn key(text: &str) -> String {
    words(text).join(" ")
}

fn words(text: &str) -> Vec<String> {
    let folded: String = text.chars().map(fold).collect();
    folded.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(Into::into).collect()
}

fn fold(c: char) -> char {
    let c = c.to_lowercase().next().unwrap_or(c);
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        _ => c,
    }
}

const STOPWORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "of", "to", "in", "on", "for", "with", "is", "are", "was", "has", "have", "o",
    "os", "as", "um", "uma", "e", "de", "do", "da", "dos", "das", "em", "no", "na", "nos", "nas", "para", "p",
    "por", "com", "que", "se", "ao", "aos", "ja", "foi",
];

fn content_words(text: &str) -> HashSet<String> {
    words(text).into_iter().filter(|w| !STOPWORDS.contains(&w.as_str())).collect()
}

fn jaccard(a: &str, b: &str) -> f64 {
    let (a, b) = (content_words(a), content_words(b));
    let union = a.union(&b).count();
    if union == 0 { 0.0 } else { a.intersection(&b).count() as f64 / union as f64 }
}

/// Same fact: identical once normalized, or very similar with no differing name or number
/// ("interview at X" vs "interview at Y" stay apart).
fn same_fact(a: &str, b: &str) -> bool {
    if key(a) == key(b) {
        return true;
    }
    if jaccard(a, b) < SAME {
        return false;
    }
    let names = |text: &str| -> HashSet<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .enumerate()
            .filter(|(i, w)| !w.is_empty() && (w.chars().any(|c| c.is_ascii_digit()) || (*i > 0 && w.chars().next().unwrap().is_uppercase())))
            .map(|(_, w)| w.chars().map(fold).collect())
            .collect()
    };
    names(a) == names(b)
}

/// Today's date (UTC) as `YYYY-MM-DD`, without a date crate.
fn today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    // Howard Hinnant's civil_from_days.
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_round_trip() {
        assert_eq!(split_date("did x (2026-11-10)"), ("did x".into(), Some("2026-11-10".into())));
        assert_eq!(split_date("did x (soon)"), ("did x (soon)".into(), None));
        assert_eq!(today().len(), 10);
    }

    #[test]
    fn near_duplicates_keep_the_newer() {
        let text = "## Done\n- Revisou currículo general para vaga Nubank (2026-08-02)\n\
                    - Revisou o currículo general p/ vaga Nubank (2026-08-03)\n";
        let (out, removed) = compact(text);
        assert_eq!(removed, vec!["Revisou currículo general para vaga Nubank"]);
        assert!(out.contains("p/ vaga Nubank (2026-08-03)"));
    }

    #[test]
    fn different_names_are_not_merged() {
        assert!(!same_fact("Fez entrevista na empresa Nubank", "Fez entrevista na empresa Itau"));
        assert!(same_fact("Prefere bullets curtos!", "prefere bullets curtos"));
    }

    #[test]
    fn trims_oldest_done_but_never_protected_sections() {
        let mut text = String::from("## Preferences\n- keep me\n## Done\n");
        for i in 0..200 {
            text.push_str(&format!("- task number {i} with some padding text here (2026-01-{:02})\n", i % 28 + 1));
        }
        text.push_str("- 📌 pinned forever (2020-01-01)\n");
        let (out, removed) = compact(&text);
        assert!(out.len() <= LIMIT);
        assert!(out.contains("keep me") && out.contains("pinned forever"));
        assert!(!removed.is_empty());
    }
}
