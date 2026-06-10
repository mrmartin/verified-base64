/-!
# Executable RFC 4648 base64 specification

This file is the project's ground truth: a direct, naive transcription of RFC 4648 §4 (base64,
standard alphabet) and §5 (base64url), with strict decoding — padding required and canonical
(§3.5: trailing bits beyond the payload must be zero).

Design constraints (see /SPEC.md for the full audit trail):
* **Core Lean only.** No mathlib, no external deps: the trusted reading surface is this file plus
  the Lean kernel.
* **Auditability over speed.** Definitions follow the RFC's own arithmetic ("concatenate 24 bits,
  split into 6-bit groups") using plain `Nat` division/modulo, not bit tricks.
* **Error reporting follows the upstream `base64` crate.** RFC 4648 defines *which* inputs are
  invalid but not *which error to report first* when several apply. For the conformance theorem
  (T4) to be pointwise equality, the error-selection precedence here matches upstream's
  `engine::general_purpose::STANDARD` engine. Each such choice is marked `-- [precedence]` with a
  SPEC.md entry. The *validity set* and *decoded payloads* are pure RFC content, independent of
  these choices.
-/

namespace Spec

/-- The bytes of an ASCII string (every character below 128). For ASCII this is exactly the
UTF-8 encoding; written via `String.data` so that it reduces inside `decide` and `#guard`. -/
def asciiBytes (s : String) : List UInt8 := s.toList.map fun c => UInt8.ofNat c.toNat

/-- RFC 4648 §4: "The 65-character subset of US-ASCII … The extra 65th character, '=', is used to
signify a special processing function." -/
def padByte : UInt8 := 61  -- '='

/-- A base64 alphabet: exactly 64 distinct single-byte symbols, none of which is the pad byte.
RFC 4648 §4 ("The Base 64 Alphabet") and §5 ("The URL and Filename safe Base 64 Alphabet") are the
two instances used in this project. -/
structure Alphabet where
  symbols : List UInt8
  length_eq : symbols.length = 64
  nodup : symbols.Nodup
  noPad : padByte ∉ symbols

/-- RFC 4648 §4, Table 1. Transcribed by writing the table's characters in value order 0..63. -/
def std : Alphabet where
  symbols := asciiBytes "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
  length_eq := by decide
  nodup := by decide
  noPad := by decide

/-- RFC 4648 §5, Table 2: same as Table 1 with `+` → `-` (62) and `/` → `_` (63). -/
def url : Alphabet where
  symbols := asciiBytes "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
  length_eq := by decide
  nodup := by decide
  noPad := by decide

/-- The symbol for a 6-bit value (callers only apply this to `i < 64`; the default is irrelevant
and never reached, but keeps the function total). -/
def Alphabet.sym (A : Alphabet) (i : Nat) : UInt8 := A.symbols.getD i 0

/-- The 6-bit value of a symbol byte, if it is in the alphabet. This is deliberately the naive
reverse lookup (`idxOf?`), not a table: its correctness is immediate from the alphabet itself. -/
def Alphabet.val? (A : Alphabet) (b : UInt8) : Option Nat := A.symbols.idxOf? b

/-! ## Encoding (RFC 4648 §4)

"The encoding process represents 24-bit groups of input bits as output strings of 4 encoded
characters. … each 24-bit input group … is treated as 4 concatenated 6-bit groups, each of which
is translated into a single character in the base 64 alphabet."

The 24-bit group of bytes `b0 b1 b2` is the number `b0·2^16 + b1·2^8 + b2`; its four 6-bit groups
are, from most to least significant, `n/2^18`, `n/2^12 mod 64`, `n/2^6 mod 64`, `n mod 64`. -/

/-- Encode one complete 24-bit group (RFC 4648 §4). -/
def encode3 (A : Alphabet) (b0 b1 b2 : UInt8) : List UInt8 :=
  let n := b0.toNat * 65536 + b1.toNat * 256 + b2.toNat
  [A.sym (n / 262144), A.sym (n / 4096 % 64), A.sym (n / 64 % 64), A.sym (n % 64)]

/-- Encode, with padding (RFC 4648 §4):

"(1) The final quantum of encoding input is an integral multiple of 24 bits; here, the final unit
of encoded output will be an integral multiple of 4 characters with no '=' padding.

(2) The final quantum of encoding input is exactly 8 bits; here, the final unit of encoded output
will be two characters followed by two '=' padding characters." — the 8 bits `b0` are followed by
4 zero bits to form two 6-bit groups: `b0/4` and `(b0 mod 4)·16`.

