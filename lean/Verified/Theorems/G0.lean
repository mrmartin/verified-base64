import Verified.Theorems.Defs

/-!
# G0 — toolchain smoke lemmas

The Phase-0 gate from the project plan: trivial-but-real facts proved about
the *extracted* code, demonstrating that the charon → aeneas → lake pipeline
produces definitions the kernel can reason about (monadic plumbing, checked
arithmetic, the `Result` model). These are not part of T1–T4; they only
certify the pipeline.
-/

open Aeneas Aeneas.Std Result

set_option linter.unnecessarySeqFocus false

namespace Verified

/-- The extracted `encoded_len` computes: 3 input bytes encode to 4 symbols,
through the full extracted `Result`/`Option`/checked-arithmetic plumbing. -/
theorem g0_encoded_len_3 : base64_core.encoded_len 3#usize ⦃ o => o = some 4#usize ⦄ := by
  unfold base64_core.encoded_len
  step* <;> simp_all <;> scalar_tac

/-- The padded tail case: 4 input bytes encode to 8 symbols (exercises the
`checked_add` overflow branch). -/
theorem g0_encoded_len_4 : base64_core.encoded_len 4#usize ⦃ o => o = some 8#usize ⦄ := by
  unfold base64_core.encoded_len
  step*
  case _ n _ _ =>
    have h := Usize.checked_add_bv_spec n 4#usize
    cases hc : n.checked_add 4#usize <;> simp_all <;> scalar_tac

/-- The extracted decode-length estimate matches upstream's documented values
(cf. upstream's `estimate_short_lengths` test; exercises an extracted `if`). -/
theorem g0_estimate_5 : base64_core.decoded_len_estimate 5#usize ⦃ n => n = 6#usize ⦄ := by
  unfold base64_core.decoded_len_estimate
  step*
  split <;> rename_i hcond
  · step as ⟨i2, h2⟩
    step as ⟨n, hn⟩
    scalar_tac
  · scalar_tac

end Verified
