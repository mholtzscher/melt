{
  bun2nix,
  pkgs,
}:
bun2nix.mkDerivation {
  pname = "melt";
  version = "0.1.0";

  src = ./.;

  bunDeps = bun2nix.fetchBunDeps {
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
}
