//! Shared pieces of the differential harness: deterministic RNG, input
//! streams, hex codec, and the common outcome/error model.
//!
//! Zero dependencies on purpose: the RNG is a self-contained xoshiro256**
//! (seeded via splitmix64), so streams are bit-reproducible forever from
//! `(seed, index)` regardless of external crate versions.

// ---------------------------------------------------------------------------
// RNG: splitmix64-seeded xoshiro256**
// ---------------------------------------------------------------------------

pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        // splitmix64 to spread the seed into the xoshiro state
        let mut x = seed;
        let mut next = move || {
            x = x.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        };
        Rng {
            s: [next(), next(), next(), next()],
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1]
            .wrapping_mul(5)
            .rotate_left(7)
            .wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }

    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.byte()).collect()
    }
}

// ---------------------------------------------------------------------------
// Hex codec (speccli line protocol: empty byte string is "-")
// ---------------------------------------------------------------------------

pub fn to_hex(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "-".to_string();
    }
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0xF) as u32, 16).unwrap());
    }
    s
}

pub fn from_hex(s: &str) -> Option<Vec<u8>> {
    if s == "-" {
        return Some(Vec::new());
    }
    if s.len() % 2 != 0 {
        return None;
    }
    let chars: Vec<u8> = s.bytes().collect();
    let mut out = Vec::with_capacity(chars.len() / 2);
    for pair in chars.chunks(2) {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Outcome model
// ---------------------------------------------------------------------------

/// Decode error, unified across implementations.
///
/// `value` of `InvalidLastSymbol` is `None` for sources that don't expose the
/// 6-bit symbol value (crates.io base64 0.22.1's tuple variant); two tags
/// `agree` when all common fields match and the values match where both are
/// present.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrTag {
    InvalidByte(usize, u8),
    InvalidLength(usize),
    InvalidLastSymbol {
        offset: usize,
        symbol: u8,
        value: Option<u8>,
    },
    InvalidPadding,
    /// Anything unparseable / out of protocol (always a divergence).
    Malformed(String),
}

impl ErrTag {
    pub fn agrees(&self, other: &ErrTag) -> bool {
        use ErrTag::*;
        match (self, other) {
            (
                InvalidLastSymbol {
                    offset: o1,
                    symbol: s1,
                    value: v1,
                },
                InvalidLastSymbol {
                    offset: o2,
                    symbol: s2,
                    value: v2,
                },
            ) => {
                o1 == o2
                    && s1 == s2
                    && match (v1, v2) {
                        (Some(a), Some(b)) => a == b,
                        _ => true,
                    }
            }
            (a, b) => a == b,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ok(Vec<u8>),
    Err(ErrTag),
}

impl Outcome {
    pub fn agrees(&self, other: &Outcome) -> bool {
        match (self, other) {
            (Outcome::Ok(a), Outcome::Ok(b)) => a == b,
            (Outcome::Err(a), Outcome::Err(b)) => a.agrees(b),
            _ => false,
        }
    }
}

impl From<base64_core::DecodeError> for ErrTag {
    fn from(e: base64_core::DecodeError) -> Self {
        use base64_core::DecodeError as C;
        match e {
            C::InvalidByte(o, b) => ErrTag::InvalidByte(o, b),
            C::InvalidLength(l) => ErrTag::InvalidLength(l),
            C::InvalidLastSymbol(o, s, v) => ErrTag::InvalidLastSymbol {
                offset: o,
                symbol: s,
                value: Some(v),
            },
            C::InvalidPadding => ErrTag::InvalidPadding,
        }
    }
}

/// Parse a speccli response line (`OK <hex>` / `ERR ...`) into an Outcome.
pub fn parse_spec_response(line: &str) -> Outcome {
    let toks: Vec<&str> = line.split_whitespace().collect();
    let malformed = || Outcome::Err(ErrTag::Malformed(line.to_string()));
    match toks.as_slice() {
        ["OK", hex] => match from_hex(hex) {
            Some(bytes) => Outcome::Ok(bytes),
            None => malformed(),
        },
        ["ERR", "IB", idx, byte] => match (idx.parse(), byte.parse()) {
            (Ok(i), Ok(b)) => Outcome::Err(ErrTag::InvalidByte(i, b)),
            _ => malformed(),
        },
        ["ERR", "IL", idx] => match idx.parse() {
            Ok(i) => Outcome::Err(ErrTag::InvalidLength(i)),
            _ => malformed(),
        },
        ["ERR", "ILS", idx, sym, val] => match (idx.parse(), sym.parse(), val.parse()) {
            (Ok(i), Ok(s), Ok(v)) => Outcome::Err(ErrTag::InvalidLastSymbol {
                offset: i,
                symbol: s,
                value: Some(v),
            }),
            _ => malformed(),
        },
        ["ERR", "IP"] => Outcome::Err(ErrTag::InvalidPadding),
        _ => malformed(),
    }
}

// ---------------------------------------------------------------------------
// Input streams (PLAN §Phase-4)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alpha {
    Std,
    Url,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Encode,
    Decode,
}

#[derive(Clone, Debug)]
pub struct Case {
    pub op: Op,
    pub alpha: Alpha,
    pub input: Vec<u8>,
}

const STD_SYMBOLS: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SYMBOLS: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn symbols(alpha: Alpha) -> &'static [u8; 64] {
    match alpha {
        Alpha::Std => STD_SYMBOLS,
        Alpha::Url => URL_SYMBOLS,
    }
}

fn alpha_of(r: u64) -> Alpha {
    if r % 2 == 0 {
        Alpha::Std
    } else {
        Alpha::Url
    }
}

/// Stream 1: random byte arrays through the encode path; lengths 0-4096,
/// biased hard toward 0-8 for tail-case density.
pub fn encode_random(rng: &mut Rng) -> Case {
    let alpha = alpha_of(rng.next_u64());
    let len = match rng.below(10) {
        0..=5 => rng.below(9),        // 60%: 0..=8
        6..=8 => rng.below(64),       // 30%: 0..=63
        _ => rng.below(4097),         // 10%: 0..=4096
    };
    Case {
        op: Op::Encode,
        alpha,
        input: rng.bytes(len),
    }
}

/// Stream 2: random strings over a weighted alphabet through the decode path.
/// Uniform random bytes almost never exercise the interesting decode branches,
/// so weight heavily toward valid symbols, '=', near-misses of the alphabet
/// boundaries ('@' 'Z'+1 '[', '`', 'z'+1 '{', '/'±1, '-', '_'), whitespace,
/// and high bytes.
pub fn decode_weighted(rng: &mut Rng) -> Case {
    let alpha = alpha_of(rng.next_u64());
    let syms = symbols(alpha);
    let len = match rng.below(10) {
        0..=6 => rng.below(13),       // 70%: 0..=12 (suffix logic density)
        7..=8 => rng.below(65),       // 20%: 0..=64
        _ => rng.below(1025),         // 10%: 0..=1024
    };
    let near_miss: &[u8] = b"@[`{.,*0=~ \t\r\n\x00\xff\x80";
    let input = (0..len)
        .map(|_| match rng.below(100) {
            0..=64 => syms[rng.below(64)],          // 65%: valid symbol
            65..=84 => b'=',                        // 20%: padding
            85..=94 => near_miss[rng.below(near_miss.len())], // 10%: near-miss
            _ => rng.byte(),                        // 5%: anything
        })
        .collect();
    Case {
        op: Op::Decode,
        alpha,
        input,
    }
}

/// Stream 3a: the deterministic adversarial corpus — every short length,
/// every padding arrangement on a full quad skeleton, every final-symbol pair
/// (canonical and not), pads in non-terminal quads.
pub fn adversarial_corpus() -> Vec<Case> {
    let mut cases = Vec::new();
    for alpha in [Alpha::Std, Alpha::Url] {
        let syms = symbols(alpha);
        // every length 0..=9 of a valid-symbol run (covers all len mod 4)
        for len in 0..=9usize {
            cases.push(Case {
                op: Op::Decode,
                alpha,
                input: vec![b'A'; len],
            });
        }
        // every (i, j) pad placement on an 8-symbol skeleton
        let skel = b"QUJDREVG";
        for i in 0..8 {
            for j in i..8 {
                let mut v = skel.to_vec();
                v[i] = b'=';
                v[j] = b'=';
                cases.push(Case {
                    op: Op::Decode,
                    alpha,
                    input: v,
                });
            }
        }
        // every final symbol after 1, 2, 3 fixed symbols (canonical + not + invalid)
        for s in 0..=255u8 {
            cases.push(Case {
                op: Op::Decode,
                alpha,
                input: vec![b'Q', s, b'=', b'='],
            });
            cases.push(Case {
                op: Op::Decode,
                alpha,
                input: vec![b'Q', b'Q', s, b'='],
            });
            cases.push(Case {
                op: Op::Decode,
                alpha,
                input: vec![b'Q', b'Q', b'Q', s],
            });
            // and crossing a quad boundary
            cases.push(Case {
                op: Op::Decode,
                alpha,
                input: [b"QQQQ".as_slice(), &[b'Q', s, b'=', b'=']].concat(),
            });
        }
        // every symbol of the alphabet as a lone final quad with full padding
        for i in 0..64 {
            cases.push(Case {
                op: Op::Decode,
                alpha,
                input: vec![syms[i], syms[(i * 7) % 64], b'=', b'='],
            });
        }
    }
    cases
}

/// Stream 3b: a randomized adversarial case — encode a random input validly
/// (pure Rust port used as the generator; any disagreement about validity will
/// surface as a divergence anyway), then flip a single random bit or splice a
/// random byte.
pub fn adversarial_random(rng: &mut Rng) -> Case {
    let alpha = alpha_of(rng.next_u64());
    let n = rng.below(13);
    let raw = rng.bytes(n);
    let table = match alpha {
        Alpha::Std => &base64_core::STANDARD_ENCODE,
        Alpha::Url => &base64_core::URL_SAFE_ENCODE,
    };
    let mut enc = base64_core::encode_alloc(&raw, table).expect("small input");
    if !enc.is_empty() {
        match rng.below(3) {
            0 => {
                // single-bit flip
                let pos = rng.below(enc.len());
                enc[pos] ^= 1 << rng.below(8);
            }
            1 => {
                // overwrite with '='
                let pos = rng.below(enc.len());
                enc[pos] = b'=';
            }
            _ => {
                // truncate
                let keep = rng.below(enc.len());
                enc.truncate(keep);
            }
        }
    }
    Case {
        op: Op::Decode,
        alpha,
        input: enc,
    }
}

/// The mixed stream used by the harness: deterministic per (seed, index).
pub fn case_at(seed: u64, index: u64) -> Case {
    let mut rng = Rng::new(seed ^ index.wrapping_mul(0xA24BAED4963EE407));
    // burn a few outputs to decorrelate near indices
    rng.next_u64();
    rng.next_u64();
    match rng.below(100) {
        0..=39 => encode_random(&mut rng),       // 40%
        40..=79 => decode_weighted(&mut rng),    // 40%
        _ => adversarial_random(&mut rng),       // 20%
    }
}
