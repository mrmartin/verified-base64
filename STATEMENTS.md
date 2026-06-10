# STATEMENTS.md — theorem statements for sign-off (checkpoint C1)

> Status: **DRAFT for review**. Extracted-code names (`base64_core.*`) are the
> Aeneas naming convention applied to `rust/base64-core`; they will be
> confirmed verbatim against the generated `lean/Verified/Extracted/` before
> proving starts, without changing the mathematical content. Reviewing these
> statements is the human checkpoint that matters: *a proved wrong statement
> is the failure mode the proofs cannot catch.*

## Conventions

- `Spec.*` is `lean/Verified/Spec.lean` (core-Lean-only RFC 4648 transcription).
- `base64_core.*` is the Aeneas extraction of `rust/base64-core`; its results
  live in the Aeneas monad `Result α = ok α | fail e | div` where `fail`
  covers panics/overflow/OOB and `div` divergence.
- Bridge definitions (in `Verified/Theorems/Defs.lean`):

```lean
inductive Alpha | std | url                      -- quantifies T1–T4 over both alphabets
def Alpha.encT : Alpha → Array U8 64#usize       -- extracted literal tables
def Alpha.decT : Alpha → Array U8 256#usize
def Alpha.spec : Alpha → Spec.Alphabet           -- Spec.std / Spec.url
def Slice.toBytes : Slice U8 → List UInt8        -- extracted slice → spec bytes
def Vec.toBytes : alloc.vec.Vec U8 → List UInt8
def errMap : Spec.Err → base64_core.DecodeError  -- constructor bijection, payloads preserved:
  -- invalidSymbol i b   ↦ InvalidByte i b
  -- invalidLength i     ↦ InvalidLength i
  -- nonCanonical i s v  ↦ InvalidLastSymbol i s v
  -- invalidPadding      ↦ InvalidPadding
```

## Spec-level lemmas (proved first; pure List-induction, no extracted code)

**S1 (spec round-trip)**
```lean
theorem Spec.decode_encode (A : Spec.Alphabet) (b : List UInt8) :
    Spec.decode A (Spec.encode A b) = .ok b
```

**S2 (spec characterization — decode succeeds exactly on encodings)**
```lean
theorem Spec.decode_ok_iff (A : Spec.Alphabet) (s b : List UInt8) :
    Spec.decode A s = .ok b ↔ Spec.encode A b = s
```
S2 is the precise meaning of "strict canonical decoding": the set of accepted
strings is exactly the image of `encode` (hence `Spec.ValidCanonical`), and
the decoded payload is the unique preimage.

## T4 — conformance (the keystone)

Stated as a *functional characterization*: the extracted computation always
terminates with `ok`, and the value it returns is exactly determined by the
spec. This one equation *is* "≃ up to error mapping" made exact — and T3
falls out of it.

**T4-decode**
```lean
theorem t4_decode (a : Alpha) (s : Slice U8) :
    base64_core.decode_alloc s a.decT
      = .ok (match Spec.decode a.spec s.toBytes with
             | .ok bs => core.result.Result.Ok (Vec.ofBytes bs)
             | .error e => core.result.Result.Err (errMap e))
```

**T4-encode** (the `usize`-overflow branch is mirrored exactly; upstream would
panic there, the port returns `Err LengthOverflow` — PORT.md row 4)
```lean
theorem t4_encode (a : Alpha) (input : Slice U8) :
    base64_core.encode_alloc input a.encT
      = .ok (if Spec.encodedLen input.toBytes.length < 2 ^ 64
             then core.result.Result.Ok (Vec.ofBytes (Spec.encode a.spec input.toBytes))
             else core.result.Result.Err base64_core.EncodeError.LengthOverflow)
```
Note: a `Slice` has length < 2⁶⁴ by construction, so the overflow branch of
T4-encode is vacuous for encode-able inputs at the theorem level — it is kept
in the statement so the equation holds for *every* slice, with no hypotheses.

## T3 — totality / no-panic (corollaries of T4, stated separately)

