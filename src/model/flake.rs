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

impl ForgeType {
    /// Get the clone URL for a repository
    pub fn clone_url(&self, owner: &str, repo: &str, host: Option<&str>) -> String {
        match self {
            ForgeType::GitHub => format!("https://github.com/{}/{}.git", owner, repo),
            ForgeType::GitLab => {
                let h = host.unwrap_or("gitlab.com");
                format!("https://{}/{}/{}.git", h, owner, repo)
            }
            ForgeType::SourceHut => {
                let h = host.unwrap_or("git.sr.ht");
                let o = if owner.starts_with('~') {
                    owner.to_string()
                } else {
                    format!("~{}", owner)
                };
                format!("https://{}/{}/{}", h, o, repo)
            }
            ForgeType::Codeberg => {
                format!("https://codeberg.org/{}/{}.git", owner, repo)
            }
            ForgeType::Gitea => {
                let h = host.unwrap_or("gitea.com");
                format!("https://{}/{}/{}.git", h, owner, repo)
            }
            ForgeType::Generic => {
                // Can't construct URL without more info
                String::new()
            }
        }
    }

    /// Get the nix lock URL for a specific revision
    pub fn lock_url(&self, owner: &str, repo: &str, rev: &str, host: Option<&str>) -> String {
        match self {
            ForgeType::GitHub => format!("github:{}/{}/{}", owner, repo, rev),
            ForgeType::GitLab => {
                if host.is_none() || host == Some("gitlab.com") {
                    format!("gitlab:{}/{}/{}", owner, repo, rev)
                } else {
                    format!(
                        "git+https://{}/{}/{}?rev={}",
                        host.unwrap(),
                        owner,
                        repo,
                        rev
                    )
                }
            }
            ForgeType::SourceHut => {
                let o = if owner.starts_with('~') {
                    owner.to_string()
                } else {
                    format!("~{}", owner)
                };
                format!("sourcehut:{}/{}/{}", o, repo, rev)
            }
            ForgeType::Codeberg => {
                format!("git+https://codeberg.org/{}/{}?rev={}", owner, repo, rev)
            }
            ForgeType::Gitea => {
                let h = host.unwrap_or("gitea.com");
                format!("git+https://{}/{}/{}?rev={}", h, owner, repo, rev)
            }
            ForgeType::Generic => {
                // Will need the original URL to construct this
                String::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forge_clone_url() {
        assert_eq!(
            ForgeType::GitHub.clone_url("NixOS", "nixpkgs", None),
            "https://github.com/NixOS/nixpkgs.git"
        );

        assert_eq!(
            ForgeType::GitLab.clone_url("owner", "repo", Some("gitlab.gnome.org")),
            "https://gitlab.gnome.org/owner/repo.git"
        );

        assert_eq!(
            ForgeType::SourceHut.clone_url("~user", "repo", None),
            "https://git.sr.ht/~user/repo"
        );
    }

    #[test]
    fn test_forge_lock_url() {
        assert_eq!(
            ForgeType::GitHub.lock_url("NixOS", "nixpkgs", "abc1234", None),
            "github:NixOS/nixpkgs/abc1234"
        );
    }
}
