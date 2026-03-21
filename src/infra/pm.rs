use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::domain::dependency::{DependencyItem, DependencyKind};
use crate::domain::risk::classify_risk;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Bun,
    Npm,
}

impl PackageManager {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Bun => "bun",
            Self::Npm => "npm",
        }
    }
}

/// Detect the package manager for a given directory.
pub fn detect(dir: &Path) -> PackageManager {
    if dir.join("bun.lockb").exists() || dir.join("bun.lock").exists() {
        return PackageManager::Bun;
    }
    PackageManager::Npm
}

// ── npm outdated --json ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct NpmOutdatedEntry {
    current: Option<String>,
    wanted: Option<String>,
    latest: Option<String>,
    #[serde(rename = "type")]
    dep_type: Option<String>,
}

/// Run `npm outdated --json` and parse the result into DependencyItems.
pub fn npm_outdated(
    dir: &Path,
    target_mode: &crate::app::TargetMode,
) -> Result<Vec<DependencyItem>> {
    let output = std::process::Command::new("npm")
        .args(["outdated", "--json"])
        .current_dir(dir)
        .output()
        .context("failed to run npm outdated")?;

    // npm outdated exits with code 1 when packages are outdated — that's fine
    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.trim().is_empty() || stdout.trim() == "{}" {
        return Ok(vec![]);
    }

    let raw: HashMap<String, NpmOutdatedEntry> = serde_json::from_str(&stdout)
        .with_context(|| format!("failed to parse npm outdated JSON:\n{stdout}"))?;

    // Read package.json to get declared ranges
    let declared = read_declared_ranges(dir).unwrap_or_default();

    let mut items: Vec<DependencyItem> = raw
        .into_iter()
        .filter_map(|(name, entry)| {
            let current = entry.current?;
            let latest = entry.latest?;
            let wanted = entry.wanted;

            let target = match target_mode {
                crate::app::TargetMode::Latest => latest.clone(),
                crate::app::TargetMode::Wanted => wanted.clone().unwrap_or_else(|| latest.clone()),
            };

            let risk = classify_risk(&current, &target);

            let kind = match entry.dep_type.as_deref() {
                Some("devDependencies") => DependencyKind::DevDependency,
                _ => DependencyKind::Dependency,
            };

            Some(DependencyItem {
                declared: declared.get(&name).cloned(),
                checked: risk.auto_select(),
                name,
                current,
                wanted,
                latest,
                target,
                kind,
                risk,
            })
        })
        .collect();

    // Sort: by risk severity desc, then name asc
    items.sort_by(|a, b| {
        risk_sort_key(a.risk)
            .cmp(&risk_sort_key(b.risk))
            .then(a.name.cmp(&b.name))
    });

    Ok(items)
}

fn risk_sort_key(r: crate::domain::risk::RiskLevel) -> u8 {
    use crate::domain::risk::RiskLevel::*;
    match r {
        Major => 0,
        ZeroMinor => 1,
        Minor => 2,
        Patch => 3,
        Unknown => 4,
    }
}

// ── package.json declared ranges ─────────────────────────────────────────────

#[derive(Deserialize)]
struct PackageJson {
    dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<HashMap<String, String>>,
}

fn read_declared_ranges(dir: &Path) -> Result<HashMap<String, String>> {
    let content =
        std::fs::read_to_string(dir.join("package.json")).context("could not read package.json")?;
    let pkg: PackageJson =
        serde_json::from_str(&content).context("could not parse package.json")?;

    let mut map = HashMap::new();
    if let Some(deps) = pkg.dependencies {
        map.extend(deps);
    }
    if let Some(devdeps) = pkg.dev_dependencies {
        map.extend(devdeps);
    }
    Ok(map)
}

// ── bun outdated (text parsing fallback) ─────────────────────────────────────

/// Run `bun outdated` and parse its table output.
/// This is less reliable than npm JSON, but necessary for Bun-native repos.
pub fn bun_outdated(
    dir: &Path,
    target_mode: &crate::app::TargetMode,
) -> Result<Vec<DependencyItem>> {
    let output = std::process::Command::new("bun")
        .arg("outdated")
        .current_dir(dir)
        .output()
        .context("failed to run bun outdated")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let declared = read_declared_ranges(dir).unwrap_or_default();

    let mut items = Vec::new();

    for line in stdout.lines() {
        // Table rows look like:
        // │ wrangler             │ 4.67.0  │ 4.67.0  │ 4.76.0  │
        if !line.contains('│') {
            continue;
        }

        let cols: Vec<&str> = line
            .split('│')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        // Expect [Package, Current, Update, Latest]
        if cols.len() < 4 {
            continue;
        }

        let name = cols[0].to_string();
        let current = cols[1].to_string();
        let wanted = cols[2].to_string();
        let latest = cols[3].to_string();

        // Skip the header row
        if name == "Package" || current == "Current" {
            continue;
        }

        // Skip if versions look like header/empty
        if current.is_empty() || latest.is_empty() {
            continue;
        }

        let target = match target_mode {
            crate::app::TargetMode::Latest => latest.clone(),
            crate::app::TargetMode::Wanted => wanted.clone(),
        };

        let risk = classify_risk(&current, &target);

        // Bun doesn't tell us dep type in this output, so guess from package.json
        let kind = if declared
            .get(&name)
            .map(|_| false) // placeholder: we'd need to check devDeps separately
            .unwrap_or(false)
        {
            DependencyKind::DevDependency
        } else {
            DependencyKind::Dependency
        };

        items.push(DependencyItem {
            declared: declared.get(&name).cloned(),
            checked: risk.auto_select(),
            name,
            current,
            wanted: Some(wanted),
            latest,
            target,
            kind,
            risk,
        });
    }

    items.sort_by(|a, b| {
        risk_sort_key(a.risk)
            .cmp(&risk_sort_key(b.risk))
            .then(a.name.cmp(&b.name))
    });

    Ok(items)
}
