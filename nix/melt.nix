{
  lib,
  stdenvNoCC,
  bun,
  nix,
  makeBinaryWrapper,
}:
args:
let
  scripts = args.scripts;
  mkModules =
    attrs:
    args.mkNodeModules (
      attrs
      // {
        canonicalizeScript = scripts + "/canonicalize-node-modules.ts";
        normalizeBinsScript = scripts + "/normalize-bun-binaries.ts";
      }
    );
in
stdenvNoCC.mkDerivation (finalAttrs: {
  pname = "melt";
  version = args.version;

  src = args.src;

  node_modules = mkModules {
    version = finalAttrs.version;
    src = finalAttrs.src;
  };

  nativeBuildInputs = [
    bun
    makeBinaryWrapper
  ];

  env.MELT_VERSION = args.version;

  dontConfigure = true;

  buildPhase = ''
    runHook preBuild

    cp -r ${finalAttrs.node_modules}/node_modules .
    chmod -R u+w ./node_modules

    cp ${./bundle.ts} ./bundle.ts
    chmod +x ./bundle.ts
    bun run ./bundle.ts

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    if [ ! -d dist ]; then
      echo "ERROR: dist directory missing after bundle step"
      exit 1
    fi

    mkdir -p $out/lib/melt
    cp -r dist $out/lib/melt/
    chmod -R u+w $out/lib/melt/dist

    mkdir -p $out/lib/melt/node_modules
    cp -r ./node_modules/.bun $out/lib/melt/node_modules/
    mkdir -p $out/lib/melt/node_modules/@opentui

    mkdir -p $out/bin
    makeWrapper ${bun}/bin/bun $out/bin/melt \
      --add-flags "run" \
      --add-flags "$out/lib/melt/dist/index.js" \
      --prefix PATH : ${lib.makeBinPath [ nix ]} \
      --argv0 melt

    runHook postInstall
  '';

  postInstall = ''
    for pkg in $out/lib/melt/node_modules/.bun/@opentui+core-* $out/lib/melt/node_modules/.bun/@opentui+solid-* $out/lib/melt/node_modules/.bun/@opentui+core@* $out/lib/melt/node_modules/.bun/@opentui+solid@*; do
      if [ -d "$pkg" ]; then
        pkgName=$(basename "$pkg" | sed 's/@opentui+\([^@]*\)@.*/\1/')
        ln -sf ../.bun/$(basename "$pkg")/node_modules/@opentui/$pkgName \
          $out/lib/melt/node_modules/@opentui/$pkgName
      fi
    done
  '';

  dontFixup = true;

  meta = {
    description = "A TUI for managing Nix flake inputs";
    homepage = "https://github.com/michaellatman/melt";
    license = lib.licenses.mit;
    platforms = [
      "aarch64-linux"
      "x86_64-linux"
      "aarch64-darwin"
      "x86_64-darwin"
    ];
    mainProgram = "melt";
  };
})
