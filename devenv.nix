{ pkgs, ... }:

{
  cachix.enable = false;

  languages.rust.enable = true;

  packages = with pkgs; [
    cargo-edit
    cargo-watch
    git
    libgit2
    nix
    openssl
    pkg-config
  ];

  env = {
    RUST_BACKTRACE = "1";
    LIBGIT2_SYS_USE_PKG_CONFIG = "1";
  };
}
