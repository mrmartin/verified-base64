#!/bin/bash
# The only sanctioned path from rust/base64-core to lean/Verified/Extracted/.
# Requires the pinned toolchain from docs/TOOLCHAIN.md:
#   - charon on PATH or at $CHARON_BIN (default ~/tools/charon/bin/charon)
#   - aeneas at $AENEAS_BIN (default ~/tools/aeneas/bin/aeneas)
# The generated files are committed; regenerate + diff to detect drift.
set -euo pipefail
cd "$(dirname "$0")/.."

CHARON_BIN="${CHARON_BIN:-$HOME/tools/charon/bin/charon}"
AENEAS_BIN="${AENEAS_BIN:-$HOME/tools/aeneas/bin/aeneas}"
LLBC=/tmp/base64_core.llbc

echo "== charon: rust/base64-core -> LLBC =="
(cd rust/base64-core && PATH="$(dirname "$CHARON_BIN"):$PATH" \
  "$CHARON_BIN" cargo --preset=aeneas --dest-file="$LLBC")

echo "== aeneas: LLBC -> lean/Verified/Extracted =="
"$AENEAS_BIN" -backend lean "$LLBC" -dest lean -subdir Verified/Extracted -split-files

echo "== generated =="
ls -la lean/Verified/Extracted/
