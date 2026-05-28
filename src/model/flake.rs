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
            FlakeInput::Git(g) if !g.rev.is_empty() => Some(&g.rev[..7.min(g.rev.len())]),
            FlakeInput::Other(o) if !o.rev.is_empty() => Some(&o.rev[..7.min(o.rev.len())]),
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
            FlakeInput::Other(_) => "other",
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
            rev: "abc".to_string(),
            last_modified: 0,
        });
        assert_eq!(short.short_rev(), Some("abc"));

        let empty = FlakeInput::Other(OtherInput {
            name: "archive".to_string(),
            rev: String::new(),
            last_modified: 0,
        });
        assert_eq!(empty.short_rev(), None);

        let path = FlakeInput::Path(PathInput {
            name: "local".to_string(),
        });
        assert_eq!(path.short_rev(), None);
    }
}
