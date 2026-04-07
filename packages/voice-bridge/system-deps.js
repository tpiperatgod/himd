const { spawnSync } = require("node:child_process");

/**
 * Assert that a required system command is available.
 * Throws with an actionable install hint if not found.
 * @param {string} command - The binary name to check
 * @param {string} installHint - Instructions for installing the command
 */
function assertCommandAvailable(command, installHint) {
  const result = spawnSync("which", [command], { encoding: "utf8" });
  if (result.status === 0) {
    return;
  }

  throw new Error(
    `Required command not found: ${command}. Install it and try again. Hint: ${installHint}`
  );
}

module.exports = { assertCommandAvailable };
