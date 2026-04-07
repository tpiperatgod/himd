import fs from "node:fs";

const pkg = JSON.parse(fs.readFileSync("plugins/himd/package.json", "utf8"));
const manifestPath = "plugins/himd/.claude-plugin/plugin.json";
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));

manifest.version = pkg.version;

fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Synced plugin manifest version to ${pkg.version}`);
