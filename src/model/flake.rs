use std::path::PathBuf;

use super::{CloneUrl, DomainError, GitHost, GitRev, LockUrl, Owner, RepoName};

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
    pub rev: Option<String>,
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

/// Validated repository location. Required forge-specific data is carried by
/// the variant, so states such as a Gitea repository without a host cannot be
/// represented by this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitRepo {
    GitHub { owner: Owner, repo: RepoName },
    GitLab { host: GitHost, owner: Owner, repo: RepoName },
    SourceHut { host: GitHost, owner: Owner, repo: RepoName },
    Codeberg { owner: Owner, repo: RepoName },
    Gitea { host: GitHost, owner: Owner, repo: RepoName },
    Generic { clone_url: CloneUrl },
}

impl GitRepo {
    pub fn github(owner: Owner, repo: RepoName) -> Self {
        Self::GitHub { owner, repo }
    }

    pub fn gitlab(host: Option<GitHost>, owner: Owner, repo: RepoName) -> Result<Self, DomainError> {
        let host = match host {
            Some(host) => host,
            None => GitHost::new("gitlab.com")?,
        };
        Ok(Self::GitLab { host, owner, repo })
    }

    pub fn sourcehut(host: Option<GitHost>, owner: Owner, repo: RepoName) -> Result<Self, DomainError> {
        let host = match host {
            Some(host) => host,
            None => GitHost::new("git.sr.ht")?,
        };
        Ok(Self::SourceHut { host, owner, repo })
    }

    pub fn codeberg(owner: Owner, repo: RepoName) -> Self {
        Self::Codeberg { owner, repo }
    }

    pub fn gitea(host: GitHost, owner: Owner, repo: RepoName) -> Self {
        Self::Gitea { host, owner, repo }
    }

    pub fn generic(clone_url: CloneUrl) -> Self {
        Self::Generic { clone_url }
    }

    pub fn forge_type(&self) -> ForgeType {
        match self {
            Self::GitHub { .. } => ForgeType::GitHub,
            Self::GitLab { .. } => ForgeType::GitLab,
            Self::SourceHut { .. } => ForgeType::SourceHut,
            Self::Codeberg { .. } => ForgeType::Codeberg,
            Self::Gitea { .. } => ForgeType::Gitea,
            Self::Generic { .. } => ForgeType::Generic,
        }
    }

    pub fn owner(&self) -> Option<&str> {
        match self {
            Self::GitHub { owner, .. }
            | Self::GitLab { owner, .. }
            | Self::SourceHut { owner, .. }
            | Self::Codeberg { owner, .. }
            | Self::Gitea { owner, .. } => Some(owner.as_str()),
            Self::Generic { .. } => None,
        }
    }

    pub fn repo_name(&self) -> Option<&str> {
        match self {
            Self::GitHub { repo, .. }
            | Self::GitLab { repo, .. }
            | Self::SourceHut { repo, .. }
            | Self::Codeberg { repo, .. }
            | Self::Gitea { repo, .. } => Some(repo.as_str()),
            Self::Generic { .. } => None,
        }
    }

    pub fn host(&self) -> Option<&str> {
        match self {
            Self::GitLab { host, .. } | Self::SourceHut { host, .. } | Self::Gitea { host, .. } => {
                Some(host.as_str())
            }
            _ => None,
        }
    }

    pub fn clone_url(&self) -> Result<CloneUrl, DomainError> {
        match self {
            Self::GitHub { owner, repo } => CloneUrl::new(format!("https://github.com/{}/{}.git", owner, repo)),
            Self::GitLab { host, owner, repo } => CloneUrl::new(format!("https://{}/{}/{}.git", host, owner, repo)),
            Self::SourceHut { host, owner, repo } => {
                let owner = sourcehut_owner(owner.as_str());
                CloneUrl::new(format!("https://{}/{}/{}", host, owner, repo))
            }
            Self::Codeberg { owner, repo } => CloneUrl::new(format!("https://codeberg.org/{}/{}.git", owner, repo)),
            Self::Gitea { host, owner, repo } => CloneUrl::new(format!("https://{}/{}/{}.git", host, owner, repo)),
            Self::Generic { clone_url } => Ok(clone_url.clone()),
        }
    }

