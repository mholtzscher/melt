{
  description = "A TUI for managing Nix flake inputs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

        nativeBuildInputs = with pkgs; [
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
          version = cargoToml.package.version;

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

        apps.default = flake-utils.lib.mkApp {
          drv = self.packages.${system}.default;
        };

        checks.default = self.packages.${system}.default;

        formatter = pkgs.nixfmt;
      }
    );
}
