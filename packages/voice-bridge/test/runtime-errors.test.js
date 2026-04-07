const test = require("node:test");
const assert = require("node:assert/strict");

test("synthesize fails with explicit ZHIPU_API_KEY guidance", async () => {
  delete process.env.ZHIPU_API_KEY;
  // Clear require cache so the module re-evaluates without the key
  delete require.cache[require.resolve("../tts.js")];
  const { synthesize } = require("../tts.js");
  await assert.rejects(() => synthesize("hello"), /ZHIPU_API_KEY/);
});

test("missing binary produces an actionable install hint", () => {
  const { assertCommandAvailable } = require("../system-deps.js");
  assert.throws(
    () => assertCommandAvailable("definitely-missing-binary", "brew install ffmpeg"),
    /brew install ffmpeg/
  );
});
