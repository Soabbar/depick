use anyhow::Result;
use futures::stream::{self, StreamExt};
use serde::Deserialize;

use crate::domain::dependency::DependencyItem;
use crate::domain::release::*;
use crate::infra::metadata::PackageMetadata;

// Packages beyond this threshold switch from full concurrency to batched.
const CONCURRENT_THRESHOLD: usize = 20;
// Batch concurrency cap used when there are more than CONCURRENT_THRESHOLD items.
const BATCH_SIZE: usize = 8;

/// Normalize a repository URL from npm metadata into a RepositoryRef.
pub fn normalize_repo(url: &str) -> Option<RepositoryRef> {
    let url = url
        .trim()
        .trim_start_matches("git+")
        .trim_end_matches(".git");

    let normalized = url
        .replace("git://github.com/", "https://github.com/")
        .replace("ssh://git@github.com/", "https://github.com/")
        .replace("git@github.com:", "https://github.com/");

    let normalized = if let Some(rest) = normalized.strip_prefix("github:") {
        format!("https://github.com/{rest}")
    } else {
        normalized
    };

    let prefix = "https://github.com/";
    let path = normalized.strip_prefix(prefix)?;

    // Strip query strings or fragments
    let path = path.split('?').next().unwrap_or(path);
    let path = path.split('#').next().unwrap_or(path);

    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 2 {
        return None;
    }

    let directory = if parts.len() > 4 && parts[2] == "tree" {
        // e.g. /owner/repo/tree/main/packages/wrangler
        Some(parts[4..].join("/"))
    } else {
        None
    };

    Some(RepositoryRef {
        host: RepoHost::Github,
        owner: parts[0].to_string(),
        repo: parts[1].to_string(),
        directory,
    })
}

fn candidate_tags(pkg: &str, version: &str) -> Vec<String> {
    vec![
        format!("v{version}"),
        version.to_string(),
        format!("{pkg}@{version}"),
    ]
}

fn github_compare_url(repo: &RepositoryRef, _pkg: &str, from: &str, to: &str) -> String {
    // Use the most common tag format as a best guess
    let f = format!("v{from}");
    let t = format!("v{to}");
    format!(
        "https://github.com/{}/{}/compare/{}...{}",
        repo.owner, repo.repo, f, t
    )
}

// ── GitHub releases API ───────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
}

/// Try to fetch the GitHub release matching the target version.
async fn find_github_release(
    client: &reqwest::Client,
    repo: &RepositoryRef,
    pkg: &str,
    target: &str,
) -> Option<GithubRelease> {
    let candidates = candidate_tags(pkg, target);
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases",
        repo.owner, repo.repo
    );

    let releases: Vec<GithubRelease> = client
        .get(&url)
        .header("User-Agent", "depick/0.1")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;

    releases
        .into_iter()
        .find(|r| candidates.iter().any(|c| &r.tag_name == c))
}

// ── Bulk preloader ────────────────────────────────────────────────────────────

/// Fetch release contexts for every item in `items` **before** the TUI opens,
/// writing results directly into `store`.
///
/// Strategy:
/// - ≤ `CONCURRENT_THRESHOLD` items → all requests fly concurrently.
/// - >  `CONCURRENT_THRESHOLD` items → `buffer_unordered(BATCH_SIZE)` caps in-flight connections so GitHub's API is not hammered and local file-descriptors are not exhausted on very large dependency lists.
///
/// Either way the caller gets a fully-populated `ReleaseContextStore` and a
/// plain `Vec<String>` of non-fatal error messages to surface in the UI.
pub async fn fetch_all_release_contexts(
    client: &reqwest::Client,
    items: &[DependencyItem],
    progress: impl Fn(usize, usize) + Send + Sync,
) -> ReleaseContextStore {
    let total = items.len();
    let concurrency = if total <= CONCURRENT_THRESHOLD {
        total.max(1) // all at once
    } else {
        BATCH_SIZE // capped batch
    };

    // Atomic counter shared across futures so the progress callback stays accurate.
    let done = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let results: Vec<(ReleaseCacheKey, ReleaseContextState)> = stream::iter(items.iter())
        .map(|item| {
            let client = client.clone();
            let done = done.clone();
            let progress = &progress;
            async move {
                let key = ReleaseCacheKey::from_item(item);
                let state = match crate::infra::metadata::fetch_metadata(&client, &item.name).await
                {
                    Ok(meta) => match resolve_release_context(&client, item, &meta).await {
                        Ok(ctx) => ReleaseContextState::Loaded(Box::new(ctx)),
                        Err(e) => ReleaseContextState::Failed(e.to_string()),
                    },
                    Err(e) => ReleaseContextState::Failed(e.to_string()),
                };
                let n = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                progress(n, total);
                (key, state)
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    let mut store = ReleaseContextStore::default();
    for (key, state) in results {
        store.entries.insert(key, state);
    }
    store
}

// ── Main resolver ─────────────────────────────────────────────────────────────

pub async fn resolve_release_context(
    client: &reqwest::Client,
    item: &DependencyItem,
    metadata: &PackageMetadata,
) -> Result<ReleaseContext> {
    let repo = metadata.repository_url.as_deref().and_then(normalize_repo);

    let (release_url, compare_url, release_title, body_raw, source, confidence) =
        if let Some(repo) = &repo {
            let releases_page = format!("https://github.com/{}/{}/releases", repo.owner, repo.repo);
            let compare = github_compare_url(repo, &item.name, &item.current, &item.target);

            // Try to find an exact GitHub release
            if let Some(release) = find_github_release(client, repo, &item.name, &item.target).await
            {
                (
                    Some(release.html_url),
                    Some(compare),
                    release
                        .name
                        .or(Some(format!("{} {}", item.name, item.target))),
                    release.body,
                    ReleaseSource::GithubRelease,
                    LinkConfidence::Verified,
                )
            } else {
                // No exact release found, fall back to releases page + guessed compare
                (
                    Some(releases_page),
                    Some(compare),
                    None,
                    None,
                    ReleaseSource::GithubTag,
                    LinkConfidence::Probable,
                )
            }
        } else {
            (
                metadata.npm_url.clone(),
                None,
                None,
                None,
                ReleaseSource::NpmMetadata,
                LinkConfidence::None,
            )
        };

    let release_body_preview = body_raw.as_deref().map(|b| build_preview(b, 400));
    let release_body_full = body_raw.as_deref().map(|b| build_preview(b, 3000));

    Ok(ReleaseContext {
        package_name: item.name.clone(),
        current_version: item.current.clone(),
        target_version: item.target.clone(),
        source,
        confidence,
        repository: repo,
        release_url,
        compare_url,
        changelog_url: metadata.homepage_url.clone(),
        release_title,
        release_body_preview,
        release_body_full,
    })
}
