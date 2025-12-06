{
  description = "A TUI for managing Nix flake inputs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    bun2nix = {
      url = "github:nix-community/bun2nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      bun2nix,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        b2n = bun2nix.packages.${system}.default;

        melt = b2n.writeBunApplication {
          pname = "melt";
          version = "0.1.0";

          src = ./.;

          # Don't try to build/bundle - we run directly with bun
          dontUseBunBuild = true;
          dontUseBunCheck = true;

          startScript = ''
            bun run src/index.tsx "$@"
          '';

          runtimeInputs = [ pkgs.nix ];

          bunDeps = b2n.fetchBunDeps {
            bunNix = ./bun.nix;
          };

          meta = with pkgs.lib; {
            description = "A TUI for managing Nix flake inputs";
            homepage = "https://github.com/your-username/melt";
            license = licenses.mit;
            platforms = platforms.unix;
            mainProgram = "melt";
          };
        };
      in
      {
        packages = {
          default = melt;
          melt = melt;
        };

        apps = {
          default = {
            type = "app";
            program = "${melt}/bin/melt";
          };
          melt = {
            type = "app";
            program = "${melt}/bin/melt";
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.bun
            pkgs.nix
            b2n
          ];
        };
      }
    );
}
