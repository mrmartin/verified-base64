# Project Plan: `verified-base64`

**One-line goal:** Publish a public GitHub repository in which the hot path of the most popular Rust base64 crate is machine-checked in Lean 4 against a directly-transcribed RFC 4648 specification, continuously re-verified in CI, differentially fuzzed against the upstream binary, with a per-theorem cost ledger and a verification badge — serving as both proof-of-capability and customer intake funnel.

**Strategic function:** This is a sales artifact, not a research artifact. Every design decision below is subordinate to two outcomes: (a) a skeptical senior engineer can re-run the verification in one command and find no hand-waving; (b) a prospective customer can see the published cost-per-theorem and conclude that human-consultant pricing is obsolete.

---

## 1. Result definition (acceptance criteria)

The project is **done** when all of the following hold:

1. **Theorems proven and kernel-checked.** `lake build` succeeds from a clean clone with a pinned toolchain, with zero `sorry`, zero `axiom` beyond Lean's standard foundations, and `#print axioms` output for each main theorem committed to the repo.
2. **The four core theorems exist** (formal statements in §4):
   - T1 round-trip: `decode (encode b) = ok b` for all byte arrays.
   - T2 canonical inverse: `encode (decode s) = s` for all *valid canonical* strings.
   - T3 totality / no-panic: `decode` and `encode` are total functions returning `Result`, never panicking, for *all* inputs.
   - T4 RFC conformance: the extracted implementation equals the executable RFC 4648 spec function, pointwise.
3. **Differential harness has executed ≥ 10⁸ inputs** between the Lean executable spec and the upstream `base64` crate binary with zero unexplained divergences, and runs as a nightly CI job. Any *explained* divergence (e.g. non-canonical input acceptance) is documented in `DIVERGENCES.md` — this is a feature, not a failure.
4. **Cost ledger** `costs.csv` is committed and current: one row per theorem with model, attempt count, token usage, USD cost, wall-clock time.
5. **CI badge** is live: green = kernel re-checked on latest commit, served via shields.io endpoint JSON from GitHub Pages.
6. **Intake funnel** is live: issue template "Request a verification" with the free-tier offer, plus `SPEC.md` documenting every formalization judgment call.
7. **README** passes the "skeptical HN commenter" test: trusted computing base is stated explicitly (Lean kernel + Charon/Aeneas translation + spec transcription), with the differential harness positioned as the mitigation for the translation gap.

**Explicit non-goals (v1):** SIMD/AVX paths of the upstream crate, streaming/chunked encoders, constant-time properties, `no_std` alloc edge cases beyond what the core loop needs, MIME/PEM line-wrapping variants (RFC 2045). Note each as out-of-scope in README; several are natural paid follow-ons.

---

## 2. Repository layout

```
verified-base64/
├── README.md                  # narrative, TCB statement, how to re-verify in 1 command
├── SPEC.md                    # every formalization choice, with RFC section references
├── DIVERGENCES.md             # explained model↔binary divergences (canonicity etc.)
├── costs.csv                  # per-theorem cost ledger
├── rust/
│   ├── base64-core/           # minimal extracted encode/decode, no_std, no deps
│   │   └── src/lib.rs
│   └── difftest/              # differential harness driver (links upstream crate)
│       └── src/main.rs
├── lean/
│   ├── lakefile.lean
│   ├── lean-toolchain          # pinned, e.g. leanprover/lean4:v4.x.0
│   ├── Verified/Spec.lean      # executable RFC 4648 spec (the ground truth)
│   ├── Verified/Extracted.lean # Aeneas output from base64-core (generated, committed)
│   ├── Verified/Theorems.lean  # T1–T4
│   └── Verified/Main.lean      # CLI entry: stdin hex → encode/decode → stdout hex
├── .github/
│   ├── workflows/verify.yml    # lake build + axiom audit on every push
│   ├── workflows/difftest.yml  # nightly 10⁷-input differential run
│   └── ISSUE_TEMPLATE/request-verification.yml
├── badge/                      # gh-pages endpoint JSON for shields.io
└── prover/
    ├── CLAUDE.md               # operating constitution for the autonomous proving loop
    └── log/                    # raw attempt logs feeding costs.csv
```

---

## 3. Phase plan

### Phase 0 — Environment and target pinning (0.5 day)

