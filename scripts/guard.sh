#!/bin/bash
# Guards against proof-debt escaping to master:
#  - no `sorry` / `admit` anywhere in the Lean sources (including Extracted)
#  - no `native_decide` (would put the Lean compiler in the TCB)
#  - no `axiom` declarations in project files (the only axioms allowed are
#    Lean's standard ones, checked by the #print axioms audit)
# Excludes .lake (dependencies are audited via #print axioms, not grep).
set -uo pipefail
cd "$(dirname "$0")/.."

fail=0

hits=$(grep -rnE '\b(sorry|admit)\b' lean/Verified* --include='*.lean' 2>/dev/null)
if [ -n "$hits" ]; then
  echo "GUARD FAIL: sorry/admit found:" >&2
  echo "$hits" >&2
  fail=1
fi

hits=$(grep -rn 'native_decide' lean/Verified* --include='*.lean' 2>/dev/null)
if [ -n "$hits" ]; then
  echo "GUARD FAIL: native_decide found:" >&2
  echo "$hits" >&2
  fail=1
fi

hits=$(grep -rnE '^\s*axiom\b' lean/Verified* --include='*.lean' 2>/dev/null)
if [ -n "$hits" ]; then
  echo "GUARD FAIL: axiom declaration found:" >&2
  echo "$hits" >&2
  fail=1
fi

if [ "$fail" -eq 0 ]; then
  echo "guards OK: no sorry/admit/native_decide/axiom in lean/Verified*"
fi
exit "$fail"
