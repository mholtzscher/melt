{
  description = "Flake with mixed input sources - github, gitlab, sourcehut, git";

  inputs = {
    # GitHub inputs (most common)
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # GitLab inputs
    # Mesa from freedesktop.org GitLab
    mesa-git = {
      url = "gitlab:mesa/mesa?host=gitlab.freedesktop.org";
      flake = false;
    };
    # GNOME projects
    gnome-shell = {
      url = "gitlab:GNOME/gnome-shell?host=gitlab.gnome.org";
      flake = false;
    };

    # SourceHut inputs
    # Drew DeVault's projects
    hare = {
      url = "sourcehut:~sircmpwn/hare";
      flake = false;
    };
    aerc = {
      url = "sourcehut:~rjarry/aerc";
      flake = false;
    };
    wlroots = {
      url = "sourcehut:~sircmpwn/wlroots";
      flake = false;
    };

    # Raw git URLs
    neovim-master = {
      url = "git+https://github.com/neovim/neovim?ref=master";
      flake = false;
    };
    linux-kernel = {
      url = "git+https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git?ref=master&shallow=1";
      flake = false;
    };

    # Path inputs (local directories)
    local-config = {
      url = "path:./config";
      flake = false;
    };
    shared-modules = {
      url = "path:../shared";
      flake = false;
    };
  };

  outputs =
    { self, nixpkgs, ... }@inputs:
    {
      packages = { };
    };
}
