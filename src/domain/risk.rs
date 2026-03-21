use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Patch,
    Minor,
    Major,
    /// 0.x minor bump — can be breaking in practice
    ZeroMinor,
    Unknown,
}

impl RiskLevel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Patch => "patch",
            Self::Minor => "minor",
            Self::Major => "major",
            Self::ZeroMinor => "0.x minor",
            Self::Unknown => "unknown",
        }
    }

    /// Whether this risk level should be pre-selected by default.
    pub fn auto_select(&self) -> bool {
        matches!(self, Self::Patch)
    }
}

/// Classify the risk of bumping from `current` to `target`.
pub fn classify_risk(current: &str, target: &str) -> RiskLevel {
    let parse = |v: &str| -> Option<(u64, u64, u64)> {
        // Strip leading non-numeric (operators like ^, ~)
        let v = v.trim_start_matches(|c: char| !c.is_ascii_digit());
        let mut parts = v.splitn(3, '.').map(|p| p.parse::<u64>().ok());
        let major = parts.next()??;
        let minor = parts.next()??;
        let patch = parts.next()??;
        Some((major, minor, patch))
    };

    let Some((cm, cn, _cp)) = parse(current) else {
        return RiskLevel::Unknown;
    };
    let Some((tm, tn, _tp)) = parse(target) else {
        return RiskLevel::Unknown;
    };

    if tm > cm {
        RiskLevel::Major
    } else if tn > cn {
        if cm == 0 {
            RiskLevel::ZeroMinor
        } else {
            RiskLevel::Minor
        }
    } else {
        RiskLevel::Patch
    }
}
