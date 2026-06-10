# DIVERGENCES.md — explained divergences between the compared implementations

The differential harness compares, pairwise on every input:
**A** the vendored upstream `base64` crate (`engine::general_purpose::STANDARD`
/ `URL_SAFE`), **B** the `base64-core` port, **C** the Lean spec executable
(`speccli`). A separate binary compares the **published crates.io
`base64 = 0.22.1`** against B on the same streams.

## Current status

| Pairing | Inputs run | Unexplained divergences | Explained divergences |
|---|---|---|---|
| A ↔ B ↔ C (all three, payload-exact) | 10⁸ stream + adversarial corpus (seed 20260613, 2026-06-10) | **0** | 0 |
| crates.io 0.22.1 ↔ B | 10⁷ stream + corpus (seed 20260613) | **0** | see note 1 |

No divergence of any kind has been observed so far.

## Explained / structural notes (not divergences)

1. **`InvalidLastSymbol` payload shape.** crates.io 0.22.1 reports
   `InvalidLastSymbol(offset, symbol)`; the vendored fork (post-PR-#293) and
   the port add the decoded 6-bit `symbol_value`. The harness compares the
   value field only when both sides expose it. The *condition* under which the
   error fires is identical; only the payload grew a field upstream after
   0.22.1.
2. **Configurations out of scope.** The comparison is exclusively about the
   strict engines (`PAD` + `RequireCanonical` + reject trailing bits). Upstream
   supports laxer configurations (`Indifferent`, `RequireNone`,
   `decode_allow_trailing_bits`) which by design accept inputs the spec
   rejects; those configurations are v1 non-goals and are not compared.

A failing nightly run uploads the offending inputs as a replay artifact
(`difftest --replay` re-runs them) and is triaged by pairing — A↔B vs B↔C —
to localize the fault before any public claim.
