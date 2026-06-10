# PORT.md — base64-core ↔ upstream correspondence

`rust/base64-core` is a line-by-line port of the scalar (non-SIMD) hot path of
the upstream `base64` crate, vendored at `upstream/rust-base64` (commit
`13f4fe8`, = v0.22.1 + #285 + #292 + #293). The port is written in the Rust
subset accepted by the Charon/Aeneas Rust→Lean toolchain.

**Configuration**: the port hardcodes the strict RFC 4648 mode — exactly
upstream's `engine::general_purpose::STANDARD` / `URL_SAFE` engines:
`encode_padding = true`, `decode_padding_mode = RequireCanonical`,
`decode_allow_trailing_bits = false`.

**Fidelity nets**: `tests/tables.rs` (literal tables == upstream's table
construction), `tests/fidelity.rs` (payload- and error-exact agreement with the
vendored upstream engines on RFC vectors, exhaustive short adversarial inputs,
every padding arrangement, every non-canonical final symbol, bit-flips, and a
deterministic random sweep), plus the heavy three-way harness in
`rust/difftest`.

## Correspondence table

| # | base64-core | Upstream source | Deviation & reason |
|---|---|---|---|
| 1 | `PAD_BYTE`, `INVALID_VALUE` | src/lib.rs:305; engine/general_purpose/mod.rs:17 | None (values copied). |
| 2 | `STANDARD_ENCODE/DECODE`, `URL_SAFE_ENCODE/DECODE` | engine/general_purpose/mod.rs:196-228 (`encode_table`/`decode_table` const fns over `alphabet::{STANDARD,URL_SAFE}`) | Tables are **literal arrays** instead of const-fn loops, so they extract to Lean as plain literals (`decide`-friendly). Equality with upstream's construction is asserted in tests/tables.rs. |
| 3 | `DecodeError` | src/decode.rs:10-44 | Single unified enum. `InvalidLastSymbol` is a tuple variant `(offset, symbol, symbol_value)` with the same field order/meaning as upstream's struct variant. `DecodeSliceError`/`OutputSliceTooSmall` dropped: all public entry points allocate exact-size buffers, making that error unreachable (upstream's Vec-based APIs `unreachable!()` on it too, engine/mod.rs:256-258). No `Debug` derive: `derive(Debug)` drags `core::fmt` machinery into the Lean extraction as axiomatized externals; tests render errors manually. |
| 4 | `EncodeError::LengthOverflow` | — (upstream panics via `.expect()` at encode.rs:85-87 / engine/mod.rs String sizing) | **Behavioral deviation, error-path only**: on `usize` overflow of the encoded length (inputs ≥ ~3/4·2⁶⁴ bytes — unreachable in practice) the port returns `Err` instead of panicking, so the no-panic theorem T3-encode is unconditional. |
| 5 | `encoded_len` | src/encode.rs:98-126 | Specialized to `padding = true`; `?`-less match kept as upstream. |
| 6 | `encode_quads` | engine/general_purpose/mod.rs:51-168 `internal_encode` | u64 fast loop (lines 54-126) **dropped** — it is a hand-unrolled, bit-identical optimization of the 3→4 loop kept here (same table, same bit extraction; covered by fidelity tests). Range-indexed sub-slices (`&input[a..b]`, `&mut output[a..b]`) replaced by absolute indexing. |
| 7 | `add_padding` | src/encode.rs:133-143 | Output subslice parameter replaced by absolute indexing from `unpadded_output_len`. |
| 8 | `encode_slice` | src/encode.rs:69-90 `encode_with_padding` | `debug_assert`s dropped; engine/config indirection removed (padding always on). |
| 9 | `encode_alloc` | engine/mod.rs:115 `Engine::encode` | Returns `Vec<u8>` instead of `String` (the bytes are identical; upstream wraps in `String::from_utf8` for ergonomics). Buffer built by a push-zero `while` loop instead of `vec![0; n]` (macro/iterator-free for Aeneas). |
| 10 | `decoded_len_estimate` | engine/general_purpose/decode.rs:14-20 `GeneralPurposeEstimate::new` | None (formula identical). |
| 11 | `complete_quads_len` | engine/general_purpose/decode.rs:131-163 | Output-size check (`OutputSliceTooSmall`, lines 158-161) dropped (see #3). The two `saturating_sub`s written as explicit branches (only the second can saturate, on empty input). `debug_assert`s dropped. |
| 12 | `decode_chunk_4` | engine/general_purpose/decode.rs:256-298 | `copy_from_slice(&accum.to_be_bytes()[..3])` replaced by three per-byte shift/cast writes (same bytes by construction). Absolute indexing instead of sub-slices. |
| 13 | — (`decode_chunk_8`) | engine/general_purpose/decode.rs:174-252 | **Dropped** — 8-symbol unroll is two fused `decode_chunk_4`s over the same table with identical error offsets (covered by fidelity tests). |
| 14 | `decode_suffix` + `SuffixScan`/`suffix_scan_step` | engine/general_purpose/decode_suffix.rs:11-165 | Specialized to `RequireCanonical` + `decode_allow_trailing_bits = false` (the other two `DecodePaddingMode` arms and the trailing-bits bypass deleted). **Scan loop unrolled**: upstream's `for` loop exits early via `return`/`continue` and writes `morsels: [u8; 4]` at a dynamic index — constructs Aeneas's loop translation rejects (loop fixed-point failure). The suffix has ≤ 4 bytes (callers guarantee it; upstream `debug_assert`s it), so the loop body became `suffix_scan_step`, a pure state transformer with a sticky error field and four morsel scalars, applied up to 4 times. State after k steps is identical to upstream's after k iterations. Output written by absolute index instead of `get_mut().ok_or(...)?` (see #3). `DecodeMetadata` return reduced to the decoded length — the padding offset is internal plumbing not exposed by upstream's public `decode`. |
| 15 | `decode_slice` | engine/general_purpose/decode.rs:35-121 `decode_helper` | 8-symbol unrolled chunk loop (lines 46-88) dropped (see #13); the 4-symbol quad loop is kept as the single main loop. `chunks_exact().enumerate()` rewritten as an index `while` loop; sub-slice plumbing replaced by absolute indices. The loop body's `?` (early function exit inside a loop — rejected by Aeneas) carried in a `quad_err` variable and returned after the loop. The remaining `?` uses outside loops became `match` (the `Try`-operator desugaring extracts as an axiomatized external). |
| 16 | `decode_alloc` | engine/mod.rs:244 `Engine::decode` / decode_vec | Estimate-sized buffer then shrink-to-fit, as upstream; the shrink is a fresh-Vec push loop instead of `Vec::truncate` (Aeneas-friendly). |

## What is *not* ported (upstream surface that is out of scope)

SIMD/AVX engines, the `chunked_encoder`/streaming readers-writers, alphabet
construction/validation (`Alphabet::new`), all non-strict configurations
(`NO_PAD`, `Indifferent`, `RequireNone`, `decode_allow_trailing_bits = true`),
`encode_string`/`encode_slice` public wrappers, and the deprecated free
functions. These are explicit non-goals of v1 (see README).
