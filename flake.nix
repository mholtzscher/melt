{
  description = "A TUI for managing Nix flake inputs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    { nixpkgs, ... }:
    let
      systems = [
        "aarch64-linux"
        "x86_64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      lib = nixpkgs.lib;
      forEachSystem = lib.genAttrs systems;
      pkgsFor = system: nixpkgs.legacyPackages.${system};

      packageJson = builtins.fromJSON (builtins.readFile ./package.json);

      defaultNodeModules = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
      hashesFile = "${./nix}/hashes.json";
      hashesData =
        if builtins.pathExists hashesFile then builtins.fromJSON (builtins.readFile hashesFile) else { };
      nodeModulesHash = hashesData.nodeModules or defaultNodeModules;
    in
    {
      packages = forEachSystem (
        system:
        let
          pkgs = pkgsFor system;
          mkNodeModules = pkgs.callPackage ./nix/node-modules.nix { hash = nodeModulesHash; };
          mkPackage = pkgs.callPackage ./nix/melt.nix { };
        in
        {
          default = mkPackage {
            version = packageJson.version or "0.1.0";
            src = ./.;
            scripts = ./nix/scripts;
            mkNodeModules = mkNodeModules;
          };
        }
      );

      devShells = forEachSystem (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              bun
              nix
            ];
          };
        }
      );
    };
}
