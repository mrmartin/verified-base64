#!/bin/bash
# Per-work-item axiom audit (prover/CLAUDE.md rule 11a).
#
# Usage: scripts/wi_audit.sh <FullyQualifiedDeclName> [more names...]
#
# For each declaration: prints its axioms and FAILS if anything beyond Lean's
# standard three (propext, Classical.choice, Quot.sound) appears — in
# particular `sorryAx`, which would mean the proof leans (transitively) on a
# sorry'd declaration, e.g. the known Aeneas-library holes:
#   Aeneas.Std.core.slice.Slice.get_unchecked                          (def by sorry)
#   Aeneas.Std.core.slice.Slice.get_unchecked_SliceIndexUsizeSlice_spec (@[step] lemma by sorry)
#   Aeneas.Std.core.str.iter.IteratorChars.collect                     (def by sorry)
set -euo pipefail
cd "$(dirname "$0")/../lean"

if [ $# -eq 0 ]; then
  echo "usage: $0 <FullyQualifiedDeclName> [more...]" >&2
  exit 2
fi

tmp=$(mktemp /tmp/wi_audit_XXXX.lean)
trap 'rm -f "$tmp"' EXIT
{
  echo "import Verified"
  for decl in "$@"; do
    echo "#print axioms $decl"
  done
} > "$tmp"

out=$(lake env lean "$tmp" 2>&1)
echo "$out"

fail=0
if echo "$out" | grep -q 'sorryAx'; then
  echo "WI-AUDIT FAIL: sorryAx detected — proof depends on a sorry'd declaration" >&2
  fail=1
fi
if echo "$out" | grep -q 'error'; then
  echo "WI-AUDIT FAIL: lean reported an error" >&2
  fail=1
fi
# anything beyond the standard three?
extra=$(echo "$out" | grep -o "depends on axioms: \[[^]]*\]" \
  | sed 's/depends on axioms: \[//; s/\]//' | tr ',' '\n' | sed 's/ //g' \
  | grep -v -E '^(propext|Classical\.choice|Quot\.sound)$' | sort -u || true)
if [ -n "$extra" ]; then
  echo "WI-AUDIT FAIL: non-standard axioms: $extra" >&2
  fail=1
fi

if [ "$fail" -eq 0 ]; then
  echo "WI-AUDIT OK: only [propext, Classical.choice, Quot.sound]"
fi
exit "$fail"
