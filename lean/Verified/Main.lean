import Verified.Spec

/-!
# speccli — the spec executable for the differential harness

Line protocol on stdin/stdout, one operation per line:

    E s <hex>      encode <hex> with the standard alphabet
    E u <hex>      encode <hex> with the URL-safe alphabet
    D s <hex>      decode <hex> with the standard alphabet
    D u <hex>      decode <hex> with the URL-safe alphabet

`<hex>` is lowercase hex with no prefix; the empty byte string is written `-`.

Responses, one line per request, flushed per line:

    OK <hex>                     success (empty result is `OK -`)
    ERR IB <idx> <byte>          Spec.Err.invalidSymbol
    ERR IL <idx>                 Spec.Err.invalidLength
    ERR ILS <idx> <sym> <val>    Spec.Err.nonCanonical
    ERR IP                       Spec.Err.invalidPadding
    ERR MALFORMED <reason>       request line did not parse

This file imports core Lean and `Verified.Spec` only (no mathlib), so the binary starts in
milliseconds and the trusted surface stays the spec itself.
-/

namespace SpecCli

def hexDigit? (c : Char) : Option Nat :=
  if '0' ≤ c ∧ c ≤ '9' then some (c.toNat - 48)
  else if 'a' ≤ c ∧ c ≤ 'f' then some (c.toNat - 87)
  else if 'A' ≤ c ∧ c ≤ 'F' then some (c.toNat - 55)
  else none

def hexToBytes? (s : String) : Option (List UInt8) :=
  if s = "-" then some [] else go s.toList
where
  go : List Char → Option (List UInt8)
    | [] => some []
    | [_] => none
    | h :: l :: rest => do
      let hv ← hexDigit? h
      let lv ← hexDigit? l
      let tail ← go rest
      pure (UInt8.ofNat (hv * 16 + lv) :: tail)

def hexChars : Array Char :=
  #['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f']

def bytesToHex (l : List UInt8) : String :=
  match l with
  | [] => "-"
  | _ =>
    l.foldl (init := "") fun acc b =>
      (acc.push (hexChars[b.toNat / 16]!)).push (hexChars[b.toNat % 16]!)

def errLine : Spec.Err → String
  | .invalidSymbol idx b => s!"ERR IB {idx} {b.toNat}"
  | .invalidLength idx => s!"ERR IL {idx}"
  | .nonCanonical idx sym val => s!"ERR ILS {idx} {sym.toNat} {val.toNat}"
  | .invalidPadding => "ERR IP"

def alphabet? : String → Option Spec.Alphabet
  | "s" => some Spec.std
  | "u" => some Spec.url
  | _ => none

def process (line : String) : String :=
  match line.splitOn " " with
  | [op, al, hex] =>
    match alphabet? al, hexToBytes? hex with
    | some A, some bytes =>
      match op with
      | "E" => s!"OK {bytesToHex (Spec.encode A bytes)}"
      | "D" =>
        match Spec.decode A bytes with
        | .ok decoded => s!"OK {bytesToHex decoded}"
        | .error e => errLine e
      | _ => "ERR MALFORMED bad-op"
    | none, _ => "ERR MALFORMED bad-alphabet"
    | _, none => "ERR MALFORMED bad-hex"
  | _ => "ERR MALFORMED bad-arity"

partial def loop (stdin : IO.FS.Stream) (stdout : IO.FS.Stream) : IO Unit := do
  let line ← stdin.getLine
  if line.isEmpty then
    return ()  -- EOF
  let trimmed := line.trimAscii.toString
  if trimmed.isEmpty then
    loop stdin stdout
  else
    stdout.putStr (process trimmed ++ "\n")
    stdout.flush
    loop stdin stdout

end SpecCli

def main : IO Unit := do
  let stdin ← IO.getStdin
  let stdout ← IO.getStdout
  SpecCli.loop stdin stdout
