# STATEMENTS.md — theorem statements for sign-off (checkpoint C1)

> Status: **READY FOR REVIEW**. All `base64_core.*` names below are confirmed
> verbatim against the committed extraction (`lean/Verified/Extracted/Funs.lean`),
> and the bridge definitions compile (`lean/Verified/Theorems/Defs.lean`).
> Reviewing these statements is the human checkpoint that matters: *a proved
> wrong statement is the failure mode the proofs cannot catch.*

## The model (what the extracted types mean)

Aeneas models Rust values as: `Slice U8` / `alloc.vec.Vec U8` = lists of
bytes with length ≤ `Usize.max`; `U8`/`Usize` = fixed-width bit-vectors with
`.val : Nat`. Every extracted function returns in the Aeneas monad

```
Result α ::= ok (v : α) | fail (e : Error) | div
```

where `fail` covers all panics (asserts, overflow, array out-of-bounds) and
`div` non-termination. The Rust-level `Result` appears inside as
`core.result.Result α ε ::= Ok v | Err e`.

## Bridge definitions (`lean/Verified/Theorems/Defs.lean`, compiled)

```lean
inductive Alpha | std | url            -- the theorems quantify over both alphabets
def Alpha.encT : Alpha → Std.Array Std.U8 64#usize    -- extracted literal tables
def Alpha.decT : Alpha → Std.Array Std.U8 256#usize
def Alpha.spec : Alpha → Spec.Alphabet                -- Spec.std / Spec.url

def sliceBytes (s : Slice Std.U8) : List UInt8        -- s.val.map (UInt8.ofNat ·.val)
def vecBytes (v : alloc.vec.Vec Std.U8) : List UInt8

-- constructor bijection, payloads preserved exactly; total and injective,
-- so the spec error uniquely determines the extracted one
def decodeErrToSpec : base64_core.DecodeError → Spec.Err
  | .InvalidByte i b          => .invalidSymbol i.val (u8ToUInt8 b)
  | .InvalidLength i          => .invalidLength i.val
  | .InvalidLastSymbol i s v  => .nonCanonical i.val (u8ToUInt8 s) (u8ToUInt8 v)
  | .InvalidPadding           => .invalidPadding

-- what T4-decode asserts, case by case (any panic/divergence/mismatch = False):
def DecodeAgrees (r : Result (core.result.Result (alloc.vec.Vec Std.U8) base64_core.DecodeError))
                 (s : Except Spec.Err (List UInt8)) : Prop :=
  match r, s with
  | .ok (.Ok v),  .ok bs     => vecBytes v = bs
  | .ok (.Err e), .error e'  => decodeErrToSpec e = e'
  | _, _ => False

-- what T4-encode asserts (LengthOverflow exactly where upstream would panic):
def EncodeAgrees (r : Result (core.result.Result (alloc.vec.Vec Std.U8) base64_core.EncodeError))
                 (inputLen : Nat) (expected : List UInt8) : Prop :=
  match r with
  | .ok (.Ok v)                  => Spec.encodedLen inputLen ≤ Usize.max ∧ vecBytes v = expected
  | .ok (.Err .LengthOverflow)   => Usize.max < Spec.encodedLen inputLen
  | _ => False
```

## Spec-level lemmas (proved first; pure List induction, no extracted code)

```lean
-- S1 (spec round-trip)
theorem Spec.decode_encode (A : Spec.Alphabet) (b : List UInt8) :
    Spec.decode A (Spec.encode A b) = .ok b

-- S2 (characterization: decode succeeds on exactly the encodings)
theorem Spec.decode_ok_iff (A : Spec.Alphabet) (s b : List UInt8) :
    Spec.decode A s = .ok b ↔ Spec.encode A b = s
```

S2 is the precise meaning of "strict canonical decoding": the accepted set is
exactly the image of `encode` (= `Spec.ValidCanonical`), and the payload is
the unique preimage.

## T4 — conformance (the keystone)

```lean
theorem t4_decode (a : Alpha) (s : Slice Std.U8) :
    DecodeAgrees (base64_core.decode_alloc s a.decT)
                 (Spec.decode a.spec (sliceBytes s))

theorem t4_encode (a : Alpha) (input : Slice Std.U8) :
    EncodeAgrees (base64_core.encode_alloc input a.encT)
                 (sliceBytes input).length
                 (Spec.encode a.spec (sliceBytes input))
```

Unconditional: they hold for *every* slice, no hypotheses. The agreement
predicates make "≃ up to error mapping" exact, and their `False` rows mean T4
already excludes panics and divergence.

## T3 — totality / no-panic (corollaries of T4, stated separately)

```lean
theorem t3_decode_total (a : Alpha) (s : Slice Std.U8) :
    ∃ r, base64_core.decode_alloc s a.decT = .ok r

theorem t3_encode_total (a : Alpha) (input : Slice Std.U8) :
    ∃ r, base64_core.encode_alloc input a.encT = .ok r
```

## T1 — round-trip (via S1 + T4)

