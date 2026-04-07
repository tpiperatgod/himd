const test = require("node:test");
const assert = require("node:assert/strict");

const { MARKER_PATH, PROFILE_PATH, buildAudioFilePath } = require("../tts.js");

test("tts runtime files use the himd prefix", () => {
  assert.equal(PROFILE_PATH, "/tmp/himd-voice-profile.json");
  assert.equal(MARKER_PATH, "/tmp/himd-last-speech-turn");
  assert.equal(buildAudioFilePath(123), "/tmp/himd-tts-123.wav");
});
