{
  description = "Test flake with all input types for melt testing";

  inputs = {
    # GitHub input (most common)
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # GitLab input (gitlab.com)
    gitlab-example.url = "gitlab:gitlab-org/gitlab-runner";

    # Sourcehut input
    sourcehut-example.url = "sourcehut:~sircmpwn/hare";

    # Codeberg/Forgejo input (uses git+https)
    codeberg-example.url = "git+https://codeberg.org/forgejo/forgejo";

    # Generic git input (self-hosted, falls back to git CLI)
    git-example.url = "git+https://git.savannah.gnu.org/git/emacs.git";

    # Path input (local, no VCS support)
    local-example = {
      url = "path:./local-input";
      flake = false;
    };
  };

  outputs =
    { self, nixpkgs, ... }:
    {
      # Minimal output for valid flake
      packages = { };
    };
}
