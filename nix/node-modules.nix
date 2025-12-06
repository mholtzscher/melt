{
  hash,
  lib,
  stdenvNoCC,
  bun,
  cacert,
  curl,
}:
args:
stdenvNoCC.mkDerivation {
  pname = "melt-node_modules";
  version = args.version;
  src = args.src;

  impureEnvVars = lib.fetchers.proxyImpureEnvVars ++ [
    "GIT_PROXY_COMMAND"
    "SOCKS_SERVER"
  ];

  nativeBuildInputs = [
    bun
    cacert
    curl
  ];

  dontConfigure = true;

  buildPhase = ''
    runHook preBuild
    export HOME=$(mktemp -d)
    export BUN_INSTALL_CACHE_DIR=$(mktemp -d)
    bun install \
      --frozen-lockfile \
      --ignore-scripts \
      --no-progress \
      --linker=isolated
    bun --bun ${args.canonicalizeScript}
    bun --bun ${args.normalizeBinsScript}
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out
    if [ -d node_modules ]; then
      cp -R node_modules $out/
    fi
    runHook postInstall
  '';

  dontFixup = true;

  outputHashAlgo = "sha256";
  outputHashMode = "recursive";
  outputHash = hash;
}
