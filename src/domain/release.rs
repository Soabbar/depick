use crate::domain::dependency::DependencyItem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReleaseCacheKey {
    pub package_name: String,
    pub current_version: String,
    pub target_version: String,
}

impl ReleaseCacheKey {
    pub fn from_item(item: &DependencyItem) -> Self {
        Self {
            package_name: item.name.clone(),
            current_version: item.current.clone(),
            target_version: item.target.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseContext {
    pub package_name: String,
    pub current_version: String,
    pub target_version: String,
    pub source: ReleaseSource,
    pub confidence: LinkConfidence,
    pub repository: Option<RepositoryRef>,
    pub release_url: Option<String>,
    pub compare_url: Option<String>,
    pub changelog_url: Option<String>,
    pub release_title: Option<String>,
    pub release_body_preview: Option<String>,
    pub release_body_full: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReleaseSource {
    GithubRelease,
    GithubTag,
    NpmMetadata,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkConfidence {
    Verified,
    Probable,
    Guessed,
    None,
}

impl LinkConfidence {
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Probable => "probable",
            Self::Guessed => "guessed",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryRef {
    pub host: RepoHost,
    pub owner: String,
    pub repo: String,
    pub directory: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoHost {
    Github,
    Gitlab,
    Bitbucket,
    Unknown,
}

/// State of a release context entry in the store.
/// Since all fetches happen at startup, only `Loaded` and `Failed` are
/// ever stored at runtime. The enum is kept extensible for future use.
#[derive(Debug, Clone)]
pub enum ReleaseContextState {
    Loaded(Box<ReleaseContext>),
    Failed(String),
}

#[derive(Debug, Default)]
pub struct ReleaseContextStore {
    pub entries: HashMap<ReleaseCacheKey, ReleaseContextState>,
}

// ── Text normalization ────────────────────────────────────────────────────────

pub fn normalize_release_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_blank = false;

    for raw_line in input.lines() {
        // Strip markdown heading markers and extra whitespace
        let line = raw_line
            .trim()
            .trim_start_matches('#')
            .trim()
            .replace('\t', " ");

        // Strip bold/italic/code markers
        let line = line.replace("**", "").replace("__", "").replace('`', "");

        // Skip image/badge lines
        if line.starts_with("![") || line.starts_with("[![") {
            continue;
        }

        let is_blank = line.is_empty();
        if is_blank {
            if !last_blank {
                out.push('\n');
            }
            last_blank = true;
            continue;
        }

        last_blank = false;
        out.push_str(&line);
        out.push('\n');
    }

    out.trim().to_string()
}

pub fn build_preview(input: &str, limit: usize) -> String {
    let normalized = normalize_release_text(input);
    let char_count = normalized.chars().count();

    if char_count <= limit {
        return normalized;
    }

    let mut out = String::new();
    let mut in_word = false;
    for ch in normalized.chars().take(limit.saturating_sub(1)) {
        in_word = !ch.is_whitespace();
        out.push(ch);
    }
    // Don't cut mid-word
    if in_word {
        while out.ends_with(|c: char| c.is_alphanumeric() || c == '-' || c == '_') {
            out.pop();
        }
    }
    out.push('…');
    out
}
