#!/usr/bin/env bash
# test-audio-provider.sh — Manual verification script for Qwen Omni audio understanding
#
# Usage:
#   ./scripts/test-audio-provider.sh <audio-file>
#
# Examples:
#   ./scripts/test-audio-provider.sh recording.wav
#   ./scripts/test-audio-provider.sh recording.mp3
#
# Prerequisites:
#   - DASHSCOPE_API_KEY set

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

FILE="${1:?Usage: $0 <audio-file>}"

if [ ! -f "$FILE" ]; then
  echo "Error: File not found: $FILE"
  exit 1
fi

echo "=== Audio Provider Test ==="
echo "File: $FILE"
echo ""

node -e "
const path = require('path');

const filePath = path.resolve('$FILE');

// Test 1: Base64 conversion
console.log('--- Test 1: Base64 conversion ---');
const { fileToBase64, buildAudioDataUrl } = require('${PROJECT_DIR}/packages/voice-bridge/audio-utils.js');
const { base64, ext, mimeType } = fileToBase64(filePath);
console.log('Extension:', ext, '| MIME:', mimeType, '| Base64 length:', base64.length);
const dataUrl = buildAudioDataUrl(filePath);
console.log('Data URL prefix:', dataUrl.slice(0, 50) + '...');

// Test 2: JSON parsing
console.log('\n--- Test 2: JSON parsing ---');
const { parseJsonResponse } = require('${PROJECT_DIR}/packages/voice-bridge/audio-utils.js');
const testCases = [
  '{\"transcript\":\"hello\",\"intent\":\"greeting\"}',
  '\`\`\`json\n{\"transcript\":\"hello\"}\n\`\`\`',
  '{\"transcript\":\"incomplete',
  'Some text before {\"transcript\":\"hello\"} some text after',
];
for (const tc of testCases) {
  const parsed = parseJsonResponse(tc);
  console.log('Input:', JSON.stringify(tc).slice(0, 60));
  console.log('Parsed:', parsed ? 'OK (' + Object.keys(parsed).join(',') + ')' : 'null');
}

// Test 3: Qwen Omni provider call
console.log('\n--- Test 3: Provider.understand() ---');
const provider = require('${PROJECT_DIR}/packages/voice-bridge/providers/qwen-omni-provider.js');
console.log('Provider:', provider.name);

provider.understand(filePath)
  .then(result => {
    console.log('Transcript:', result.transcript || '(empty)');
    console.log('Provider:', result.provider);
    console.log('Model:', result.model);
    console.log('Confidence:', result.confidence);
    if (result.summary) console.log('Summary:', result.summary);
    if (result.intent) console.log('Intent:', result.intent);
    if (result.emotion) console.log('Emotion:', JSON.stringify(result.emotion));
    if (result.tone) console.log('Tone:', result.tone.join(', '));
    if (result.key_points) console.log('Key points:', result.key_points.join('; '));
    if (result.warnings && result.warnings.length > 0) console.log('Warnings:', result.warnings);
    console.log('\n=== PASS ===');
  })
  .catch(err => {
    console.error('ERROR:', err.message);
    process.exit(1);
  });
"
