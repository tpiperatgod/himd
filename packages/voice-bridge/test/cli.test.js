const test = require("node:test");
const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const path = require("node:path");
const pkg = require("../package.json");

const cliPath = path.resolve(__dirname, "../bin/himd-voice-bridge.js");

test("CLI --help succeeds", () => {
  const result = spawnSync("node", [cliPath, "--help"], { encoding: "utf8" });
  assert.equal(result.status, 0);
  assert.match(result.stdout, /himd-voice-bridge/);
});

test("CLI --version returns package version", () => {
  const result = spawnSync("node", [cliPath, "--version"], { encoding: "utf8" });
  assert.equal(result.status, 0);
  assert.equal(result.stdout.trim(), pkg.version);
});
