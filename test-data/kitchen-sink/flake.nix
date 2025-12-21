{
  description = "Kitchen sink flake - maximum chaos for stress testing";

  inputs = {
    # === CORE NIXPKGS VARIANTS ===
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    nixpkgs-stable.url = "github:nixos/nixpkgs?ref=nixos-24.11";
    nixpkgs-2405.url = "github:nixos/nixpkgs?ref=nixos-24.05";
    nixpkgs-2311.url = "github:nixos/nixpkgs?ref=nixos-23.11";
    nixpkgs-master.url = "github:nixos/nixpkgs?ref=master";
    nixpkgs-staging.url = "github:nixos/nixpkgs?ref=staging-next";
    nixos-hardware.url = "github:nixos/nixos-hardware";

    # === FLAKE INFRASTRUCTURE ===
    systems.url = "github:nix-systems/default";
    flake-utils.url = "github:numtide/flake-utils";
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    # === HOME & SYSTEM MANAGEMENT ===
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    darwin = {
      url = "github:lnl7/nix-darwin";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-on-droid = {
      url = "github:nix-community/nix-on-droid";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # === DEVELOPMENT ===
    devenv.url = "github:cachix/devenv";
    devshell.url = "github:numtide/devshell";
    dream2nix.url = "github:nix-community/dream2nix";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    pre-commit-hooks.url = "github:cachix/git-hooks.nix";

    # === RUST ECOSYSTEM ===
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url = "github:ipetkov/crane";
    naersk.url = "github:nix-community/naersk";
    fenix.url = "github:nix-community/fenix";

    # === OTHER LANGUAGES ===
    gomod2nix.url = "github:nix-community/gomod2nix";
    bun2nix.url = "github:nix-community/bun2nix";
    poetry2nix.url = "github:nix-community/poetry2nix";
    pyproject-nix.url = "github:nix-community/pyproject.nix";
    haskell-flake.url = "github:srid/haskell-flake";
    zig-overlay.url = "github:mitchellh/zig-overlay";

    # === SECRETS MANAGEMENT ===
    agenix.url = "github:ryantm/agenix";
    sops-nix.url = "github:Mic92/sops-nix";
    ragenix.url = "github:yaxitech/ragenix";

    # === DISK & BOOT ===
    disko.url = "github:nix-community/disko";
    lanzaboote.url = "github:nix-community/lanzaboote";
    impermanence.url = "github:nix-community/impermanence";
    nixos-anywhere.url = "github:nix-community/nixos-anywhere";

    # === DEPLOYMENT ===
    deploy-rs.url = "github:serokell/deploy-rs";
    colmena.url = "github:zhaofengli/colmena";

    # === CACHING ===
    attic.url = "github:zhaofengli/attic";
    cachix-deploy.url = "github:cachix/cachix-deploy-flake";

    # === HYPRLAND ECOSYSTEM ===
    hyprland.url = "github:hyprwm/Hyprland";
    hyprpaper.url = "github:hyprwm/hyprpaper";
    hyprlock.url = "github:hyprwm/hyprlock";
    hypridle.url = "github:hyprwm/hypridle";
    hyprpicker.url = "github:hyprwm/hyprpicker";
    hyprland-plugins.url = "github:hyprwm/hyprland-plugins";
    hyprland-contrib.url = "github:hyprwm/contrib";

    # === THEMING ===
    stylix.url = "github:danth/stylix";
    catppuccin.url = "github:catppuccin/nix";
    base16.url = "github:SenchoPens/base16.nix";

    # === GAMING ===
    nix-gaming.url = "github:fufexan/nix-gaming";
    nix-citizen.url = "github:LovingMelody/nix-citizen";

    # === EDITORS ===
    nixvim.url = "github:nix-community/nixvim";
    helix.url = "github:helix-editor/helix";

    # === BROWSERS ===
    firefox-nightly.url = "github:nix-community/flake-firefox-nightly";

    # === UTILITIES ===
    nix-index-database.url = "github:nix-community/nix-index-database";
    nh.url = "github:viperML/nh";
    nix-alien.url = "github:thiagokokada/nix-alien";
    nurl.url = "github:nix-community/nurl";

    # === NIX LANGUAGE SERVERS ===
    nil.url = "github:oxalica/nil";
    nixd.url = "github:nix-community/nixd";

    # === NON-GITHUB SOURCES ===

    # GitLab
    mesa-git = {
      url = "gitlab:mesa/mesa?host=gitlab.freedesktop.org";
      flake = false;
    };
    gnome-shell = {
      url = "gitlab:GNOME/gnome-shell?host=gitlab.gnome.org";
      flake = false;
    };

    # SourceHut
    hare = {
      url = "sourcehut:~sircmpwn/hare";
      flake = false;
    };
    aerc = {
      url = "sourcehut:~rjarry/aerc";
      flake = false;
    };

    # Raw git
    neovim-master = {
      url = "git+https://github.com/neovim/neovim?ref=master";
      flake = false;
    };

    # Path (local)
    local-secrets = {
      url = "path:./secrets";
      flake = false;
    };
  };

  outputs = inputs: {
    nixosConfigurations = { };
    homeConfigurations = { };
    darwinConfigurations = { };
    devShells = { };
    packages = { };
    overlays = { };
    templates = { };
  };
}
