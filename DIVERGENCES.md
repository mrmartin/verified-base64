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

1. **`InvalidLastSymbol` payload shape** (C1 review item, resolved). The
   vendored upstream at `13f4fe8` — which includes PR #293 — defines
   `InvalidLastSymbol { offset, symbol, symbol_value }` (3 fields,
   upstream/rust-base64/src/decode.rs:29); the crates.io **0.22.1 release**
   predates #293 and defines `InvalidLastSymbol(usize, u8)` (2 fields). The
   port follows the vendored upstream (3 fields, PORT.md row 3). Therefore:
   the A↔B↔C harness compares **all three fields exactly** (A = vendored
   fork, 3 fields on both sides — `difftest::map_upstream_err`); only the
   separate crates.io cross-check compares the third field leniently
   (`ErrTag::agrees`: value compared only when both sides expose it —
   `cratesio-xcheck::map_cratesio_err` maps to `value: None`). The
   *condition* under which the error fires is identical in all four
   implementations; only the payload grew a field upstream after 0.22.1.
2. **Configurations out of scope.** The comparison is exclusively about the
   strict engines (`PAD` + `RequireCanonical` + reject trailing bits). Upstream
   supports laxer configurations (`Indifferent`, `RequireNone`,
   `decode_allow_trailing_bits`) which by design accept inputs the spec
   rejects; those configurations are v1 non-goals and are not compared.

A failing nightly run uploads the offending inputs as a replay artifact
(`difftest --replay` re-runs them) and is triaged by pairing — A↔B vs B↔C —
to localize the fault before any public claim.
