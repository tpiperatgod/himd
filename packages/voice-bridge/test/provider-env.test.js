const test = require("node:test");
const assert = require("node:assert/strict");

function loadProvider() {
  delete require.cache[require.resolve("../providers/qwen-omni-provider.js")];
  return require("../providers/qwen-omni-provider.js");
}

function withEnv(nextEnv, fn) {
  const keys = ["AUDIO_MODEL"];
  const prev = Object.fromEntries(keys.map((key) => [key, process.env[key]]));
  for (const key of keys) delete process.env[key];
  Object.assign(process.env, nextEnv);
  try {
    fn();
  } finally {
    for (const key of keys) {
      if (prev[key] === undefined) delete process.env[key];
      else process.env[key] = prev[key];
    }
  }
}

test("AUDIO_MODEL defaults to qwen3-omni-flash", () => {
  withEnv({}, () => {
    const { getAudioModel } = loadProvider();
    assert.equal(getAudioModel(), "qwen3-omni-flash");
  });
});

test("AUDIO_MODEL override is respected", () => {
  withEnv({ AUDIO_MODEL: "qwen3-omni-flash" }, () => {
    const { getAudioModel } = loadProvider();
    assert.equal(getAudioModel(), "qwen3-omni-flash");
  });
});
