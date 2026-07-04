{
  description = "A TUI for managing Nix flake inputs";

  inputs = {
    nixpkgs.url = "github:cachix/devenv-nixpkgs/rolling";
    flake-utils.url = "github:numtide/flake-utils";
    devenv = {
      url = "github:cachix/devenv";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      devenv,
    }@inputs:
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

          cargoHash = "sha256-LemcUS45QP6rXDVlkthu6oIQo4k0qjU3+OnatG3oEjg=";

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

        devShells.default = devenv.lib.mkShell {
          inherit inputs pkgs;
          modules = [ ./devenv.nix ];
        };

        formatter = pkgs.nixfmt;
      }
    );
}