```lean
theorem t1_roundtrip (a : Alpha) (b : Slice Std.U8)
    (h : Spec.encodedLen (sliceBytes b).length ≤ Usize.max) :
    ∃ (e : alloc.vec.Vec Std.U8) (d : alloc.vec.Vec Std.U8),
      base64_core.encode_alloc b a.encT = .ok (.Ok e) ∧
      (∀ es : Slice Std.U8, es.val = e.val →
        base64_core.decode_alloc es a.decT = .ok (.Ok d) ∧ d.val = b.val)
```

The hypothesis `h` is the honest `usize` bound (encoded length representable;
fails only for inputs ≥ ~13.8 exabytes — one sentence in README). The inner
quantification re-slices the encoded vector, since decode consumes a slice.

## T2 — canonical inverse (via S2 + T4)

```lean
theorem t2_canonical_inverse (a : Alpha) (s : Slice Std.U8)
    (h : Spec.ValidCanonical a.spec (sliceBytes s)) :
    ∃ (d : alloc.vec.Vec Std.U8),
      base64_core.decode_alloc s a.decT = .ok (.Ok d) ∧
      (∀ ds : Slice Std.U8, ds.val = d.val →
        ∃ e, base64_core.encode_alloc ds a.encT = .ok (.Ok e) ∧ e.val = s.val)
```

## What these claim together (plain English)

For both RFC 4648 alphabets, on **every** input without exception, the
extracted port: never panics and always terminates (T3, and the `False` rows
of T4's predicates); decode inverts encode (T1); encode inverts decode on
exactly the canonical strings (T2, with S2 pinning "canonical" = "is an
encoding"). T4 itself is **two guarantees** (C1 amendment): (a) *RFC 4648
conformance* on the accept/reject boundary and all decoded payloads — the
accepted set and returned bytes are the RFC's, strictly read; and (b)
*bit-exact agreement with rust-base64's error reporting* on rejection —
which error, at which offset, with which payload. The RFC has no error
model; error identity is upstream-conformance (the drop-in-replacement
guarantee), transcribed as SPEC.md Q1–Q6.

**Not claimed**: anything about SIMD paths, non-strict configurations,
streaming APIs, or timing; the port↔upstream gap is closed empirically
(PORT.md + difftest A↔B at 10⁸), not by proof; and the Charon/Aeneas
translation itself is trusted (pinned, mitigated by B↔C difftest at 10⁸).

## Already proved (G0 gate, on master)

`g0_encoded_len_3`, `g0_encoded_len_4`, `g0_estimate_5` — trivial facts about
the extracted code certifying the pipeline; each depends on exactly
`[propext, Classical.choice, Quot.sound]`.

Note for the audit: the Aeneas *library* contains two internal `sorry`s
(`Aeneas/Std/Slice.lean:587`, `Aeneas/Std/StringIter.lean:13`). Any theorem
that (transitively) used them would show `sorryAx` in `#print axioms`; the
per-theorem audit therefore detects contamination mechanically, and the G0
lemmas demonstrate clean dependencies.

## Proving order (work items)

| WI | Target | Expected tools |
|---|---|---|
| 01 | table lemmas: `decT` inverts `encT` (with 255 default); `encT` values = `Spec` alphabet symbols | `decide` |
| 02 | S1 | induction on 3-byte blocks + tail cases |
| 03 | S2 | induction + case analysis of `decodeSuffix` |
| 04 | per-block bit identities (`encode3` vs table lookups) | `bv_decide`/`decide` (≤ 2¹² cases) |
| 05 | `encode_quads_loop` spec: after k iterations, output prefix = spec symbols of input prefix | `step`/`scalar_tac` + invariant |
| 06 | t4_encode | WI-04+05 + `add_padding` lemma |
| 07 | `decode_chunk_4` ≡ `Spec.decode4` | `bv_decide` per symbol |
| 08 | `decode_slice_loop` invariant | induction |
| 09 | `complete_quads_len` characterization | `omega`/`scalar_tac` |
| 10a–e | `decode_suffix` ≡ `Spec.decodeSuffix`: `suffix_scan_step` ≡ `Spec.scanSuffix` step, then ≤ 4 straight-line applications | `step` symbolic execution (the Rust-side unroll already removed the loop) |
| 11 | t4_decode | WI-07..10 composition |
| 12 | t3_decode_total / t3_encode_total | corollaries of T4 |
| 13 | t1_roundtrip | S1 + T4 |
| 14 | t2_canonical_inverse | S2 + T4 |

---

**Sign-off** (checkpoint C1, blocking — see prover/CLAUDE.md rule 3):

- [ ] Spec.lean read against RFC 4648 (esp. §3.5 strictness, §4 tail cases)
- [ ] SPEC.md precedence quirks Q1–Q6 acknowledged as deliberate
- [ ] Bridge definitions (Defs.lean) read: they are part of what a reader must trust
- [ ] S1/S2, T1–T4 statements above approved as *the* claims to prove
- [ ] Non-claims section approved as honest

Signed: ____________________ Date: ____________
