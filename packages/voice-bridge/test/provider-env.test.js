const test = require("node:test");
const assert = require("node:assert/strict");

function loadFactory() {
  delete require.cache[require.resolve("../providers/audio-provider.js")];
  return require("../providers/audio-provider.js");
}

function withEnv(nextEnv, fn) {
  const keys = ["HIMD_AUDIO_PROVIDER", "HIMD_FALLBACK_PROVIDER"];
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

test("HIMD_AUDIO_PROVIDER selects provider", () => {
  withEnv({ HIMD_AUDIO_PROVIDER: "glm-asr" }, () => {
    const { getAudioProvider } = loadFactory();
    assert.equal(getAudioProvider().name, "glm-asr");
  });
});

test("defaults to qwen-omni when unset", () => {
  withEnv({}, () => {
    const { getAudioProvider } = loadFactory();
    assert.equal(getAudioProvider().name, "qwen-omni");
  });
});

test("HIMD_FALLBACK_PROVIDER can disable fallback", () => {
  withEnv({ HIMD_FALLBACK_PROVIDER: "none" }, () => {
    const { getFallbackProvider } = loadFactory();
    assert.equal(getFallbackProvider(), null);
  });
});