**Do:**
- Pin the target: `base64` crate (marshallpierce/rust-base64), latest stable release at project start; record exact version + commit hash in README. This is the most-downloaded base64 crate on crates.io; the demo's force comes from the install base.
- Pin Lean: latest stable Lean 4 via `elan`, committed `lean-toolchain` file. Pin mathlib only if needed (prefer avoiding the dependency; `omega`, `simp`, `decide`, and `bv_decide` ship with core/std and should suffice for bit-twiddling).
- Install Charon + Aeneas from their GitHub repos; pin both by commit hash in README (these move fast and translation output is part of the TCB — reproducibility is non-negotiable).
- Smoke-test the toolchain: translate a 10-line Rust function through Charon → Aeneas → Lean and prove something trivial about it. **Do this before anything else** — Aeneas's supported-Rust subset is the project's main technical risk, and we want to discover friction on day one, not day ten.

**Result:** a `docs/TOOLCHAIN.md` with exact versions and a passing hello-world translation.

### Phase 1 — Executable RFC 4648 specification in Lean (1–2 days)

This is the intellectually load-bearing phase. The spec is the ground truth everything else is measured against; an error here invalidates the project silently.

**Do:**
- Transcribe RFC 4648 §4 (standard alphabet, `+/`, `=` padding) into `Spec.lean` as direct, naive, *obviously-correct* Lean functions over `ByteArray`. Optimize for auditability, not speed: a reviewer must be able to hold the spec and the RFC side by side and check line-by-line correspondence. Include §5 (URL-safe alphabet `-_`) as a parameterized alphabet — near-free generality and doubles the conformance surface.
- Decode must be **strict**: reject any character outside the alphabet, reject bad padding placement, reject incorrect padded length, and **reject non-canonical final symbols** (RFC 4648 §3.5: trailing bits beyond the payload must be zero — e.g. `"QQ=="` is canonical for byte `0x41`, but `"QR=="` decodes to the same byte with nonzero discarded bits). Canonicity rejection is the deliberate choice to document loudly in `SPEC.md`, because *this is exactly where real-world implementations diverge* and where the differential harness is most likely to surface interesting behavior in upstream crates. The upstream `base64` crate exposes this via `DecodePaddingMode`/canonical checks — record which configuration of the upstream crate our spec corresponds to.
- Every nontrivial transcription choice gets an entry in `SPEC.md`: RFC section quoted by reference, the Lean rendering, and one sentence of justification. This document is simultaneously the audit trail and the advertisement for the spec-authorship skill a paid engagement buys.
- Compile `Spec.lean` to a native executable (`lake exe`) with a trivial CLI protocol: `encode`/`decode` subcommand, hex on stdin, hex (or `ERR:<reason>`) on stdout. This executable is one side of the differential harness.
- Unit-test the spec against RFC 4648 §10 test vectors (`""`, `"f"`, `"fo"`, `"foo"`, `"foob"`, `"fooba"`, `"foobar"`) plus a hand-built negative suite: bad chars, bad padding, non-canonical finals, embedded `=`, length ≡ 1 mod 4.

**Result:** `Spec.lean` + `SPEC.md` + passing vector tests + native spec executable.

### Phase 2 — Extraction and translation (1–2 days, risk-bearing)

**Do:**
- Create `rust/base64-core`: a minimal `no_std`, zero-dependency crate containing the scalar encode/decode logic **ported line-by-line from the upstream crate's source** (the non-SIMD path), preserving structure, lookup tables, and control flow as faithfully as Aeneas's input subset allows. Document every deviation (e.g. replacing an unsupported iterator chain with an explicit loop) in a `PORT.md` table: upstream lines ↔ core lines ↔ reason.
- Why the port instead of translating the upstream crate directly: Aeneas consumes a restricted Rust subset (no arbitrary trait objects, limited unsafe, etc.) and the upstream crate's engine abstraction + SIMD dispatch will not pass through. The port keeps the trusted gap **small, explicit, and itself differentially tested** (Phase 4 fuzzes `base64-core` against upstream too, closing the port-fidelity question empirically).
- Run Charon on `base64-core` to get the LLBC, then Aeneas targeting Lean. Commit the generated `Extracted.lean` (regenerable, but committed output keeps `lake build` self-contained and the diff reviewable).
- Iterate the port until translation succeeds. Budget expectation: 2–4 rounds of rewriting table lookups / slice patterns into Aeneas-friendly form. If Aeneas proves unworkable on some construct, fallback option is hax → (F*/Coq backend is more mature, but staying in Lean is strongly preferred for the autonomous-prover leverage) — escalate before switching.

**Result:** `Extracted.lean` building under `lake build`, `PORT.md` complete.

### Phase 3 — Theorems via the autonomous proving loop (3–7 days wall clock, mostly unattended)

**Formal statements (adjust names to Aeneas output conventions):**

