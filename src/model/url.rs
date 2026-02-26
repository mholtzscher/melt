use crate::model::ForgeType;

/// A parsed flake URL or reference
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlakeUrl {
    pub owner: String,
    pub repo: String,
    pub forge_type: ForgeType,
}

impl FlakeUrl {
    /// Parse owner and repo from a git URL
    pub fn parse_owner_repo(url: &str) -> Option<(String, String)> {
        fn parse_owner_repo_from_path(path: &str) -> Option<(String, String)> {
            let mut segments: Vec<&str> =
                path.split(['/', '\\']).filter(|s| !s.is_empty()).collect();

            if segments.len() < 2 {
                return None;
            }

            let repo_segment = segments.pop()?;
            let repo = repo_segment.trim_end_matches(".git");
            if repo.is_empty() {
                return None;
            }

            let owner = segments.join("/");
            if owner.is_empty() {
                return None;
            }

            Some((owner, repo.to_string()))
        }

        let url = url.trim();
        if url.is_empty() {
            return None;
        }

        let url = url.strip_prefix("git+").unwrap_or(url);

        // Scheme URLs: https://host/owner/repo, ssh://git@host:port/owner/repo
        if url.starts_with("https://") || url.starts_with("http://") || url.starts_with("ssh://") {
            let rest = url
                .strip_prefix("https://")
                .or_else(|| url.strip_prefix("http://"))
                .or_else(|| url.strip_prefix("ssh://"))?;

            // Drop authority (host / user@host:port)
            let path = rest.split_once('/')?.1;
            let path = path.split(['?', '#']).next().unwrap_or(path);

            return parse_owner_repo_from_path(path);
        }

        // SCP-style: git@host:owner/repo.git
        if url.contains(':') && !url.contains("://") {
            let (_, path) = url.split_once(':')?;
            let path = path.split(['?', '#']).next().unwrap_or(path);

            return parse_owner_repo_from_path(path);
        }

        None
    }

    /// Detect the forge type from URL
    pub fn detect_forge_from_url(url: &str) -> ForgeType {
        if url.contains("github.com") {
            ForgeType::GitHub
        } else if url.contains("gitlab") {
            ForgeType::GitLab
        } else if url.contains("sr.ht") || url.contains("sourcehut") {
            ForgeType::SourceHut
        } else if url.contains("codeberg.org") {
            ForgeType::Codeberg
        } else if url.contains("gitea") || url.contains("forgejo") {
            ForgeType::Gitea
        } else {
            ForgeType::Generic
        }
    }

    /// Build a display URL for the input
    pub fn build_display_url(
        type_: &str,
        owner: &str,
        repo: &str,
        host: Option<&str>,
        locked_url: Option<&str>,
        original_url: Option<&str>,
    ) -> String {
        match type_ {
            "github" => format!("github:{}/{}", owner, repo),
            "gitlab" => {
                if let Some(h) = host {
                    if h != "gitlab.com" {
                        return format!("gitlab:{}/{} ({})", owner, repo, h);
                    }
                }
                format!("gitlab:{}/{}", owner, repo)
            }
            "sourcehut" => {
                let o = if owner.starts_with('~') {
                    owner.to_string()
                } else {
                    format!("~{}", owner)
                };
                format!("sourcehut:{}/{}", o, repo)
            }
            "git" => locked_url
                .or(original_url)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("git:{}/{}", owner, repo)),
            _ => "unknown".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_forge_type() {
        assert_eq!(
            FlakeUrl::detect_forge_from_url("https://github.com/a"),
            ForgeType::GitHub
        );
        assert_eq!(
            FlakeUrl::detect_forge_from_url("https://gitlab.com/a"),
            ForgeType::GitLab
        );
    }

    #[test]
    fn test_parse_owner_repo_from_url_https() {
        assert_eq!(
            FlakeUrl::parse_owner_repo("https://codeberg.org/LGFae/awww"),
            Some(("LGFae".to_string(), "awww".to_string()))
        );
        assert_eq!(
            FlakeUrl::parse_owner_repo("https://github.com/NixOS/nixpkgs.git"),
            Some(("NixOS".to_string(), "nixpkgs".to_string()))
        );
        assert_eq!(
            FlakeUrl::parse_owner_repo("https://gitlab.com/owner/repo"),
            Some(("owner".to_string(), "repo".to_string()))
        );
        assert_eq!(
            FlakeUrl::parse_owner_repo("https://gitlab.com/group/subgroup/repo.git"),
            Some(("group/subgroup".to_string(), "repo".to_string()))
        );
    }

    #[test]
    fn test_parse_owner_repo_from_url_ssh_scp_style() {
        assert_eq!(
            FlakeUrl::parse_owner_repo("git@github.com:owner/repo.git"),
            Some(("owner".to_string(), "repo".to_string()))
        );
        assert_eq!(
            FlakeUrl::parse_owner_repo("git@codeberg.org:LGFae/awww.git"),
            Some(("LGFae".to_string(), "awww".to_string()))
        );
        assert_eq!(
            FlakeUrl::parse_owner_repo("git@gitlab.com:group/subgroup/repo.git"),
            Some(("group/subgroup".to_string(), "repo".to_string()))
        );
    }

    #[test]
    fn test_parse_owner_repo_from_url_ssh_scheme() {
        assert_eq!(
            FlakeUrl::parse_owner_repo("ssh://git@github.com/owner/repo.git"),
            Some(("owner".to_string(), "repo".to_string()))
        );
        assert_eq!(
            FlakeUrl::parse_owner_repo("ssh://git@example.com:2222/owner/repo.git"),
            Some(("owner".to_string(), "repo".to_string()))
        );
        assert_eq!(
            FlakeUrl::parse_owner_repo("ssh://git@gitlab.com/group/subgroup/repo.git"),
            Some(("group/subgroup".to_string(), "repo".to_string()))
        );
    }

    #[test]
    fn test_parse_owner_repo_from_url_edge_cases() {
        assert_eq!(
            FlakeUrl::parse_owner_repo("https://github.com/owner/repo/"),
            Some(("owner".to_string(), "repo".to_string()))
        );
        assert_eq!(FlakeUrl::parse_owner_repo("invalid-url"), None);
        assert_eq!(FlakeUrl::parse_owner_repo(""), None);
        assert_eq!(FlakeUrl::parse_owner_repo("https://github.com/"), None);
        assert_eq!(
            FlakeUrl::parse_owner_repo("https://github.com/owner/"),
            None
        );
    }
}
