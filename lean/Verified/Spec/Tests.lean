import Verified.Spec

/-!
# Spec unit tests

RFC 4648 §10 test vectors plus a hand-built negative suite (bad symbols, bad padding placement,
non-canonical finals, embedded pads, every length mod 4). All checks are `#guard`s: they are
evaluated at `lake build` time, so the build itself re-runs the vector suite.

Expected error values (offsets, bytes, 6-bit values) follow the upstream `base64` crate's
`engine::general_purpose::STANDARD` behavior, which the spec's `-- [precedence]` choices
transcribe; the differential harness (rust/difftest) cross-checks them at scale.
-/

namespace Spec.Tests

/-- ASCII string to bytes (test convenience). -/
private def b (s : String) : List UInt8 := Spec.asciiBytes s

private def okOf (r : Except Spec.Err (List UInt8)) (v : List UInt8) : Bool :=
  match r with
  | .ok x => x == v
  | .error _ => false

private def errOf (r : Except Spec.Err (List UInt8)) (e : Spec.Err) : Bool :=
  match r with
  | .ok _ => false
  | .error e' => e' == e

/-! ## RFC 4648 §10 vectors, standard alphabet -/

#guard Spec.encode Spec.std (b "") = b ""
#guard Spec.encode Spec.std (b "f") = b "Zg=="
#guard Spec.encode Spec.std (b "fo") = b "Zm8="
#guard Spec.encode Spec.std (b "foo") = b "Zm9v"
#guard Spec.encode Spec.std (b "foob") = b "Zm9vYg=="
#guard Spec.encode Spec.std (b "fooba") = b "Zm9vYmE="
#guard Spec.encode Spec.std (b "foobar") = b "Zm9vYmFy"

#guard okOf (Spec.decode Spec.std (b "")) (b "")
#guard okOf (Spec.decode Spec.std (b "Zg==")) (b "f")
#guard okOf (Spec.decode Spec.std (b "Zm8=")) (b "fo")
#guard okOf (Spec.decode Spec.std (b "Zm9v")) (b "foo")
#guard okOf (Spec.decode Spec.std (b "Zm9vYg==")) (b "foob")
#guard okOf (Spec.decode Spec.std (b "Zm9vYmE=")) (b "fooba")
#guard okOf (Spec.decode Spec.std (b "Zm9vYmFy")) (b "foobar")

/-! ## URL-safe alphabet (RFC 4648 §5): high values exercise `-` (62) and `_` (63) -/

#guard Spec.encode Spec.url [0xff, 0xff, 0xfe] = b "___-"
#guard Spec.encode Spec.std [0xff, 0xff, 0xfe] = b "///+"
#guard okOf (Spec.decode Spec.url (b "___-")) [0xff, 0xff, 0xfe]
#guard okOf (Spec.decode Spec.url (b "Zm9vYg==")) (b "foob")
-- alphabets are not interchangeable
#guard errOf (Spec.decode Spec.std (b "___-")) (.invalidSymbol 0 95)
#guard errOf (Spec.decode Spec.url (b "///+")) (.invalidSymbol 0 47)

/-! ## Round-trip on a longer input (drives the non-terminal quad path) -/

private def longInput : List UInt8 := (List.range 64).map UInt8.ofNat

#guard okOf (Spec.decode Spec.std (Spec.encode Spec.std longInput)) longInput
#guard okOf (Spec.decode Spec.url (Spec.encode Spec.url longInput)) longInput

/-! ## Negative suite: every length mod 4 -/

-- length ≡ 1 (mod 4): one valid symbol in final quad → invalidLength just past it
#guard errOf (Spec.decode Spec.std (b "Z")) (.invalidLength 1)
#guard errOf (Spec.decode Spec.std (b "ZzzzZ")) (.invalidLength 5)
-- length ≡ 1 (mod 4) with invalid last byte: reported before the left-to-right scan (Q1)
#guard errOf (Spec.decode Spec.std (b "$$$$\n")) (.invalidSymbol 4 10)
-- ... but a valid last byte falls through to the scan, which hits the earlier bad byte
#guard errOf (Spec.decode Spec.std (b "$$$$A")) (.invalidSymbol 0 36)
-- length ≡ 2, 3 (mod 4): valid symbols but missing padding → invalidPadding
#guard errOf (Spec.decode Spec.std (b "Zg")) .invalidPadding
#guard errOf (Spec.decode Spec.std (b "Zm8")) .invalidPadding
#guard errOf (Spec.decode Spec.std (b "Zg=")) .invalidPadding

/-! ## Negative suite: bad symbols -/

#guard errOf (Spec.decode Spec.std (b "$AAA")) (.invalidSymbol 0 36)
#guard errOf (Spec.decode Spec.std (b "AAAA$AAA")) (.invalidSymbol 4 36)
#guard errOf (Spec.decode Spec.std (b "AA\x00A")) (.invalidSymbol 2 0)

/-! ## Negative suite: padding placement -/

-- pad in the first two positions of the final quad
#guard errOf (Spec.decode Spec.std (b "=QQQ")) (.invalidSymbol 0 61)
#guard errOf (Spec.decode Spec.std (b "Q=QQ")) (.invalidSymbol 1 61)
-- symbol after padding: reported at the first pad's offset, as the pad byte
#guard errOf (Spec.decode Spec.std (b "QQ=A")) (.invalidSymbol 2 61)
-- pad inside a non-terminal quad
#guard errOf (Spec.decode Spec.std (b "QQ==QQQQ")) (.invalidSymbol 2 61)
-- all-pad final quad
#guard errOf (Spec.decode Spec.std (b "====")) (.invalidSymbol 0 61)
#guard errOf (Spec.decode Spec.std (b "QQQQ====")) (.invalidSymbol 4 61)

/-! ## Negative suite: non-canonical finals (RFC 4648 §3.5) -/

-- "QQ==" is canonical for 0x41; "QR==" decodes to the same byte but sets trailing bits
#guard okOf (Spec.decode Spec.std (b "QQ==")) [0x41]
#guard errOf (Spec.decode Spec.std (b "QR==")) (.nonCanonical 1 82 17)
#guard okOf (Spec.decode Spec.std (b "QQQ=")) [0x41, 0x04]
#guard errOf (Spec.decode Spec.std (b "QQR=")) (.nonCanonical 2 82 17)
-- a full final quad is always canonical
#guard okOf (Spec.decode Spec.std (b "QQQR")) [0x41, 0x04, 0x11]

/-! ## encodedLen -/

#guard Spec.encodedLen 0 = 0
#guard Spec.encodedLen 1 = 4
#guard Spec.encodedLen 2 = 4
#guard Spec.encodedLen 3 = 4
#guard Spec.encodedLen 4 = 8
#guard (Spec.encode Spec.std longInput).length = Spec.encodedLen longInput.length

end Spec.Tests