- **T1 (round-trip):** `∀ (b : ByteArray) (alpha : Alphabet), Extracted.decode alpha (Extracted.encode alpha b) = .ok b`
- **T2 (canonical inverse):** `∀ (s : ByteArray) (alpha), Spec.validCanonical alpha s → Extracted.encode alpha ((Extracted.decode alpha s).get) = s`
- **T3 (totality):** Aeneas renders panics as the `.fail` constructor of its result monad; prove `∀ input, Extracted.decode alpha input ≠ .fail .panic` (and same for encode). With Aeneas this is often nearly free once T4 closes, but state it as a separate named theorem — "never panics on any input" is the line item a customer understands instantly.
- **T4 (conformance, the keystone):** `∀ (b : ByteArray) (alpha), Extracted.encode alpha b = Spec.encode alpha b` and `∀ (s : ByteArray) (alpha), Extracted.decode alpha s ≃ Spec.decode alpha s` (≃ = agreement up to error-value mapping, defined explicitly). Prove T1/T2 about `Spec` first (cleaner induction), then transport through T4 — this decomposition is also the right shape for the prover loop, since spec-level lemmas are easier and build the lemma library the conformance proof needs.

**How (the proving loop):**
- Run Claude Code unattended against the Lean project, per the existing PutnamBench-style operating constitution, adapted in `prover/CLAUDE.md`: the verifier is `lake build`; one theorem (or named lemma) per work item; every attempt appends a JSON line to `prover/log/` (timestamp, target, model, tokens in/out, outcome); no `sorry`/`axiom` may ever be committed to a green branch; decompose into ≤40-line lemmas when stuck > N attempts; prefer `omega` for index arithmetic, `bv_decide`/`decide` for per-symbol bit identities, `simp` sets built up in a project-local attribute.
- Expected proof structure: per-3-byte-block encode lemma and per-4-symbol decode lemma proved by `decide`/`bv_decide` over the finite symbol domain, then induction on blocks for the loop, with explicit handling of the 1- and 2-byte tail + padding cases. The lookup tables produce 256-way case lemmas — these are exactly what decision procedures eat for breakfast; the human-attention items are the loop invariants and the tail-case statements.
- Human checkpoints (Martin, ~1–2 h each): (1) review T1–T4 *statements* before proving starts — a proved wrong statement is the failure mode that matters; (2) review the final axiom audit; (3) review `SPEC.md` for honesty.
- After completion, a script aggregates `prover/log/` → `costs.csv`: `theorem, model, attempts, tokens_in, tokens_out, usd, wallclock_h`. Commit it. **This file is a primary deliverable** — publish real numbers, including failed-attempt costs; the honesty is the marketing.

**Result:** T1–T4 green in CI, `#print axioms` audit committed, `costs.csv` current.

### Phase 4 — Differential fuzzing harness (1–2 days)

The harness closes the two trust gaps proofs cannot: spec-transcription fidelity and port fidelity.

