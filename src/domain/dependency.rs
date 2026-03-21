use super::risk::RiskLevel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyItem {
    pub name: String,
    pub current: String,
    pub wanted: Option<String>,
    pub latest: String,
    /// The version we'll actually update to (wanted or latest, per user mode)
    pub target: String,
    /// The declared range in package.json, e.g. "^4.67.0"
    pub declared: Option<String>,
    pub kind: DependencyKind,
    pub risk: RiskLevel,
    pub checked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyKind {
    Dependency,
    DevDependency,
}

impl DependencyKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Dependency => "dep",
            Self::DevDependency => "dev",
        }
    }
}

impl DependencyItem {
    /// Build the install command for this item.
    pub fn install_command(
        &self,
        pm: &crate::infra::pm::PackageManager,
        write: &crate::app::WriteMode,
    ) -> String {
        let version_str = match write {
            crate::app::WriteMode::PreserveRange => {
                // Re-use the operator from declared, default to ^
                let operator = self
                    .declared
                    .as_deref()
                    .and_then(|d| {
                        let first = d.chars().next()?;
                        if matches!(first, '^' | '~' | '>' | '<' | '=') {
                            Some(first.to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "^".to_string());
                format!("{}{}", operator, self.target)
            }
            crate::app::WriteMode::Exact => self.target.clone(),
        };

        match pm {
            crate::infra::pm::PackageManager::Bun => {
                format!("bun add {}@{}", self.name, version_str)
            }
            crate::infra::pm::PackageManager::Npm => {
                format!("npm install {}@{}", self.name, version_str)
            }
        }
    }

    /// The planned manifest change as a display string.
    pub fn planned_change(&self) -> String {
        let declared = self.declared.as_deref().unwrap_or(&self.current);
        let operator = declared
            .chars()
            .next()
            .filter(|c| matches!(c, '^' | '~'))
            .map(|c| c.to_string())
            .unwrap_or_default();
        format!("{} → {}{}", declared, operator, self.target)
    }
}
