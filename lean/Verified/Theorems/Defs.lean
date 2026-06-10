import Verified.Spec
import Verified.Extracted.Funs

/-!
# Bridge definitions: extracted model ↔ specification

Everything needed to *state* the theorems (STATEMENTS.md). These definitions
are part of the trusted statement-reading surface, so they are kept small and
direct: value-level conversions from the Aeneas model (`Slice`/`Vec` are
length-bounded lists, `U8`/`Usize` are bit-vectors) to the spec's
`List UInt8`/`Nat`, and the agreement predicates the theorems assert.
-/

open Aeneas Aeneas.Std

namespace Verified

/-- The two RFC 4648 alphabets the theorems quantify over. -/
inductive Alpha where
  | std
  | url
  deriving Repr, DecidableEq

/-- The extracted encode table for an alphabet (a literal 64-entry array). -/
def Alpha.encT : Alpha → Std.Array Std.U8 64#usize
  | .std => base64_core.STANDARD_ENCODE
  | .url => base64_core.URL_SAFE_ENCODE

/-- The extracted decode table for an alphabet (a literal 256-entry array). -/
def Alpha.decT : Alpha → Std.Array Std.U8 256#usize
  | .std => base64_core.STANDARD_DECODE
  | .url => base64_core.URL_SAFE_DECODE

/-- The specification alphabet. -/
def Alpha.spec : Alpha → Spec.Alphabet
  | .std => Spec.std
  | .url => Spec.url

/-- Aeneas `U8` (an 8-bit bit-vector) to the spec's `UInt8` — exact, since
`x.val < 256`. -/
def u8ToUInt8 (x : Std.U8) : UInt8 := UInt8.ofNat x.val

/-- The bytes of an extracted slice, as the spec reads them. -/
def sliceBytes (s : Slice Std.U8) : List UInt8 := s.val.map u8ToUInt8

/-- The bytes of an extracted vector, as the spec reads them. -/
def vecBytes (v : alloc.vec.Vec Std.U8) : List UInt8 := v.val.map u8ToUInt8

/-- Constructor bijection from the extracted error to the spec error, payloads
preserved (`Usize.val`/`U8.val` are exact). This direction is total; it is
injective, so the spec error uniquely determines the extracted one. -/
def decodeErrToSpec : base64_core.DecodeError → Spec.Err
  | .InvalidByte i b => .invalidSymbol i.val (u8ToUInt8 b)
  | .InvalidLength i => .invalidLength i.val
  | .InvalidLastSymbol i s v => .nonCanonical i.val (u8ToUInt8 s) (u8ToUInt8 v)
  | .InvalidPadding => .invalidPadding

/-- What T4-decode asserts, case by case:
the extracted computation must terminate without panic (`Aeneas .ok`), and
return `Ok` exactly when the spec decodes, with equal bytes — or the matching
error. Any other combination (panic, divergence, ok-vs-error mismatch) is
`False`. -/
def DecodeAgrees
    (r : Result (core.result.Result (alloc.vec.Vec Std.U8) base64_core.DecodeError))
    (s : Except Spec.Err (List UInt8)) : Prop :=
  match r, s with
  | .ok (.Ok v), .ok bs => vecBytes v = bs
  | .ok (.Err e), .error e' => decodeErrToSpec e = e'
  | _, _ => False

/-- Companion to T1/T2's inner re-slicing (C1 review note 1): the slice those
statements quantify over always exists — `Vec` and `Slice` are the same
length-bounded-list subtype, so any vector's contents re-slice directly. The
`∀ es, es.val = e.val → …` form is therefore never vacuous. -/
theorem exists_slice_of_vec (v : alloc.vec.Vec Std.U8) :
    ∃ es : Slice Std.U8, es.val = v.val :=
  ⟨⟨v.val, v.property⟩, rfl⟩

/-- What T4-encode asserts: the extracted computation terminates without
panic, returning the spec's encoding whenever the encoded length fits in
`usize`, and `LengthOverflow` otherwise (where upstream would panic —
PORT.md row 4). -/
def EncodeAgrees
    (r : Result (core.result.Result (alloc.vec.Vec Std.U8) base64_core.EncodeError))
    (inputLen : Nat) (expected : List UInt8) : Prop :=
  match r with
  | .ok (.Ok v) => Spec.encodedLen inputLen ≤ Usize.max ∧ vecBytes v = expected
  | .ok (.Err .LengthOverflow) => Usize.max < Spec.encodedLen inputLen
  | _ => False

end Verified