(3) The final quantum of encoding input is exactly 16 bits; here, the final unit of encoded
output will be three characters followed by one '=' padding character." — the 16 bits are
followed by 2 zero bits: groups `n/2^10`, `n/2^4 mod 64`, `(n mod 16)·4`. -/
def encode (A : Alphabet) : List UInt8 → List UInt8
  | [] => []
  | [b0] =>
    [A.sym (b0.toNat / 4), A.sym (b0.toNat % 4 * 16), padByte, padByte]
  | [b0, b1] =>
    let n := b0.toNat * 256 + b1.toNat
    [A.sym (n / 1024), A.sym (n / 16 % 64), A.sym (n % 16 * 4), padByte]
  | b0 :: b1 :: b2 :: rest => encode3 A b0 b1 b2 ++ encode A rest

/-- Encoded (padded) length: `4·⌈n/3⌉`, written as upstream writes it. RFC 4648 §4: every 3-byte
group yields 4 characters, and a final 1- or 2-byte quantum yields one padded 4-character unit. -/
def encodedLen (n : Nat) : Nat := n / 3 * 4 + (if n % 3 > 0 then 4 else 0)

/-! ## Decoding (strict: RFC 4648 §3.5, §4, with upstream-matching error selection) -/

/-- Decode errors, in 1:1 correspondence with upstream's `DecodeError` (see SPEC.md §Errors):
* `invalidSymbol idx b` — byte `b` at input offset `idx` is outside the alphabet, or is a
  misplaced pad byte (upstream `InvalidByte`);
* `invalidLength idx` — the final quad contains exactly one symbol, which no encoding produces
  (upstream `InvalidLength`);
* `nonCanonical idx sym val` — RFC 4648 §3.5: the final symbol (offset `idx`, byte `sym`, 6-bit
  value `val`) sets trailing bits not used by the payload (upstream `InvalidLastSymbol`);
* `invalidPadding` — the count of pad bytes is not exactly what the symbol count requires
  (upstream `InvalidPadding`). -/
inductive Err where
  | invalidSymbol (idx : Nat) (b : UInt8)
  | invalidLength (idx : Nat)
  | nonCanonical (idx : Nat) (sym : UInt8) (val : UInt8)
  | invalidPadding
  deriving Repr, DecidableEq

/-- Decode one non-terminal 4-symbol group at absolute offset `idx` into 3 bytes (RFC 4648 §4,
the inverse of `encode3`). Every byte must be an alphabet symbol; pad bytes are not in the
alphabet, so a pad in a non-terminal group reports `invalidSymbol` — as upstream does. The
leftmost invalid byte is reported. -- [precedence] -/
def decode4 (A : Alphabet) (idx : Nat) (c0 c1 c2 c3 : UInt8) : Except Err (List UInt8) := do
  let v0 ← match A.val? c0 with | some v => pure v | none => throw (.invalidSymbol idx c0)
  let v1 ← match A.val? c1 with | some v => pure v | none => throw (.invalidSymbol (idx + 1) c1)
  let v2 ← match A.val? c2 with | some v => pure v | none => throw (.invalidSymbol (idx + 2) c2)
  let v3 ← match A.val? c3 with | some v => pure v | none => throw (.invalidSymbol (idx + 3) c3)
  let n := v0 * 262144 + v1 * 4096 + v2 * 64 + v3
  pure [UInt8.ofNat (n / 65536), UInt8.ofNat (n / 256 % 256), UInt8.ofNat (n % 256)]

/-- State accumulated by the left-to-right scan of the final (≤ 4 byte) input group. -/
structure SuffixState where
  /-- 6-bit values of the symbols seen, in order (at most 4). -/
  morsels : List Nat := []
  /-- Number of pad bytes seen. -/
  pads : Nat := 0
  /-- Offset (within the suffix) of the first pad byte; meaningful when `pads > 0`. -/
  firstPadOff : Nat := 0
  /-- Last symbol byte seen and its 6-bit value (for `nonCanonical` reporting). -/
  lastSym : UInt8 := 0
  lastVal : Nat := 0

/-- Scan the final group left to right (transcribes the upstream suffix loop; SPEC.md §Suffix):
* a pad byte in the first two positions of the final quad is `invalidSymbol` there — at most two
  pads can ever be required (RFC 4648 §4 cases (2)/(3)); -- [precedence]
