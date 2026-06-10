import Verified.Spec

/-!
# Spec-level lemmas (S1, S2)

Pure `List` reasoning about the specification itself; no extracted code.
Statements signed off at checkpoint C1 (STATEMENTS.md) — do not modify
(prover/CLAUDE.md rule 3).
-/

namespace Spec

/-- S1 (spec round-trip): decoding any encoding succeeds and returns the
original bytes. -/
theorem decode_encode (A : Spec.Alphabet) (b : List UInt8) :
    Spec.decode A (Spec.encode A b) = .ok b := by
  sorry

/-- S2 (characterization): decode succeeds on exactly the encodings, with the
unique preimage as payload — the precise content of "strict canonical
decoding". -/
theorem decode_ok_iff (A : Spec.Alphabet) (s b : List UInt8) :
    Spec.decode A s = .ok b ↔ Spec.encode A b = s := by
  sorry

end Spec
