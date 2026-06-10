# Prover loop operating constitution — verified-base64

You are proving the theorems of the verified-base64 project (see
/PLAN_verified-base64.md §3 and /STATEMENTS.md). This file is binding policy
for every proving session, human-attended or not.

## Ground rules

1. **The verifier is `lake build`** (run in `lean/`). Nothing counts until the
   full package builds. Scoped builds (`lake build +Verified.Theorems.Tables`)
   are fine for iteration; the work item closes only on a full green build.
2. **Never commit `sorry`, `admit`, custom `axiom`, or `native_decide` to
   master.** Master is always sorry-free; proving happens on `proving/*`
   branches; merge only green. `decide` is allowed; `native_decide` is
   forbidden (it would put the Lean compiler into the TCB — stated in README).
3. **Statements are frozen.** Theorem statements in `Verified/Theorems/` were
   signed off at checkpoint C1. Do not change a statement, weaken a
   hypothesis, or strengthen a conclusion without explicit human sign-off. If
   a statement appears false, STOP: write up the counterexample evidence and
   escalate — a falsified statement is a project-level event (it means the
   spec, the port, or the statement is wrong), not a proof engineering
   problem.
4. **Do not edit** `Verified/Spec.lean`, `Verified/Spec/Tests.lean`,
   `Verified/Extracted/*` (generated), `Verified/Main.lean`, or any Rust code.
   Helper lemmas go in the `Verified/Theorems/` module that uses them.

## Work discipline

5. **One named work item at a time** (WI-01 … WI-15, order in STATEMENTS.md).
   A work item is one theorem or one named helper lemma.
6. **Decompose when stuck.** After ~5 failed strategies on one goal, extract
   the blocking subgoal as a named helper lemma (≤ 40 lines), prove it
   separately, and continue. Helper lemmas inherit the logging duty.
7. **Tactic policy.** Prefer, in order of cheapness: `decide` for finite/table
   facts (64- and 256-way); `bv_decide` for per-block bit identities on
   fixed-width words; `omega` for index arithmetic; `simp` with a curated
   local simp set; Aeneas's `step`/`step*`/`scalar_tac` for extracted-code
   goals; structural induction only where the statement demands it
   (block-recursive functions). Never `decide` over a domain bigger than 2¹⁶
   (e.g. never enumerate 3-byte blocks — use `bv_decide` instead).
8. **decode_suffix is proved by finite unrolling**, not loop induction: case
   split on the suffix length (0–4) and symbolically execute. If the
   extracted loop shape resists unrolling, escalate to the human for an
   invariant statement rather than burning attempts.

## Logging (mandatory — feeds the public cost ledger)

9. Every attempt appends one JSON line to `prover/log/<UTC-date>-<target>.jsonl`:

   ```json
   {"ts": "2026-06-12T14:03:55Z", "target": "encode_quads_loop_spec",
    "phase": "T4e", "model": "<model-id>", "attempt": 3,
    "tokens_in": 0, "tokens_out": 0, "usd": 0.0, "wall_s": 412,
    "outcome": "green|fail|decomposed|escalated", "note": "<one line>"}
   ```

   `phase` ∈ {S1, S2, T4e, T4d, T3, T1, T2, infra}. Token/cost fields may be
   filled by the session wrapper afterwards; never fabricate them — leave 0
   and let the wrapper backfill from session accounting.
10. Failed attempts are logged, not erased. The public `costs.csv` includes
    failure costs by design — honesty is the marketing.
11. After each green work item: `python3 prover/aggregate_costs.py > costs.csv`,
    commit on the proving branch with the work item in the message.

## Session etiquette

12. At session start: `git pull`, confirm clean tree, confirm `lake build` is
    green before touching anything. At session end: leave the branch green or
    explicitly mark the last work item `fail` in the log.
13. Time-box: if a work item exceeds ~2h wall without progress, log
    `escalated`, write a STUCK.md note (goal state, what was tried), move on.
