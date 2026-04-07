#!/bin/bash
# Fallback TTS: speaks the last assistant message if speech.say wasn't called.
# Called by a stop hook when Claude's turn ends without TTS.

MARKER_FILE="/tmp/himd-last-speech-turn"

# Check if speech.say was already called in the last 60 seconds
if [ -f "$MARKER_FILE" ]; then
  marker_ts=$(cat "$MARKER_FILE")
  now_ms=$(python3 -c "import time; print(int(time.time()*1000))")
  age=$(( now_ms - marker_ts ))
  if [ "$age" -lt 60000 ]; then
    exit 0
  fi
fi

echo "[himd fallback] No TTS detected this turn. Primary path (speech.say) should handle TTS."
echo "[himd fallback] If you see this frequently, the /hi skill prompt may need adjustment."
exit 0