    pub fn lock_url(&self, rev: &GitRev) -> Result<LockUrl, DomainError> {
        match self {
            Self::GitHub { owner, repo } => LockUrl::new(format!("github:{}/{}/{}", owner, repo, rev)),
            Self::GitLab { host, owner, repo } if host.as_str() == "gitlab.com" => {
                LockUrl::new(format!("gitlab:{}/{}/{}", owner, repo, rev))
            }
            Self::GitLab { host, owner, repo } => {
                LockUrl::new(format!("git+https://{}/{}/{}?rev={}", host, owner, repo, rev))
            }
            Self::SourceHut { owner, repo, .. } => {
                let owner = sourcehut_owner(owner.as_str());
                LockUrl::new(format!("sourcehut:{}/{}/{}", owner, repo, rev))
            }
            Self::Codeberg { owner, repo } => {
                LockUrl::new(format!("git+https://codeberg.org/{}/{}?rev={}", owner, repo, rev))
            }
            Self::Gitea { host, owner, repo } => {
                LockUrl::new(format!("git+https://{}/{}/{}?rev={}", host, owner, repo, rev))
            }
            Self::Generic { .. } => Err(DomainError::InvalidLockUrl),
        }
    }
}

fn sourcehut_owner(owner: &str) -> String {
    if owner.starts_with('~') {
        owner.to_string()
    } else {
        format!("~{}", owner)
    }
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
            FlakeInput::Git(g) if !g.rev.is_empty() => Some(&g.rev[..7.min(g.rev.len())]),
            FlakeInput::Other(o) => o.rev.as_deref().map(|rev| &rev[..7.min(rev.len())]),
            _ => None,
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
            FlakeInput::Other(_) => "unsupported",
        }
    }
}

impl ForgeType {
    /// Get the clone URL for a repository
    pub fn clone_url(&self, owner: &str, repo: &str, host: Option<&str>) -> Option<String> {
        match self {
            ForgeType::GitHub => Some(format!("https://github.com/{}/{}.git", owner, repo)),
            ForgeType::GitLab => {
                let h = host.unwrap_or("gitlab.com");
                Some(format!("https://{}/{}/{}.git", h, owner, repo))
            }
            ForgeType::SourceHut => {
                let h = host.unwrap_or("git.sr.ht");
                let o = if owner.starts_with('~') {
                    owner.to_string()
                } else {
                    format!("~{}", owner)
                };
                Some(format!("https://{}/{}/{}", h, o, repo))
            }
            ForgeType::Codeberg => Some(format!("https://codeberg.org/{}/{}.git", owner, repo)),
            ForgeType::Gitea => {
                let h = host?;
                Some(format!("https://{}/{}/{}.git", h, owner, repo))
            }
            ForgeType::Generic => None,
        }
    }

