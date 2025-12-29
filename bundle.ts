#!/usr/bin/env bun

import fs from "node:fs";
import path from "node:path";
import solidPlugin from "./node_modules/@opentui/solid/scripts/solid-plugin";

const dir = process.cwd();
const version = process.env.MELT_VERSION ?? "local";

fs.rmSync(path.join(dir, "dist"), { recursive: true, force: true });

const result = await Bun.build({
  entrypoints: ["./src/index.tsx"],
  outdir: "./dist",
  target: "bun",
  sourcemap: "none",
  plugins: [solidPlugin],
  external: ["@opentui/core"],
  define: {
    MELT_VERSION: `'${version}'`,
  },
});

if (!result.success) {
  console.error("bundle failed");
  for (const log of result.logs) console.error(log);
  process.exit(1);
}
