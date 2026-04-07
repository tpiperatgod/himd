import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const pluginDir = path.join(repoRoot, "plugins", "himd");
const pluginPkgPath = path.join(pluginDir, "package.json");
const pluginManifestPath = path.join(pluginDir, ".claude-plugin", "plugin.json");
const hiCommandPath = path.join(pluginDir, "commands", "hi.md");

for (const requiredPath of [pluginPkgPath, pluginManifestPath, hiCommandPath]) {
  if (!fs.existsSync(requiredPath)) {
    throw new Error(`Missing required plugin file: ${requiredPath}`);
  }
}

const pluginPkg = JSON.parse(fs.readFileSync(pluginPkgPath, "utf8"));
const pluginManifest = JSON.parse(fs.readFileSync(pluginManifestPath, "utf8"));

if (pluginPkg.version !== pluginManifest.version) {
  throw new Error(
    `Plugin version mismatch: package.json=${pluginPkg.version}, plugin.json=${pluginManifest.version}`
  );
}

if (fs.existsSync(path.join(pluginDir, ".mcp.json"))) {
  throw new Error("plugins/himd/.mcp.json must not exist");
}

console.log("Plugin validation passed");
