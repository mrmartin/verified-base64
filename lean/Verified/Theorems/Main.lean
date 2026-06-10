import Verified.Theorems.Defs
import Verified.Theorems.SpecLemmas

/-!
# T1–T4 — the four core theorems

Statements signed off at checkpoint C1 (STATEMENTS.md) — do not modify
(prover/CLAUDE.md rule 3). Helper lemmas live in the other
`Verified/Theorems/` modules; these are the customer-facing claims.
-/

open Aeneas Aeneas.Std

namespace Verified

/-- T4-decode (keystone): on every input slice, the extracted decoder
terminates without panic and agrees pointwise with the RFC 4648 spec —
same bytes on success, same error with the same payload on failure. -/
theorem t4_decode (a : Alpha) (s : Slice Std.U8) :
    DecodeAgrees (base64_core.decode_alloc s a.decT)
      (Spec.decode a.spec (sliceBytes s)) := by
  sorry

/-- T4-encode (keystone): on every input slice, the extracted encoder
terminates without panic and returns the spec's encoding whenever the encoded
length fits in `usize`, `LengthOverflow` otherwise. -/
theorem t4_encode (a : Alpha) (input : Slice Std.U8) :
    EncodeAgrees (base64_core.encode_alloc input a.encT)
      (sliceBytes input).length (Spec.encode a.spec (sliceBytes input)) := by
  sorry

/-- T3: decode is total — it never panics, never overflows, never reads out
of bounds, and always terminates, on every input. -/
theorem t3_decode_total (a : Alpha) (s : Slice Std.U8) :
    ∃ r, base64_core.decode_alloc s a.decT = .ok r := by
  sorry

/-- T3: encode is total — `usize` overflow of the output length is an `Err`
value, not a panic. -/
theorem t3_encode_total (a : Alpha) (input : Slice Std.U8) :
    ∃ r, base64_core.encode_alloc input a.encT = .ok r := by
  sorry

/-- T1 (round-trip): decoding an encoding returns the original bytes. The
hypothesis is the honest `usize` bound on the encoded length (fails only for
inputs ≥ ~13.8 EB); the inner quantifier re-slices the encoded vector, since
decode consumes a slice. -/
theorem t1_roundtrip (a : Alpha) (b : Slice Std.U8)
    (h : Spec.encodedLen (sliceBytes b).length ≤ Usize.max) :
    ∃ (e d : alloc.vec.Vec Std.U8),
      base64_core.encode_alloc b a.encT = .ok (.Ok e) ∧
      (∀ es : Slice Std.U8, es.val = e.val →
        base64_core.decode_alloc es a.decT = .ok (.Ok d) ∧ d.val = b.val) := by
  sorry

/-- T2 (canonical inverse): on every valid canonical string — exactly the
image of `Spec.encode`, by S2 — decode succeeds and encode maps the result
back to the original string. -/
theorem t2_canonical_inverse (a : Alpha) (s : Slice Std.U8)
    (h : Spec.ValidCanonical a.spec (sliceBytes s)) :
    ∃ (d : alloc.vec.Vec Std.U8),
      base64_core.decode_alloc s a.decT = .ok (.Ok d) ∧
      (∀ ds : Slice Std.U8, ds.val = d.val →
        ∃ e, base64_core.encode_alloc ds a.encT = .ok (.Ok e) ∧ e.val = s.val) := by
  sorry

end Verified
