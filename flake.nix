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
        pkgs = nixpkgs.legacyPackages.${system};

        melt = pkgs.stdenv.mkDerivation {
          pname = "melt";
          version = "0.1.0";

          src = ./.;

          nativeBuildInputs = [
            pkgs.bun
            pkgs.makeWrapper
          ];

          # Skip configure phase
          dontConfigure = true;

          buildPhase = ''
            runHook preBuild

            # Set up bun cache in build directory
            export HOME=$TMPDIR

            # Install dependencies
            bun install --frozen-lockfile

            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall

            # Create output directories
            mkdir -p $out/lib/melt
            mkdir -p $out/bin

            # Copy application files
            cp -r src $out/lib/melt/
            cp -r node_modules $out/lib/melt/
            cp package.json $out/lib/melt/
            cp bun.lock $out/lib/melt/
            cp tsconfig.json $out/lib/melt/

            # Create wrapper script
            makeWrapper ${pkgs.bun}/bin/bun $out/bin/melt \
              --add-flags "run $out/lib/melt/src/index.tsx" \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.nix ]}

            runHook postInstall
          '';

          meta = with pkgs.lib; {
            description = "A TUI for managing Nix flake inputs";
            homepage = "https://github.com/your-username/melt";
            license = licenses.mit;
            maintainers = [ ];
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
          ];
        };
      }
    );
}
