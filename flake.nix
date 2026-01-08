{
  description = "A TUI for managing Nix flake inputs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
          ];
        };

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          makeWrapper
        ];

        buildInputs =
          with pkgs;
          [
            openssl
            libgit2
          ]
          ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.libiconv
          ];

      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "melt";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit nativeBuildInputs buildInputs;

          # Use system OpenSSL instead of building from source
          OPENSSL_NO_VENDOR = 1;

          # Skip tests in Nix build - run with `cargo test` in devShell instead
          # (test fixtures use paths that don't resolve correctly in the sandbox)
          doCheck = false;

          # Runtime dependencies
          postInstall = ''
            wrapProgram $out/bin/melt \
              --prefix PATH : ${
                pkgs.lib.makeBinPath [
                  pkgs.nix
                  pkgs.git
                ]
              }
          '';

          nativeCheckInputs = [
            pkgs.nix
            pkgs.git
          ];

          meta = with pkgs.lib; {
            description = "A TUI for managing Nix flake inputs";
            homepage = "https://github.com/mholtzscher/melt";
            license = licenses.mit;
            maintainers = [ ];
            mainProgram = "melt";
          };
        };

        devShells.default = pkgs.mkShell {
          inherit buildInputs;

          nativeBuildInputs =
            nativeBuildInputs
            ++ (with pkgs; [
              cargo-watch
              cargo-edit
            ]);

          RUST_BACKTRACE = 1;

          # For git2 to find libgit2
          LIBGIT2_SYS_USE_PKG_CONFIG = 1;
        };
      }
    );
}
