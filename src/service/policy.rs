use crate::model::ForgeType;

pub fn build_clone_url(
    forge_type: ForgeType,
    owner: &str,
    repo: &str,
    host: Option<&str>,
) -> Option<String> {
    let url = match forge_type {
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
        ForgeType::Codeberg => format!("https://codeberg.org/{}/{}.git", owner, repo),
        ForgeType::Gitea => {
            let h = host.unwrap_or("gitea.com");
            format!("https://{}/{}/{}.git", h, owner, repo)
        }
        ForgeType::Generic => return None,
    };

    Some(url)
}

pub fn build_lock_url(
    forge_type: ForgeType,
    owner: &str,
    repo: &str,
    rev: &str,
    host: Option<&str>,
) -> Option<String> {
    let url = match forge_type {
        ForgeType::GitHub => format!("github:{}/{}/{}", owner, repo, rev),
        ForgeType::GitLab => match host {
            None | Some("gitlab.com") => format!("gitlab:{}/{}/{}", owner, repo, rev),
            Some(h) => format!("git+https://{}/{}/{}?rev={}", h, owner, repo, rev),
        },
        ForgeType::SourceHut => {
            let o = if owner.starts_with('~') {
                owner.to_string()
            } else {
                format!("~{}", owner)
            };
            format!("sourcehut:{}/{}/{}", o, repo, rev)
        }
        ForgeType::Codeberg => format!("git+https://codeberg.org/{}/{}?rev={}", owner, repo, rev),
        ForgeType::Gitea => {
            let h = host.unwrap_or("gitea.com");
            format!("git+https://{}/{}/{}?rev={}", h, owner, repo, rev)
        }
        ForgeType::Generic => return None,
    };

    Some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_clone_url() {
        assert_eq!(
            build_clone_url(ForgeType::GitHub, "NixOS", "nixpkgs", None),
            Some("https://github.com/NixOS/nixpkgs.git".to_string())
        );

        assert_eq!(
            build_clone_url(ForgeType::GitLab, "owner", "repo", Some("gitlab.gnome.org")),
            Some("https://gitlab.gnome.org/owner/repo.git".to_string())
        );

        assert_eq!(
            build_clone_url(ForgeType::SourceHut, "~user", "repo", None),
            Some("https://git.sr.ht/~user/repo".to_string())
        );

        assert_eq!(
            build_clone_url(ForgeType::Generic, "owner", "repo", None),
            None
        );
    }

    #[test]
    fn test_build_lock_url() {
        assert_eq!(
            build_lock_url(ForgeType::GitHub, "NixOS", "nixpkgs", "abc1234", None),
            Some("github:NixOS/nixpkgs/abc1234".to_string())
        );

        assert_eq!(
            build_lock_url(ForgeType::Generic, "owner", "repo", "abc1234", None),
            None
        );
    }
}
