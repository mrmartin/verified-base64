# SPEC.md — formalization choices in `lean/Verified/Spec.lean`

This document records every nontrivial judgment call made while transcribing
RFC 4648 into the executable Lean specification. The spec is the project's
ground truth: theorems are proved *against it*, so an error here would
invalidate them silently. Three independent nets check it: this line-by-line
audit, the RFC §10 vectors + negative suite (`lean/Verified/Spec/Tests.lean`,
re-run on every `lake build` via `#guard`), and the three-way differential
harness (`rust/difftest`, ≥10⁸ inputs against the upstream crate binary).

**Reading surface**: `Spec.lean` imports core Lean only — no mathlib, no Aeneas
library, no project code. A reviewer needs the RFC, that one file, and nothing
else.

**Upstream correspondence**: the spec models the upstream `base64` crate's
`engine::general_purpose::STANDARD` / `URL_SAFE` engines (vendored at
`upstream/rust-base64`, commit `13f4fe8`): encode always pads;
decode requires padding (`DecodePaddingMode::RequireCanonical`) and rejects
non-zero trailing bits (`decode_allow_trailing_bits = false`).

---

## S1. Byte strings as `List UInt8`

**Choice**: all definitions are over `List UInt8`; the CLI converts at the
boundary.

**Why**: structural induction over lists is the cleanest shape for both the
spec-level round-trip proofs and the conformance proof. `UInt8` (not `Nat`,
not `Fin 256`) keeps "byte" meaning byte with zero encoding tricks.

## S2. Alphabets (RFC 4648 §4 Table 1, §5 Table 2)

**Choice**: `Spec.Alphabet` is a structure: a `List UInt8` of the 64 symbols
in value order, with three proof fields — `length = 64`, no duplicates, pad
byte not a member — discharged by `decide` for the two concrete instances.
The symbol lists are written as the ASCII string from the RFC's own table
(`asciiBytes "ABC…+/"`).

**Why**: the reviewer compares one string literal against the RFC table,
character by character. The proof fields are exactly the properties the RFC
asserts in prose ("65-character subset of US-ASCII", distinctness, `=`
reserved for padding); they are also what the round-trip theorems need.

## S3. Reverse lookup is naive search, not a table

