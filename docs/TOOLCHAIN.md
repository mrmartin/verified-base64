# TOOLCHAIN.md — exact pinned versions

Everything that participates in producing or checking the proofs is pinned
here. The translation toolchain (Charon + Aeneas) is part of the trusted
computing base; reproducibility of its output is non-negotiable.

| Component | Version / pin | Role |
|---|---|---|
| upstream `base64` | vendored fork at `upstream/rust-base64`, commit `13f4fe8` (= v0.22.1 + PRs #285, #292, #293) | verification target (difftest side A) |
| crates.io `base64` | `=0.22.1` (locked in rust/cratesio-xcheck) | published-artifact cross-check |
| Rust (project) | stable (workspace builds; difftest/port are version-insensitive) | port + harness |
| Rust (Charon) | `nightly-2026-06-01` + rustc-dev, llvm-tools-preview, rust-src, miri (auto-installed from charon's `rust-toolchain`) | Charon driver |
| Charon | `9dd7f23c8458b2366ce0b5ca7529c5ad4c5fb350` (= Aeneas `charon-pin`) | Rust → LLBC |
| Aeneas | `5138c03bd39e870abe1ad3a572865cf8c15f43d6` (2026-06-08) | LLBC → Lean |
| OCaml | 5.3.0 (opam switch) | builds Aeneas |
| opam | 2.5.1 | OCaml package manager |
| Lean | `leanprover/lean4:v4.30.0-rc2` (committed `lean/lean-toolchain`; this exact version is pinned by the Aeneas Lean backend) | proofs + spec executable |
| mathlib | `v4.30.0-rc2` (transitive dependency of the Aeneas Lean library; locked in `lean/lake-manifest.json`) | proof layer only — the spec (`Verified/Spec.lean`, `Verified/Main.lean`) imports core Lean exclusively |

## Install (Ubuntu, no nix)

```bash
# opam + OCaml switch
sh <(curl -fsSL https://opam.ocaml.org/install.sh)   # installs the opam binary
opam init --bare -n
opam switch create 5.3.0
eval "$(opam env --switch=5.3.0)"

# Aeneas + its pinned Charon (needs libgmp-dev, build-essential)
git clone https://github.com/AeneasVerif/aeneas ~/tools/aeneas
cd ~/tools/aeneas && git checkout 5138c03bd39e870abe1ad3a572865cf8c15f43d6
git clone https://github.com/AeneasVerif/charon ~/tools/charon
cd ~/tools/charon && git checkout "$(tail -1 ~/tools/aeneas/charon-pin)"
ln -sfn ~/tools/charon ~/tools/aeneas/charon
opam install -y ppx_deriving visitors easy_logging zarith yojson core_unix odoc \
  ocamlgraph menhir ocamlformat.0.27.0 unionFind progress domainslib
make -C ~/tools/charon build-charon-rust   # rustup pulls the pinned nightly
make -C ~/tools/charon build-charon-ml
make -C ~/tools/aeneas                     # -> ~/tools/aeneas/bin/aeneas

# Lean (elan reads lean/lean-toolchain automatically)
cd lean && lake build
```

## Regenerating the extracted code

`scripts/extract.sh` is the only sanctioned path from `rust/base64-core` to
`lean/Verified/Extracted/`; the generated files are committed so that
`lake build` is self-contained and diffs are reviewable.
