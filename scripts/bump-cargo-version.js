const fs = require("fs");
const path = require("path");

const nextVersion = process.argv[2];

if (!nextVersion) {
  console.error("Usage: bump-cargo-version <version>");
  process.exit(1);
}

const filePath = path.join(__dirname, "..", "Cargo.toml");
const contents = fs.readFileSync(filePath, "utf8");
const lines = contents.split("\n");

let inPackage = false;
let updated = false;

const updatedLines = lines.map((line) => {
  if (line.trim() === "[package]") {
    inPackage = true;
    return line;
  }

  if (inPackage && line.startsWith("[")) {
    inPackage = false;
  }

  if (inPackage && !updated && line.trim().startsWith("version =")) {
    updated = true;
    return "version = \"" + nextVersion + "\"";
  }

  return line;
});

if (!updated) {
  console.error("Failed to update Cargo.toml version");
  process.exit(1);
}

fs.writeFileSync(filePath, updatedLines.join("\n"));