* a symbol after a pad is reported as `invalidSymbol` *at the first pad's offset, with the pad
  byte* — upstream reports this case so; -- [precedence]
* a byte outside the alphabet is `invalidSymbol` at its own offset. -/
def scanSuffix (A : Alphabet) (idx : Nat) : Nat → List UInt8 → SuffixState →
    Except Err SuffixState
  | _, [], st => pure st
  | off, b :: rest, st =>
    if b == padByte then
      if off < 2 then
        throw (.invalidSymbol (idx + off) b)
      else
        scanSuffix A idx (off + 1) rest
          { st with
            pads := st.pads + 1
            firstPadOff := if st.pads == 0 then off else st.firstPadOff }
    else if st.pads > 0 then
      throw (.invalidSymbol (idx + st.firstPadOff) padByte)
    else
      match A.val? b with
      | none => throw (.invalidSymbol (idx + off) b)
      | some v =>
        scanSuffix A idx (off + 1) rest
          { st with morsels := st.morsels ++ [v], lastSym := b, lastVal := v }

/-- Decode the final 0–4 input bytes starting at absolute offset `idx` (`suffix` is empty iff the
whole input is empty). After the scan, in upstream's check order: -- [precedence]
1. a single symbol in the final quad is `invalidLength` (no encoding emits 6 unpadded bits);
2. RFC 4648 §4: counting pads, the final unit must be a whole 4-character quantum
   (`invalidPadding`);
3. RFC 4648 §3.5: "The padding step in base 64 … makes it possible for non-significant bits to be
   present … implementations MUST reject the encoded data if it contains characters outside the
   base alphabet … or if the padding bits have not been set to zero" — `k` symbols carry `6·k`
   bits of which only `8·⌊6k/8⌋` are payload; the rest must be zero (`nonCanonical`).
The payload bytes are the top `⌊6k/8⌋` bytes of the (zero-extended) 24-bit group. -/
def decodeSuffix (A : Alphabet) (idx : Nat) (suffix : List UInt8) : Except Err (List UInt8) := do
  let st ← scanSuffix A idx 0 suffix {}
  let k := st.morsels.length
  if suffix ≠ [] ∧ k < 2 then
    throw (.invalidLength (idx + k))
  if (st.pads + k) % 4 ≠ 0 then
    throw .invalidPadding
  let bytes := k * 6 / 8
  let n := st.morsels.getD 0 0 * 262144 + st.morsels.getD 1 0 * 4096 +
    st.morsels.getD 2 0 * 64 + st.morsels.getD 3 0
  if n % 2 ^ (24 - 8 * bytes) ≠ 0 then
    throw (.nonCanonical (idx + k - 1) st.lastSym (UInt8.ofNat st.lastVal))
  pure ((List.range bytes).map fun j => UInt8.ofNat (n / 2 ^ (16 - 8 * j) % 256))

/-- Decode all non-terminal quads left to right, then the final group. The final quad — even when
it is a complete 4 bytes — is handled by `decodeSuffix`, because it may contain padding: the
pattern below peels a quad only when at least one more byte follows it. -/
def decodeMain (A : Alphabet) (idx : Nat) : List UInt8 → Except Err (List UInt8)
  | c0 :: c1 :: c2 :: c3 :: rest@(_ :: _) => do
    let bytes ← decode4 A idx c0 c1 c2 c3
    let tail ← decodeMain A (idx + 4) rest
    pure (bytes ++ tail)
  | suffix => decodeSuffix A idx suffix

/-- Strict base64 decode (RFC 4648 §4/§5 alphabets, §3.5 canonicity, padding required).

The initial `length % 4 = 1` check transcribes an upstream convenience: a trailing byte that is
neither a pad nor a symbol (e.g. a stray `\n`) is reported *before* the left-to-right scan.
-- [precedence] (SPEC.md §Q1) -/
def decode (A : Alphabet) (s : List UInt8) : Except Err (List UInt8) := do
  if s.length % 4 = 1 then
    match s.getLast? with
    | some b =>
      if b ≠ padByte ∧ (A.val? b).isNone then
        throw (.invalidSymbol (s.length - 1) b)
    | none => pure ()  -- unreachable: length % 4 = 1 implies s ≠ []
  decodeMain A 0 s

/-- A byte string is a valid canonical encoding iff it is the encoding of some byte list. The
spec-level round-trip theorems show `decode` succeeds on exactly this set. -/
def ValidCanonical (A : Alphabet) (s : List UInt8) : Prop := ∃ b, encode A b = s

end Spec
