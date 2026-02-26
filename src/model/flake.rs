use std::path::PathBuf;

/// Data about a loaded flake
#[derive(Debug, Clone)]
pub struct FlakeData {
    pub path: PathBuf,
    pub inputs: Vec<FlakeInput>,
}

/// A flake input - can be git-based, a local path, or something else
#[derive(Debug, Clone)]
pub enum FlakeInput {
    Git(GitInput),
    Path(PathInput),
    Other(OtherInput),
}

/// Git-based flake input (GitHub, GitLab, SourceHut, etc.)
#[derive(Debug, Clone)]
pub struct GitInput {
    pub name: String,
    pub owner: String,
    pub repo: String,
    pub forge_type: ForgeType,
    pub host: Option<String>,
    pub reference: Option<String>, // branch/tag
    pub rev: String,
    pub last_modified: i64,
    pub url: String,
}

/// Local path input
#[derive(Debug, Clone)]
pub struct PathInput {
    pub name: String,
}

/// Other input types (tarball, file, etc.)
#[derive(Debug, Clone)]
pub struct OtherInput {
    pub name: String,
    pub rev: String,
    pub last_modified: i64,
}

/// Type of git forge
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeType {
    GitHub,
    GitLab,
    SourceHut,
    Codeberg,
    Gitea,
    Generic,
}

impl FlakeInput {
    /// Get the name of the input
    pub fn name(&self) -> &str {
        match self {
            FlakeInput::Git(g) => &g.name,
            FlakeInput::Path(p) => &p.name,
            FlakeInput::Other(o) => &o.name,
        }
    }

    /// Get the short revision (first 7 chars) if available
    pub fn short_rev(&self) -> Option<&str> {
        match self {
            FlakeInput::Git(g) => Some(&g.rev[..7.min(g.rev.len())]),
            FlakeInput::Path(_) => None,
            FlakeInput::Other(o) => Some(&o.rev[..7.min(o.rev.len())]),
        }
    }

    /// Get the last modified timestamp if available
    pub fn last_modified(&self) -> Option<i64> {
        match self {
            FlakeInput::Git(g) => Some(g.last_modified),
            FlakeInput::Path(_) => None,
            FlakeInput::Other(o) => Some(o.last_modified),
        }
    }

    /// Get a display string for the type
    pub fn type_display(&self) -> &'static str {
        match self {
            FlakeInput::Git(_) => "git",
            FlakeInput::Path(_) => "path",
            FlakeInput::Other(_) => "other",
        }
    }
}
