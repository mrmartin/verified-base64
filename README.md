# verified-base64

> **Status: work in progress.** This README is a placeholder; the claims below
> describe the project's target state and will be replaced by verified claims
> (with a live CI badge) as they land.

Machine-checked verification of the hot path of [`base64`](https://crates.io/crates/base64)
— the most-downloaded base64 crate on crates.io — against a directly-transcribed
[RFC 4648](https://www.rfc-editor.org/rfc/rfc4648) specification in Lean 4,
via the [Charon](https://github.com/AeneasVerif/charon)/[Aeneas](https://github.com/AeneasVerif/aeneas)
Rust→Lean translation toolchain, continuously re-verified in CI and
differentially fuzzed against the upstream binary.

## What will be proven

- **T1 round-trip**: `decode (encode b) = ok b` for all byte arrays.
- **T2 canonical inverse**: `encode (decode s) = s` for all valid canonical strings.
- **T3 totality / no-panic**: `decode` and `encode` never panic, for *all* inputs.
- **T4 conformance** — two guarantees in one pointwise equality:
  1. **RFC 4648 conformance** on the accept/reject boundary and all decoded
     payloads: the set of accepted strings and the bytes returned are exactly
     the RFC's (strict reading: §4 padding required, §3.5 canonicity enforced).
  2. **Bit-exact agreement with rust-base64's error reporting** on rejection:
     *which* error, at *which* offset, with *which* payload — the RFC has no
     error model, so error identity is upstream-conformance (the drop-in
     guarantee), transcribed in the spec and marked `[precedence]` (SPEC.md
     Q1–Q6).

Across all of T1–T4 there is exactly **one hypothesis**: T1 assumes the
encoded length fits in `usize` (inputs below ~13.8 EB). Everything else is
hypothesis-free, for every input without exception.

## Repository layout

```
upstream/rust-base64/   vendored upstream crate at an exact commit (never modified)
rust/base64-core/       minimal scalar port of the upstream hot path (no_std, zero deps)
rust/difftest/          differential harness: upstream ↔ port ↔ Lean spec executable
lean/                   Lean 4 package: RFC 4648 spec, extracted code, theorems
prover/                 autonomous proving loop: constitution, logs, cost ledger
```

## Trust notes (to be expanded into the full TCB section)

- **`EncodeError::LengthOverflow` is proof-only territory.** Upstream panics
  when the encoded length overflows `usize`; the port returns `Err` there so
  the no-panic theorem is unconditional (PORT.md row 4). No fuzzer can reach
  that branch (it needs a ~13.8 EB input), so this single behavior is the one
  place where port ≡ upstream rests on neither proof nor differential
  testing — only on the documented one-line deviation.
- The error-identity half of T4 (which error/offset/payload fires first on
  multi-defect inputs) is **upstream-conformance, not RFC content** — RFC
  4648 defines validity, not an error model. Every such transcription choice
  is marked in SPEC.md (Q1–Q6).

## Pinned versions

| Component | Pin |
|---|---|
| upstream `base64` | fork of marshallpierce/rust-base64 at `13f4fe8` (= v0.22.1 + #285 + #292 + #293) |
| Lean | `leanprover/lean4:v4.30.0-rc2` |
| Aeneas | `5138c03bd39e870abe1ad3a572865cf8c15f43d6` |
| Charon | `9dd7f23c8458b2366ce0b5ca7529c5ad4c5fb350` |

## License

MIT OR Apache-2.0, same as the upstream crate (see LICENSE-MIT / LICENSE-APACHE).