    /// Get the nix lock URL for a specific revision
    pub fn lock_url(
        &self,
        owner: &str,
        repo: &str,
        rev: &str,
        host: Option<&str>,
    ) -> Option<String> {
        match self {
            ForgeType::GitHub => Some(format!("github:{}/{}/{}", owner, repo, rev)),
            ForgeType::GitLab => match host {
                None | Some("gitlab.com") => Some(format!("gitlab:{}/{}/{}", owner, repo, rev)),
                Some(h) => Some(format!("git+https://{}/{}/{}?rev={}", h, owner, repo, rev)),
            },
            ForgeType::SourceHut => {
                let o = if owner.starts_with('~') {
                    owner.to_string()
                } else {
                    format!("~{}", owner)
                };
                Some(format!("sourcehut:{}/{}/{}", o, repo, rev))
            }
            ForgeType::Codeberg => Some(format!(
                "git+https://codeberg.org/{}/{}?rev={}",
                owner, repo, rev
            )),
            ForgeType::Gitea => {
                let h = host?;
                Some(format!("git+https://{}/{}/{}?rev={}", h, owner, repo, rev))
            }
            ForgeType::Generic => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner(value: &str) -> Owner {
        Owner::new(value).unwrap()
    }

    fn repo_name(value: &str) -> RepoName {
        RepoName::new(value).unwrap()
    }

    fn host(value: &str) -> GitHost {
        GitHost::new(value).unwrap()
    }

    fn rev(value: &str) -> GitRev {
        GitRev::new(value).unwrap()
    }

    #[test]
    fn test_forge_clone_url() {
        assert_eq!(
            ForgeType::GitHub.clone_url("NixOS", "nixpkgs", None),
            Some("https://github.com/NixOS/nixpkgs.git".to_string())
        );

        assert_eq!(
            ForgeType::GitLab.clone_url("owner", "repo", Some("gitlab.gnome.org")),
            Some("https://gitlab.gnome.org/owner/repo.git".to_string())
        );

        assert_eq!(
            ForgeType::SourceHut.clone_url("~user", "repo", None),
            Some("https://git.sr.ht/~user/repo".to_string())
        );

        assert_eq!(
            ForgeType::SourceHut.clone_url("user", "repo", None),
            Some("https://git.sr.ht/~user/repo".to_string())
        );

        assert_eq!(
            ForgeType::Codeberg.clone_url("owner", "repo", None),
            Some("https://codeberg.org/owner/repo.git".to_string())
        );

        assert_eq!(
            ForgeType::Gitea.clone_url("owner", "repo", Some("git.example.org")),
            Some("https://git.example.org/owner/repo.git".to_string())
        );

        assert_eq!(ForgeType::Gitea.clone_url("owner", "repo", None), None);
        assert_eq!(ForgeType::Generic.clone_url("owner", "repo", None), None);
    }

    #[test]
    fn test_forge_lock_url() {
        assert_eq!(
            ForgeType::GitHub.lock_url("NixOS", "nixpkgs", "abc1234", None),
            Some("github:NixOS/nixpkgs/abc1234".to_string())
        );

        assert_eq!(
            ForgeType::GitLab.lock_url("owner", "repo", "abc1234", None),
            Some("gitlab:owner/repo/abc1234".to_string())
        );

        assert_eq!(
            ForgeType::GitLab.lock_url("owner", "repo", "abc1234", Some("gitlab.gnome.org")),
            Some("git+https://gitlab.gnome.org/owner/repo?rev=abc1234".to_string())
        );

        assert_eq!(
            ForgeType::SourceHut.lock_url("user", "repo", "abc1234", None),
            Some("sourcehut:~user/repo/abc1234".to_string())
        );

        assert_eq!(
            ForgeType::Codeberg.lock_url("owner", "repo", "abc1234", None),
            Some("git+https://codeberg.org/owner/repo?rev=abc1234".to_string())
        );

        assert_eq!(
            ForgeType::Gitea.lock_url("owner", "repo", "abc1234", Some("git.example.org")),
            Some("git+https://git.example.org/owner/repo?rev=abc1234".to_string())
        );

        assert_eq!(
            ForgeType::Gitea.lock_url("owner", "repo", "abc1234", None),
            None
        );
        assert_eq!(
            ForgeType::Generic.lock_url("owner", "repo", "abc1234", None),
            None
        );
    }

    #[test]
    fn test_git_repo_urls() {
        assert_eq!(
            GitRepo::github(owner("NixOS"), repo_name("nixpkgs"))
                .clone_url()
                .unwrap()
                .as_str(),
            "https://github.com/NixOS/nixpkgs.git"
        );

        assert_eq!(
            GitRepo::gitlab(Some(host("gitlab.gnome.org")), owner("owner"), repo_name("repo"))
                .unwrap()
                .lock_url(&rev("abc1234"))
                .unwrap()
                .as_str(),
            "git+https://gitlab.gnome.org/owner/repo?rev=abc1234"
        );

        assert_eq!(
            GitRepo::sourcehut(None, owner("user"), repo_name("repo"))
                .unwrap()
                .clone_url()
                .unwrap()
                .as_str(),
            "https://git.sr.ht/~user/repo"
        );

        assert_eq!(
            GitRepo::codeberg(owner("owner"), repo_name("repo"))
                .lock_url(&rev("abc1234"))
                .unwrap()
                .as_str(),
            "git+https://codeberg.org/owner/repo?rev=abc1234"
        );

        assert_eq!(
            GitRepo::gitea(host("git.example.org"), owner("owner"), repo_name("repo"))
                .clone_url()
                .unwrap()
                .as_str(),
            "https://git.example.org/owner/repo.git"
        );

        assert!(GitRepo::generic(CloneUrl::new("https://example.org/repo.git").unwrap())
            .lock_url(&rev("abc1234"))
            .is_err());
    }

    #[test]
    fn test_flake_input_short_rev() {
        let git = FlakeInput::Git(GitInput {
            name: "nixpkgs".to_string(),
            owner: "NixOS".to_string(),
            repo: "nixpkgs".to_string(),
            forge_type: ForgeType::GitHub,
            host: None,
            reference: None,
            rev: "abcdef123456".to_string(),
            last_modified: 0,
            url: "github:NixOS/nixpkgs".to_string(),
        });
        assert_eq!(git.short_rev(), Some("abcdef1"));

        let short = FlakeInput::Other(OtherInput {
            name: "archive".to_string(),
            rev: Some("abc".to_string()),
            last_modified: 0,
        });
        assert_eq!(short.short_rev(), Some("abc"));

        let empty = FlakeInput::Other(OtherInput {
            name: "archive".to_string(),
            rev: None,
            last_modified: 0,
        });
        assert_eq!(empty.short_rev(), None);

        let path = FlakeInput::Path(PathInput {
            name: "local".to_string(),
        });
        assert_eq!(path.short_rev(), None);
    }
}
