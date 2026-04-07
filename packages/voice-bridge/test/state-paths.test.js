const test = require("node:test");
const assert = require("node:assert/strict");
const os = require("node:os");
const path = require("node:path");

const { getBaseDir, getControlDir, generateOutputPath } = require("../capture.js");

test("runtime state defaults to an OS temp directory instead of the install path", () => {
  assert.equal(getBaseDir(), path.join(os.tmpdir(), "himd-voice-bridge"));
});

test("control dir lives under the runtime state directory", () => {
  assert.equal(getControlDir(), path.join(getBaseDir(), "control"));
});

test("capture output stays under the runtime state directory", () => {
  const outputPath = generateOutputPath(getBaseDir());
  assert.match(outputPath, new RegExp(`^${getBaseDir().replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`));
});
