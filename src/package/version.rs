use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionReq {
    comparators: Vec<Comparator>,
    any: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionParseError {
    message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionReqError {
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComparatorOp {
    Exact,
    Gt,
    Ge,
    Lt,
    Le,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Comparator {
    op: ComparatorOp,
    version: Version,
}

impl Version {
    #[must_use]
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn parse(text: &str) -> Result<Self, VersionParseError> {
        parse_version(text)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl VersionReq {
    #[must_use]
    pub fn any() -> Self {
        Self {
            comparators: Vec::new(),
            any: true,
        }
    }

    pub fn parse(text: &str) -> Result<Self, VersionReqError> {
        text.parse()
    }

    #[must_use]
    pub fn matches(&self, version: &Version) -> bool {
        if self.any {
            return true;
        }

        self.comparators.iter().all(|cmp| cmp.matches(version))
    }
}

impl FromStr for VersionReq {
    type Err = VersionReqError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(VersionReqError::new(
                "version requirement must not be empty",
            ));
        }
        if trimmed == "*" {
            return Ok(Self::any());
        }

        let mut comparators = Vec::new();
        for token in trimmed
            .split(|ch: char| ch.is_whitespace() || ch == ',')
            .filter(|part| !part.is_empty())
        {
            parse_req_token(token, &mut comparators)?;
        }

        if comparators.is_empty() {
            return Err(VersionReqError::new(
                "version requirement did not contain any comparators",
            ));
        }

        Ok(Self {
            comparators,
            any: false,
        })
    }
}

impl Comparator {
    fn matches(&self, other: &Version) -> bool {
        match self.op {
            ComparatorOp::Exact => other == &self.version,
            ComparatorOp::Gt => other > &self.version,
            ComparatorOp::Ge => other >= &self.version,
            ComparatorOp::Lt => other < &self.version,
            ComparatorOp::Le => other <= &self.version,
        }
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.any {
            return write!(f, "*");
        }
        for (index, cmp) in self.comparators.iter().enumerate() {
            if index > 0 {
                write!(f, " ")?;
            }
            write!(f, "{cmp}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Comparator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self.op {
            ComparatorOp::Exact => "=",
            ComparatorOp::Gt => ">",
            ComparatorOp::Ge => ">=",
            ComparatorOp::Lt => "<",
            ComparatorOp::Le => "<=",
        };
        write!(f, "{op}{}", self.version)
    }
}

fn parse_req_token(token: &str, comparators: &mut Vec<Comparator>) -> Result<(), VersionReqError> {
    if token.is_empty() {
        return Err(VersionReqError::new(
            "version requirement token must not be empty",
        ));
    }

    if let Some(stripped) = token.strip_prefix(">=") {
        let version = parse_version_req_component(stripped)?;
        comparators.push(Comparator {
            op: ComparatorOp::Ge,
            version,
        });
        return Ok(());
    }
    if let Some(stripped) = token.strip_prefix("<=") {
        let version = parse_version_req_component(stripped)?;
        comparators.push(Comparator {
            op: ComparatorOp::Le,
            version,
        });
        return Ok(());
    }
    if let Some(stripped) = token.strip_prefix('>') {
        let version = parse_version_req_component(stripped)?;
        comparators.push(Comparator {
            op: ComparatorOp::Gt,
            version,
        });
        return Ok(());
    }
    if let Some(stripped) = token.strip_prefix('<') {
        let version = parse_version_req_component(stripped)?;
        comparators.push(Comparator {
            op: ComparatorOp::Lt,
            version,
        });
        return Ok(());
    }
    if let Some(stripped) = token.strip_prefix('^') {
        let base = parse_version_req_component(stripped)?;
        add_caret_range(&base, comparators);
        return Ok(());
    }
    if let Some(stripped) = token.strip_prefix('~') {
        let base = parse_version_req_component(stripped)?;
        add_tilde_range(&base, comparators);
        return Ok(());
    }

    if token.contains('*') {
        add_wildcard_range(token, comparators)?;
        return Ok(());
    }

    let version = parse_version_req_component(token)?;
    comparators.push(Comparator {
        op: ComparatorOp::Exact,
        version,
    });
    Ok(())
}

fn add_caret_range(base: &Version, comparators: &mut Vec<Comparator>) {
    comparators.push(Comparator {
        op: ComparatorOp::Ge,
        version: base.clone(),
    });
    let upper = if base.major > 0 {
        Version::new(base.major + 1, 0, 0)
    } else if base.minor > 0 {
        Version::new(0, base.minor + 1, 0)
    } else {
        Version::new(0, 0, base.patch + 1)
    };
    comparators.push(Comparator {
        op: ComparatorOp::Lt,
        version: upper,
    });
}

fn add_tilde_range(base: &Version, comparators: &mut Vec<Comparator>) {
    comparators.push(Comparator {
        op: ComparatorOp::Ge,
        version: base.clone(),
    });
    let upper = Version::new(base.major, base.minor + 1, 0);
    comparators.push(Comparator {
        op: ComparatorOp::Lt,
        version: upper,
    });
}

fn add_wildcard_range(
    token: &str,
    comparators: &mut Vec<Comparator>,
) -> Result<(), VersionReqError> {
    let parts: Vec<_> = token.split('.').collect();
    if parts.is_empty() || parts.iter().any(|part| part.is_empty()) {
        return Err(VersionReqError::new(format!(
            "invalid wildcard version `{token}`"
        )));
    }

    let mut numeric_parts = Vec::new();
    for part in &parts {
        if *part == "*" {
            break;
        }
        numeric_parts.push(*part);
    }
    let lower = parse_version_components(&numeric_parts.join("."))?;
    comparators.push(Comparator {
        op: ComparatorOp::Ge,
        version: lower,
    });

    let wildcard_index = parts
        .iter()
        .position(|part| *part == "*")
        .unwrap_or(parts.len());
    let upper = match wildcard_index {
        0 => Version::new(parse_component("0")?.saturating_add(1), 0, 0),
        1 => Version::new(parse_component(parts[0])?.saturating_add(1), 0, 0),
        2 => Version::new(
            parse_component(parts[0])?,
            parse_component(parts[1])?.saturating_add(1),
            0,
        ),
        _ => Version::new(
            parse_component(parts.get(0).copied().unwrap_or("0"))?,
            parse_component(parts.get(1).copied().unwrap_or("0"))?,
            parse_component(parts.get(2).copied().unwrap_or("0"))?.saturating_add(1),
        ),
    };
    comparators.push(Comparator {
        op: ComparatorOp::Lt,
        version: upper,
    });
    Ok(())
}

fn parse_version_req_component(text: &str) -> Result<Version, VersionReqError> {
    parse_version_components(text)
}

fn parse_version_components(text: &str) -> Result<Version, VersionReqError> {
    let parts: Vec<_> = text
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect();
    if parts.is_empty() {
        return Err(VersionReqError::new(
            "version string must contain at least one numeric component",
        ));
    }
    let major = parse_component(parts.get(0).copied().unwrap_or("0"))?;
    let minor = parse_component(parts.get(1).copied().unwrap_or("0"))?;
    let patch = parse_component(parts.get(2).copied().unwrap_or("0"))?;
    Ok(Version::new(major, minor, patch))
}

fn parse_version(text: &str) -> Result<Version, VersionParseError> {
    let parts: Vec<_> = text
        .trim()
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect();
    if parts.is_empty() {
        return Err(VersionParseError::new(
            "version string must contain at least one numeric component",
        ));
    }
    let major = parse_component(parts.get(0).copied().unwrap_or("0"))
        .map_err(|err| VersionParseError::new(err.message))?;
    let minor = parse_component(parts.get(1).copied().unwrap_or("0"))
        .map_err(|err| VersionParseError::new(err.message))?;
    let patch = parse_component(parts.get(2).copied().unwrap_or("0"))
        .map_err(|err| VersionParseError::new(err.message))?;
    Ok(Version::new(major, minor, patch))
}

fn parse_component(text: &str) -> Result<u64, VersionReqError> {
    if text.is_empty() {
        return Err(VersionReqError::new("version component must not be empty"));
    }
    text.parse::<u64>()
        .map_err(|_| VersionReqError::new(format!("invalid numeric component `{text}` in version")))
}

impl VersionReqError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for VersionReqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for VersionReqError {}

impl VersionParseError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for VersionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for VersionParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_versions() {
        let v = Version::parse("1.2.3").expect("parse version");
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        let short = Version::parse("2.1").expect("parse short version");
        assert_eq!(short, Version::new(2, 1, 0));
    }

    #[test]
    fn version_req_matches_ranges() {
        let req = VersionReq::parse(">=1.2.0 <2.0.0").expect("parse req");
        assert!(req.matches(&Version::new(1, 2, 0)));
        assert!(req.matches(&Version::new(1, 9, 9)));
        assert!(!req.matches(&Version::new(2, 0, 0)));
    }

    #[test]
    fn caret_and_wildcard_requirements_work() {
        let caret = VersionReq::parse("^1.2.3").expect("parse caret");
        assert!(caret.matches(&Version::new(1, 5, 0)));
        assert!(!caret.matches(&Version::new(2, 0, 0)));

        let wildcard = VersionReq::parse("1.2.*").expect("parse wildcard");
        assert!(wildcard.matches(&Version::new(1, 2, 9)));
        assert!(!wildcard.matches(&Version::new(1, 3, 0)));
    }
}