**Choice**: `Alphabet.val? b = symbols.idxOf? b` (RFC: "translated into a
single character" — decoding inverts the translation).

**Why**: a 256-entry decode table (as the implementation uses) is an
*optimization* whose correctness should be **proven**, not assumed. The spec
uses the definitionally-obvious inverse; the conformance theorem T4 is what
shows the implementation's table equals it.

## S4. Encoding by `Nat` arithmetic (RFC 4648 §4)

**Choice**: a 24-bit group is the number `b0·2¹⁶ + b1·2⁸ + b2`; the four
6-bit groups are `n/2¹⁸`, `n/2¹² mod 64`, `n/2⁶ mod 64`, `n mod 64`
(`encode3`). Tails per §4 cases (2)/(3): one byte → `b0/4`, `(b0 mod 4)·16`
then `==`; two bytes → `n/2¹⁰`, `n/2⁴ mod 64`, `(n mod 16)·4` then `=`.

**Why**: this is the RFC's own description ("treated as 4 concatenated 6-bit
groups") rendered in the most elementary arithmetic available. No shifts, no
masks, no accumulator state — division and remainder only.

## S5. Decode strictness

**Choice**: decode **rejects** (a) any byte outside the alphabet
(`invalidSymbol`), (b) any misplaced pad and any missing/excess padding
(`invalidSymbol` / `invalidPadding`), (c) a final quad with exactly one
symbol (`invalidLength`), and (d) non-zero trailing bits in the final symbol
(`nonCanonical`).

**Why**: (a)–(c) are RFC 4648 §4 read strictly (a padded encoding is a whole
number of 4-character quanta with 0/1/2 trailing `=`). (d) is RFC 4648 §3.5
verbatim: "implementations MUST reject the encoded data … if the padding bits
have not been set to zero". **This is where real-world implementations
diverge** (e.g. WHATWG forgiving-base64 accepts non-canonical finals), and it
is the deliberate headline choice of this project: the spec models the strict
reading, which is also the upstream `STANDARD` engine's configuration.
Example: `"QQ=="` is the canonical encoding of `0x41`; `"QR=="` carries the
same payload bits plus a non-zero trailing nibble → rejected with
`nonCanonical 1 'R' 17`.

## S6. Error vocabulary

**Choice**: four constructors in 1:1 correspondence with upstream's
`DecodeError`, payloads included:

| `Spec.Err` | upstream | payload |
|---|---|---|
| `invalidSymbol idx b` | `InvalidByte(idx, b)` | absolute offset, offending byte |
| `invalidLength idx` | `InvalidLength(idx)` | offset just past the last symbol |
| `nonCanonical idx sym val` | `InvalidLastSymbol{offset, symbol, symbol_value}` | offset, byte, 6-bit value |
| `invalidPadding` | `InvalidPadding` | — |

**Why**: T4 (conformance) is pointwise equality up to a *constructor
bijection* (`errMap`), with payloads preserved exactly. A coarser spec error
type would weaken T4 precisely where customers ask the most questions ("what
does it return on bad input?").

## S7. Error-selection precedence (the `-- [precedence]` marks)

RFC 4648 defines which inputs are invalid but **not** which error an input
that is invalid in several ways should report. For T4 to be an equality, the
spec transcribes upstream's selection order. These are
**implementation-conformance choices, not RFC content** — the set of valid
inputs and all decoded payloads are unaffected. Each mark in `Spec.lean`:

* **Q1 — trailing-byte pre-check** (`Spec.decode`): when `|s| mod 4 = 1` and
  the *last* byte is neither pad nor symbol, it is reported before the
  left-to-right scan. Upstream: `complete_quads_len`, decode.rs:139-146 (a
  convenience for trailing `\n`). Consequence: `"$$$$\n"` reports offset 4,
  not 0. Without this clause the spec would report `invalidSymbol 0 '$'`.
* **Q2 — left-to-right scan**: within quads and within the suffix, the
  leftmost offending byte wins (`decode4`, `scanSuffix`). Upstream: loop
  order.
* **Q3 — symbol after padding** (`scanSuffix`): reported at the *first pad's
  offset* with the *pad byte* as payload, not at the symbol. Upstream:
  decode_suffix.rs:70-74. Example: `"QQ=A"` → `invalidSymbol 2 '='`.
* **Q4 — pad in the first two positions** of the final quad is
  `invalidSymbol` at its own offset (at most two pads can ever be required —
  RFC §4 cases (2)/(3) — so a pad at position 0/1 can never begin a valid
  final quad). Upstream: decode_suffix.rs:47-56.
* **Q5 — post-scan check order** (`decodeSuffix`): `invalidLength` (single
  symbol) → `invalidPadding` (quantum incomplete) → `nonCanonical` (trailing
  bits). Upstream: decode_suffix.rs:92-144. Consequence: `"Zg="` is
  `invalidPadding` (not `nonCanonical` — the padding check fires first);
  `"Z"` is `invalidLength 1` (not `invalidPadding`).
* **Q6 — the terminal quad is processed by the suffix logic even when
  complete** (`decodeMain`'s `rest@(_ :: _)` pattern): a pad may only appear
  in the last four bytes. Upstream: `complete_quads_len` excludes the last
  quad from the main loop.

## S8. Empty input decodes to empty output

`decode A [] = ok []` (suffix scan of `[]` with zero pads and zero morsels
passes all checks). Upstream agrees. RFC: zero 24-bit groups.

## S9. `encodedLen`

`encodedLen n = n/3·4 + (4 if n mod 3 > 0 else 0)` — RFC §4's output length,
written in the same shape as upstream's `encoded_len` so that the overflow
hypothesis of theorem T1 ("the encoded length fits in `usize`") reads
identically on both sides. The spec itself is over `Nat` and never overflows;
the bound appears only in theorem statements about the extracted code.

## S10. Out of scope (v1 non-goals, mirrored from /PLAN_verified-base64.md)

Unpadded variants (`NO_PAD` configs, `DecodePaddingMode::{Indifferent,
RequireNone}`), forgiving decoding (`decode_allow_trailing_bits = true`),
MIME/PEM line wrapping (RFC 2045), base32/base16 (RFC 4648 §6–§8), streaming
encoders, and alphabet validation (`Alphabet::new`'s checks are *assumed* via
the structure's proof fields, which `decide` verifies for the two instances
used).
