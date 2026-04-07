#!/usr/bin/env node

const pkg = require("../package.json");

if (process.argv.includes("--version")) {
  console.log(pkg.version);
  process.exit(0);
}

if (process.argv.includes("--help")) {
  console.log(`@himd/voice-bridge ${pkg.version}

Starts the himd local stdio MCP server.

Usage:
  himd-voice-bridge
  himd-voice-bridge --help
  himd-voice-bridge --version
`);
  process.exit(0);
}

require("../index.js");
