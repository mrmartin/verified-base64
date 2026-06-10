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
- **T4 RFC conformance**: the extracted implementation equals the executable
  RFC 4648 spec function, pointwise.

## Repository layout

```
upstream/rust-base64/   vendored upstream crate at an exact commit (never modified)
rust/base64-core/       minimal scalar port of the upstream hot path (no_std, zero deps)
rust/difftest/          differential harness: upstream ↔ port ↔ Lean spec executable
lean/                   Lean 4 package: RFC 4648 spec, extracted code, theorems
prover/                 autonomous proving loop: constitution, logs, cost ledger
```

## Pinned versions

| Component | Pin |
|---|---|
| upstream `base64` | fork of marshallpierce/rust-base64 at `13f4fe8` (= v0.22.1 + #285 + #292 + #293) |
| Lean | `leanprover/lean4:v4.30.0-rc2` |
| Aeneas | `5138c03bd39e870abe1ad3a572865cf8c15f43d6` |
| Charon | `9dd7f23c8458b2366ce0b5ca7529c5ad4c5fb350` |

## License

MIT OR Apache-2.0, same as the upstream crate (see LICENSE-MIT / LICENSE-APACHE).
