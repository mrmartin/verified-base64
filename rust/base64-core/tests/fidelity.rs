//! Port fidelity: base64-core's encode/decode agree exactly — payloads, error
//! variants, and error offsets — with the vendored upstream crate configured as
//! `engine::general_purpose::STANDARD` / `URL_SAFE` (the strict RFC 4648 mode
//! this port hardcodes).
//!
//! This is the cheap, always-on fidelity net; the heavy differential harness
//! lives in rust/difftest.

use base64::engine::general_purpose::{STANDARD, URL_SAFE};
use base64::Engine;

type CoreResult = Result<Vec<u8>, base64_core::DecodeError>;

fn map_upstream_err(e: base64::DecodeError) -> base64_core::DecodeError {
    use base64::DecodeError as U;
    use base64_core::DecodeError as C;
    match e {
        U::InvalidByte(o, b) => C::InvalidByte(o, b),
        U::InvalidLength(l) => C::InvalidLength(l),
        U::InvalidLastSymbol {
            offset,
            symbol,
            symbol_value,
        } => C::InvalidLastSymbol(offset, symbol, symbol_value),
        U::InvalidPadding => C::InvalidPadding,
    }
}

struct Pair {
    engine: &'static base64::engine::GeneralPurpose,
    enc: &'static [u8; 64],
    dec: &'static [u8; 256],
}

fn pairs() -> [Pair; 2] {
    [
        Pair {
            engine: &STANDARD,
            enc: &base64_core::STANDARD_ENCODE,
            dec: &base64_core::STANDARD_DECODE,
        },
        Pair {
            engine: &URL_SAFE,
            enc: &base64_core::URL_SAFE_ENCODE,
            dec: &base64_core::URL_SAFE_DECODE,
        },
    ]
}

fn check_encode(p: &Pair, input: &[u8]) {
    let upstream = p.engine.encode(input);
    let core = base64_core::encode_alloc(input, p.enc).expect("no overflow on test sizes");
    assert_eq!(
        upstream.as_bytes(),
        core.as_slice(),
        "encode mismatch on input {input:?}"
    );
}

fn check_decode(p: &Pair, input: &[u8]) {
    let upstream: CoreResult = p.engine.decode(input).map_err(map_upstream_err);
    let core = base64_core::decode_alloc(input, p.dec);
    assert_eq!(upstream, core, "decode mismatch on input {input:?}");
}

#[test]
fn rfc4648_vectors() {
    let vectors: [(&[u8], &str); 7] = [
        (b"", ""),
        (b"f", "Zg=="),
        (b"fo", "Zm8="),
        (b"foo", "Zm9v"),
        (b"foob", "Zm9vYg=="),
        (b"fooba", "Zm9vYmE="),
        (b"foobar", "Zm9vYmFy"),
    ];
    for p in pairs() {
        for (plain, encoded) in vectors {
            check_encode(&p, plain);
            check_decode(&p, encoded.as_bytes());
        }
    }
    // and the canonical RFC examples through the standard pair only
    let p = &pairs()[0];
    for (plain, encoded) in vectors {
        let core = base64_core::encode_alloc(plain, p.enc).unwrap();
        assert_eq!(core, encoded.as_bytes());
    }
}

#[test]
fn encode_all_lengths() {
    for p in pairs() {
        for len in 0..200usize {
            let input: Vec<u8> = (0..len).map(|i| (i * 7 + 13) as u8).collect();
            check_encode(&p, &input);
        }
    }
}

#[test]
fn decode_roundtrip_all_lengths() {
    for p in pairs() {
        for len in 0..200usize {
            let input: Vec<u8> = (0..len).map(|i| (i * 31 + 5) as u8).collect();
            let encoded = p.engine.encode(&input);
            check_decode(&p, encoded.as_bytes());
        }
    }
}

/// Structured adversarial decode inputs: every length mod 4, padding in every
/// position, non-canonical finals, embedded pad, invalid bytes everywhere.
#[test]
fn decode_adversarial() {
    let interesting: &[u8] = b"AQgw=Zz09+/-_\n \0\xff@[`{ABC";
    for p in pairs() {
        // exhaustive over short strings drawn from an interesting alphabet
        for a in interesting {
            check_decode(&p, &[*a]);
            for b in interesting {
                check_decode(&p, &[*a, *b]);
                for c in interesting {
                    check_decode(&p, &[*a, *b, *c]);
                }
            }
        }
        // every padding arrangement on a 8-symbol skeleton
        let skel = b"QUJDREVG"; // "ABCDEF"
        for i in 0..skel.len() {
            for j in i..skel.len() {
                let mut v = skel.to_vec();
                v[i] = b'=';
                v[j] = b'=';
                check_decode(&p, &v);
            }
        }
        // non-canonical finals: QQ.. with every final symbol
        for sym in 0u8..=255 {
            check_decode(&p, &[b'Q', sym, b'=', b'=']);
            check_decode(&p, &[b'Q', b'Q', sym, b'=']);
            check_decode(&p, &[b'Q', b'Q', b'Q', sym]);
        }
    }
}

/// Deterministic pseudo-random sweep (xorshift64*, no deps): random bytes
/// through encode, weighted near-base64 strings through decode.
#[test]
fn random_sweep() {
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545F4914F6CDD1D);
        state
    };

    let decode_charset: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/-_======\n\r @[`{\x00\xff";

    for p in pairs() {
        for _ in 0..20_000 {
            // encode: random bytes, length biased small
            let r = next();
            let len = if r % 4 == 0 {
                (r >> 8) as usize % 9 // 0..=8: tail-case density
            } else {
                (r >> 8) as usize % 256
            };
            let input: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            check_encode(&p, &input);

            // decode round-trip of that input
            let encoded = p.engine.encode(&input);
            check_decode(&p, encoded.as_bytes());

            // decode: weighted near-base64 string
            let r = next();
            let dlen = (r >> 8) as usize % 33;
            let dinput: Vec<u8> = (0..dlen)
                .map(|_| decode_charset[(next() as usize) % decode_charset.len()])
                .collect();
            check_decode(&p, &dinput);

            // single-bit flips of a valid encoding
            if !encoded.is_empty() {
                let mut flipped = encoded.clone().into_bytes();
                let pos = (next() as usize) % flipped.len();
                let bit = 1u8 << ((next() as usize) % 8);
                flipped[pos] ^= bit;
                check_decode(&p, &flipped);
            }
        }
    }
}