**Do:**
- `rust/difftest`: a driver that generates inputs and compares **three implementations pairwise**: (A) upstream `base64` crate (pinned), (B) `base64-core` port, (C) Lean spec executable (subprocess, hex protocol; batch 10³ inputs per process spawn to amortize). A↔B validates the port; B↔C validates translation+spec; A↔C is the headline pairing.
- Input generation, three streams: (1) random byte arrays (lengths 0–4096, biased to 0–8 for tail-case density) through the encode path; (2) random *strings over a weighted alphabet* (valid symbols, `=`, near-miss bytes like `@ [ ` { /`+1`, whitespace, high bytes) through the decode path — uniform random bytes almost never exercise the interesting decode branches, so the weighting matters; (3) structured adversarial corpus: every length mod 4, every padding arrangement, every non-canonical final-symbol pair, single-bit-flips of valid encodings.
- Comparison semantics: encode must match byte-exact; decode must match on ok/err status and on payload when ok. Where upstream's *configured mode* legitimately differs from the strict spec (e.g. upstream configured to accept non-canonical input), record the configuration mapping in `DIVERGENCES.md` and assert the *expected* relationship instead of equality — an explained, documented divergence between crates is publishable material, not noise.
- Optionally add `cargo-fuzz` coverage-guided fuzzing on A↔B (in-process, fast); keep the spec-executable comparison on the random/structured streams (subprocess-rate-bound).
- CI: nightly job, 10⁷ inputs, failing run uploads the offending input as an artifact and opens an issue automatically. One-off local run to 10⁸ before launch.

**Result:** zero unexplained divergences at 10⁸; `DIVERGENCES.md` populated; nightly job green.

### Phase 5 — CI, badge, intake (1 day)

**Do:**
- `verify.yml`: on every push/PR — elan-pinned toolchain, `lake build`, grep-guard against `sorry`/`axiom`/`native_decide` (decide policy: allow `decide`, forbid `native_decide` to keep the compiler out of the TCB; state this in README), run the `#print axioms` audit script, run RFC vector tests. Cache `.lake` aggressively; cold Lean builds will otherwise dominate CI time.
- Badge: a tiny job publishes `badge/verified.json` (shields.io endpoint schema: label "formally verified", message = short commit hash, green) to `gh-pages` only on green `verify.yml`. README embeds it at the top. The badge must be backed by the *re-check*, not hand-set — that mechanical honesty is the entire value of the certification mark.
- `request-verification.yml` issue template, fields: link to source (repo + path + commit), the property in plain English, why it matters / what breaks if it's false, license. Pin an issue: **"First verification request per organization is free"** with a one-paragraph description of what they receive (theorem statements for sign-off → kernel-checked proofs in a public repo → cost ledger entry → badge).

**Result:** live badge, live intake, one-command re-verification documented in README.

### Phase 6 — Publication package (0.5 day, coordinate with Martin)

**Do (prepare; Martin times the release):**
- README narrative order: (1) badge + one-line claim, (2) "verify it yourself" one-liner (`elan ... && lake build`), (3) what exactly is proven (T1–T4 in plain English), (4) **trusted computing base, stated before anyone asks**: Lean kernel, Charon/Aeneas pinned commits, the spec transcription (mitigated by `SPEC.md` audit + RFC vectors), the port (mitigated by `PORT.md` + A↔B fuzzing), (5) the cost table inline — this is the screenshot people will share, (6) divergence findings if any, (7) intake offer, (8) non-goals.
- Drafts for Show HN and r/rust. Working titles: "Show HN: We formally verified rust-base64's decoder against RFC 4648, autonomously, for $X" — the dollar number goes in the title. If the harness surfaced a real upstream divergence, that leads instead.
- Tag `v1.0.0`, archive the exact prover logs behind `costs.csv`.

---

## 4. Schedule and budget

| Phase | Duration | Compute budget |
|---|---|---|
| 0 Toolchain | 0.5 d | — |
| 1 Spec | 1–2 d | — |
| 2 Extraction | 1–2 d | — |
| 3 Proofs | 3–7 d (unattended) | $50–150 API (or MAX-plan time) |
| 4 Difftest | 1–2 d | ~$5 CI |
| 5 CI/badge/intake | 1 d | — |
| 6 Publication | 0.5 d | — |
| **Total** | **~8–15 calendar days** | **< $200** |

Phases 1, 2, 4 are parallelizable after Phase 0. The critical path is 0 → 2 → 3.

## 5. Risks and responses

| Risk | Likelihood | Response |
|---|---|---|
| Aeneas can't ingest the ported loop | Medium | Rewrite into the supported subset (explicit loops, index arith); PORT.md records deviations; A↔B fuzzing keeps the port honest. Escalate before considering hax/F*. |
| Conformance proof (T4) stalls in the prover loop | Medium | Decompose: prove spec-side T1/T2 first; per-symbol lemmas by `decide`; human writes the loop invariant *statement* only, prover fills the proof. This is the one place to spend human Lean skill. |
| Spec transcription error | Low but fatal | RFC §10 vectors + adversarial unit suite + 10⁸ differential inputs + SPEC.md line-by-line audit. Three independent nets. |
| Upstream divergence found is actually our bug | Medium | Every divergence triages through A↔B vs B↔C pairing to localize fault before any public claim. No disclosure of an upstream issue without a minimized repro and maintainer contact first. |
| "You only verified a port" objection | Certain | Pre-empt in README TCB section; the A↔B fuzz stream is the standing answer. Offer direct-translation of upstream as a paid follow-on. |

## 6. Handover checklist for development

- [ ] Phase 0 toolchain doc committed; hello-world translation green
- [ ] `Spec.lean` + `SPEC.md` + vector tests green; spec executable builds
- [ ] `base64-core` + `PORT.md`; `Extracted.lean` builds
- [ ] T1–T4 statements reviewed and signed off by Martin **before** unattended proving begins
- [ ] T1–T4 proven; axiom audit clean; `costs.csv` committed
- [ ] 10⁸ differential inputs, zero unexplained divergences; `DIVERGENCES.md` written
- [ ] CI + badge + intake template live
- [ ] README TCB section reviewed by Martin; publication drafts ready
