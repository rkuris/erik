#!/usr/bin/env bash
set -euo pipefail

# Run tests inside the container. Accepts Playwright args (optional).
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
cd "$SCRIPT_DIR"

# Ensure deps are installed (idempotent)
if [ ! -d node_modules ]; then
  npm ci
  npx playwright install --with-deps
fi

# Default to running full test suite
if [ "$#" -eq 0 ]; then
  npx playwright test
else
  npx playwright test "$@"
fi