The customer-legible line items: the outer `= .ok …` in T4 already excludes
`fail` (panic, overflow, array-OOB) *and* `div` (non-termination).

```lean
theorem t3_decode_total (a : Alpha) (s : Slice U8) :
    ∃ r, base64_core.decode_alloc s a.decT = .ok r

theorem t3_encode_total (a : Alpha) (input : Slice U8) :
    ∃ r, base64_core.encode_alloc input a.encT = .ok r
```

## T1 — round-trip (via S1 + T4)

```lean
theorem t1_roundtrip (a : Alpha) (b : Slice U8) :
    ∃ e, base64_core.encode_alloc b a.encT = .ok (.Ok e) ∧
         base64_core.decode_alloc e.toSlice a.decT = .ok (.Ok ⟨b.val⟩)
```
No overflow hypothesis is needed: `b : Slice U8` implies `b.len < 2⁶⁴`… but
`encodedLen` can still exceed `usize` for lengths above ~3/4·2⁶⁴. Aeneas
models slice lengths as `< 2⁶⁴`; whether the hypothesis
`Spec.encodedLen b.toBytes.length < 2 ^ 64` is required will be settled
against the extracted model at stub time. **If required, T1 carries exactly
that one hypothesis** (≈ "input shorter than 13.8 exabytes"), stated in
README in one sentence.

## T2 — canonical inverse (via S2 + T4)

```lean
theorem t2_canonical_inverse (a : Alpha) (s : Slice U8)
    (h : Spec.ValidCanonical a.spec s.toBytes) :
    ∃ d, base64_core.decode_alloc s a.decT = .ok (.Ok d) ∧
         base64_core.encode_alloc d.toSlice a.encT = .ok (.Ok ⟨s.val⟩)
```

## What these four together claim (plain English)

For both RFC 4648 alphabets, on **every** input without exception, the
extracted port: never panics and always terminates (T3); returns exactly what
the RFC spec function returns, including which error and its payload (T4);
decode inverts encode (T1); encode inverts decode on exactly the canonical
strings (T2, with S2 pinning "canonical" to "is an encoding").

What they do **not** claim: anything about the SIMD paths, non-strict
configurations, streaming APIs, or timing behavior; and the port↔upstream gap
is closed empirically (PORT.md + difftest A↔B at 10⁸), not by proof.

## Proving order (work items)

| WI | Target | Expected tools |
|---|---|---|
| 01 | table facts: `decT` is the inverse-with-default of `encT`; `encT` lists = `Spec` alphabets | `decide` |
| 02 | S1 | induction on 3-byte blocks + tail cases |
| 03 | S2 | induction + per-case analysis of `decodeSuffix` |
| 04 | per-block bit identities (encode3 vs table lookups) | `bv_decide` / `decide` over ≤ 2¹² cases |
| 05 | `encode_quads` loop spec: after k iterations, `output[0..4k] = Spec` symbols of `input[0..3k]` | loop invariant + `step`/`scalar_tac` |
| 06 | t4_encode | WI-04+05 + padding lemma |
| 07 | `decode_chunk_4` spec vs `Spec.decode4` | `bv_decide` per symbol + composition |
| 08 | quad-loop invariant | induction |
| 09 | `complete_quads_len` characterization | `omega` |
| 10a–e | `decode_suffix` ≡ `Spec.decodeSuffix`, by finite unrolling over suffix length 0–4 | `step` symbolic execution, ~16 paths |
| 11 | t4_decode | WI-07..10 composition |
| 12 | t3_decode_total, t3_encode_total | corollaries of T4 |
| 13 | t1_roundtrip | S1 + T4 |
| 14 | t2_canonical_inverse | S2 + T4 |

---

**Sign-off** (checkpoint C1, blocking — see prover/CLAUDE.md rule 3):

- [ ] Spec.lean read against RFC 4648 (esp. §3.5 strictness, §4 tail cases)
- [ ] SPEC.md precedence quirks Q1–Q6 acknowledged as deliberate
- [ ] T1–T4 + S1/S2 statements above approved as *the* claims to prove
- [ ] Non-claims section approved as honest

Signed: ____________________ Date: ____________
