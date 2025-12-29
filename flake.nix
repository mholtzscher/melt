{
  description = "A TUI for managing Nix flake inputs";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    systems.url = "github:nix-systems/default";
    bun2nix = {
      url = "github:nix-community/bun2nix?tag=2.0.6";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.systems.follows = "systems";
    };
  };

  nixConfig = {
    extra-substituters = [
      "https://cache.nixos.org"
      "https://nix-community.cachix.org"
    ];
    extra-trusted-public-keys = [
      "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };

  outputs =
    inputs:
    let
      eachSystem = inputs.nixpkgs.lib.genAttrs (import inputs.systems);

      pkgsFor = eachSystem (
        system:
        import inputs.nixpkgs {
          inherit system;
          overlays = [ inputs.bun2nix.overlays.default ];
        }
      );
    in
    {
      packages = eachSystem (
        system:
        let
          pkgs = pkgsFor.${system};
        in
        {
          default = pkgs.bun2nix.mkDerivation {
            pname = "melt";
            version = "0.1.0";

            src = ./.;

            bunDeps = pkgs.bun2nix.fetchBunDeps {
              bunNix = ./bun.nix;
            };

            nativeBuildInputs = with pkgs; [
              makeWrapper
              bun
            ];

            env.MELT_VERSION = "0.1.0";

            buildPhase = ''
              runHook preBuild
              bun run ./bundle.ts
              runHook postBuild
            '';

            installPhase = ''
              runHook preInstall
              mkdir -p $out/lib/melt $out/bin

              cp -r dist node_modules $out/lib/melt

              makeWrapper ${pkgs.bun}/bin/bun $out/bin/melt \
                --add-flags "run" \
                --add-flags "$out/lib/melt/dist/index.js" \
                --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.nix ]} \
                --argv0 melt

              runHook postInstall
            '';

            meta = {
              description = "A TUI for managing Nix flake inputs";
              mainProgram = "melt";
            };
          };
        }
      );

      devShells = eachSystem (system: {
        default = pkgsFor.${system}.mkShell {
          packages = with pkgsFor.${system}; [
            bun
            bun2nix
            oxlint
            oxfmt
          ];

          shellHook = ''
            bun install --frozen-lockfile
            export PATH=${pkgsFor.${system}.oxfmt}/bin:${pkgsFor.${system}.oxlint}/bin:$PATH
          '';
        };
      });
    };
}
