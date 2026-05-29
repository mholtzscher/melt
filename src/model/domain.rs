use std::borrow::Borrow;
use std::fmt;

/// Error returned when constructing a validated domain value fails.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum DomainError {
    InvalidInputName,
    InvalidOwner,
    InvalidRepoName,
    InvalidGitRev,
    InvalidHost,
    InvalidGitRef,
    InvalidCloneUrl,
    InvalidLockUrl,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            DomainError::InvalidInputName => "input name must not be empty",
            DomainError::InvalidOwner => "repository owner must not be empty",
            DomainError::InvalidRepoName => "repository name must not be empty",
            DomainError::InvalidGitRev => {
                "git revision must not be empty and must be a valid revision-like string"
            }
            DomainError::InvalidHost => {
                "git host must not be empty and must not contain a URL scheme or path"
            }
            DomainError::InvalidGitRef => "git reference must not be empty",
            DomainError::InvalidCloneUrl => "clone URL must not be empty",
            DomainError::InvalidLockUrl => "lock URL must not be empty",
        };
        f.write_str(message)
    }
}

impl std::error::Error for DomainError {}

macro_rules! validated_string_type {
    (
        $(#[$meta:meta])*
        $name:ident,
        $error:expr,
        $validator:expr
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
                let value = value.into();
                let trimmed = value.trim();
                if trimmed.is_empty() || !($validator)(trimmed) {
                    return Err($error);
                }
                Ok(Self(trimmed.to_string()))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            #[allow(dead_code)]
            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = DomainError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = DomainError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl Borrow<str> for $name {
            fn borrow(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

fn no_control_chars(value: &str) -> bool {
    !value.chars().any(char::is_control)
}

fn git_rev_like(value: &str) -> bool {
    no_control_chars(value) && !value.chars().any(char::is_whitespace)
}

fn host_like(value: &str) -> bool {
    no_control_chars(value)
        && !value.contains("://")
        && !value.contains('/')
        && !value.contains('\\')
        && !value.chars().any(char::is_whitespace)
}

fn url_like(value: &str) -> bool {
    no_control_chars(value) && !value.chars().any(char::is_whitespace)
}

validated_string_type!(
    /// Validated non-empty flake input name.
    InputName,
    DomainError::InvalidInputName,
    no_control_chars
);

validated_string_type!(
    /// Validated non-empty repository owner or namespace.
    Owner,
    DomainError::InvalidOwner,
    no_control_chars
);

validated_string_type!(
    /// Validated non-empty repository name.
    RepoName,
    DomainError::InvalidRepoName,
    no_control_chars
);

validated_string_type!(
    /// Validated non-empty git revision.
    GitRev,
    DomainError::InvalidGitRev,
    git_rev_like
);

validated_string_type!(
    /// Validated git host without URL scheme or path.
    GitHost,
    DomainError::InvalidHost,
    host_like
);

validated_string_type!(
    /// Validated non-empty git branch/tag/ref name.
    GitRef,
    DomainError::InvalidGitRef,
    git_rev_like
);

validated_string_type!(
    /// Validated non-empty clone URL.
    CloneUrl,
    DomainError::InvalidCloneUrl,
    url_like
);

validated_string_type!(
    /// Validated non-empty Nix lock URL.
    LockUrl,
    DomainError::InvalidLockUrl,
    url_like
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_name_rejects_empty_and_control_chars() {
        assert_eq!(InputName::new(""), Err(DomainError::InvalidInputName));
        assert_eq!(InputName::new("   "), Err(DomainError::InvalidInputName));
        assert_eq!(
            InputName::new("bad\nname"),
            Err(DomainError::InvalidInputName)
        );
        assert_eq!(InputName::new("nixpkgs").unwrap().as_str(), "nixpkgs");
    }

    #[test]
    fn owner_and_repo_reject_empty_values() {
        assert_eq!(Owner::new(""), Err(DomainError::InvalidOwner));
        assert_eq!(RepoName::new(""), Err(DomainError::InvalidRepoName));
        assert_eq!(Owner::new("NixOS").unwrap().as_str(), "NixOS");
        assert_eq!(RepoName::new("nixpkgs").unwrap().as_str(), "nixpkgs");
    }

    #[test]
    fn git_rev_rejects_empty_and_whitespace() {
        assert_eq!(GitRev::new(""), Err(DomainError::InvalidGitRev));
        assert_eq!(GitRev::new("abc def"), Err(DomainError::InvalidGitRev));
        assert_eq!(GitRev::new("abcdef123").unwrap().as_str(), "abcdef123");
    }

    #[test]
    fn host_rejects_scheme_path_and_whitespace() {
        assert_eq!(GitHost::new(""), Err(DomainError::InvalidHost));
        assert_eq!(
            GitHost::new("https://example.com"),
            Err(DomainError::InvalidHost)
        );
        assert_eq!(
            GitHost::new("example.com/org"),
            Err(DomainError::InvalidHost)
        );
        assert_eq!(GitHost::new("bad host"), Err(DomainError::InvalidHost));
        assert_eq!(
            GitHost::new("git.example.com").unwrap().as_str(),
            "git.example.com"
        );
    }

    #[test]
    fn urls_reject_empty_and_whitespace() {
        assert_eq!(CloneUrl::new(""), Err(DomainError::InvalidCloneUrl));
        assert_eq!(
            LockUrl::new("github:owner/repo/abc def"),
            Err(DomainError::InvalidLockUrl)
        );
        assert_eq!(
            CloneUrl::new("https://github.com/NixOS/nixpkgs.git")
                .unwrap()
                .as_str(),
            "https://github.com/NixOS/nixpkgs.git"
        );
        assert_eq!(
            LockUrl::new("github:NixOS/nixpkgs/abc123")
                .unwrap()
                .as_str(),
            "github:NixOS/nixpkgs/abc123"
        );
    }

    #[test]
    fn wrappers_support_display_and_borrow() {
        let name = InputName::new("nixpkgs").unwrap();
        assert_eq!(name.to_string(), "nixpkgs");
        let borrowed: &str = name.borrow();
        assert_eq!(borrowed, "nixpkgs");
    }
}
