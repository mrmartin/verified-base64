//! Minimal scalar port of the `base64` crate's hot path (vendored at
//! `upstream/rust-base64`, commit 13f4fe8), written in the Rust subset that the
//! Charon/Aeneas Rust→Lean toolchain accepts.
//!
//! Port rules (every deviation from upstream is recorded in /PORT.md):
//! - Strict RFC 4648 mode is hardcoded: encode always pads; decode requires
//!   canonical padding (`DecodePaddingMode::RequireCanonical`) and rejects
//!   non-zero trailing bits (`decode_allow_trailing_bits = false`). This is
//!   exactly the configuration of upstream's `engine::general_purpose::STANDARD`.
//! - The u64 "fast loop" in encode and the 8-symbol unrolled decode chunk are
//!   dropped; they are bit-for-bit equivalent optimizations of the loops kept
//!   here.
//! - Iterators, `copy_from_slice`/`to_be_bytes`, and range-indexed subslices
//!   are rewritten as index-based `while` loops with absolute indices.
//! - Alphabets are passed as table references (`&[u8; 64]` / `&[u8; 256]`);
//!   the tables are literal arrays (checked against upstream in tests/).
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

/// upstream: src/lib.rs `PAD_BYTE`
pub const PAD_BYTE: u8 = 61; // b'='

/// upstream: src/engine/general_purpose/mod.rs `INVALID_VALUE`
pub const INVALID_VALUE: u8 = 255;

// ---------------------------------------------------------------------------
// Lookup tables.
//
// upstream builds these with the `const fn`s `encode_table`/`decode_table`
// (src/engine/general_purpose/mod.rs:196-228) applied to `alphabet::STANDARD`
// and `alphabet::URL_SAFE`. Here they are literal arrays so that they extract
// to Lean as plain array literals; tests/tables.rs proves the literals equal
// upstream's construction.
// ---------------------------------------------------------------------------

pub static STANDARD_ENCODE: [u8; 64] = [
    65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80,
    81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 97, 98, 99, 100, 101, 102,
    103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118,
    119, 120, 121, 122, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 43, 47,
];

pub static STANDARD_DECODE: [u8; 256] = [
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 62, 255, 255, 255, 63,
    52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 255, 255, 255, 255, 255, 255,
    255, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
    15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 255, 255, 255, 255, 255,
    255, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
    41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
];

pub static URL_SAFE_ENCODE: [u8; 64] = [
    65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80,
    81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 97, 98, 99, 100, 101, 102,
    103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118,
    119, 120, 121, 122, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 45, 95,
];

pub static URL_SAFE_DECODE: [u8; 256] = [
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 62, 255, 255,
    52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 255, 255, 255, 255, 255, 255,
    255, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
    15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 255, 255, 255, 255, 63,
    255, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
    41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
];

// ---------------------------------------------------------------------------
// Errors.
//
// upstream: src/decode.rs `DecodeError`. Single unified enum — the
// `DecodeSliceError` wrapper and `OutputSliceTooSmall` are dropped because all
// public entry points here allocate exact-size buffers (PORT.md).
// `InvalidLastSymbol` is a tuple variant `(offset, symbol, symbol_value)`,
// matching the field order of upstream's struct variant (post-PR-#293).
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// An invalid byte was found in the input (absolute offset, byte).
    InvalidByte(usize, u8),
    /// The length of the input, as measured in valid base64 symbols, is invalid.
    InvalidLength(usize),
    /// The last non-padding input symbol's encoded 6 bits have nonzero trailing
    /// bits: (absolute offset, symbol, symbol_value).
    InvalidLastSymbol(usize, u8, u8),
    /// The number of padding characters is not canonical for the input length.
    InvalidPadding,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    /// The encoded length would overflow `usize`.
    LengthOverflow,
}

// ---------------------------------------------------------------------------
// Encode.
// ---------------------------------------------------------------------------

/// Encoded (padded) length for an input of `bytes_len` bytes.
///
/// upstream: src/encode.rs:98-126 `encoded_len`, specialized to `padding = true`.
/// Returns `None` if the encoded length can't be represented in `usize`.
pub fn encoded_len(bytes_len: usize) -> Option<usize> {
    let rem = bytes_len % 3;

    let complete_input_chunks = bytes_len / 3;
    let complete_chunk_output = match complete_input_chunks.checked_mul(4) {
        Some(n) => n,
        None => return None,
    };

    if rem > 0 {
        complete_chunk_output.checked_add(4)
    } else {
        Some(complete_chunk_output)
    }
}

/// Encode complete 3-byte chunks plus the 1- or 2-byte tail, without padding.
/// Returns the number of symbols written.
///
/// upstream: src/engine/general_purpose/mod.rs:51-168 `internal_encode`, with
/// the u64 fast loop (lines 54-126) dropped; this is the remainder loop
/// (137-150) and tail (152-165) with absolute output indexing.
///
/// Precondition: `output.len() >= encoded_len(input.len())` (unpadded part).
pub fn encode_quads(input: &[u8], output: &mut [u8], encode_table: &[u8; 64]) -> usize {
    const LOW_SIX_BITS_U8: u8 = 0x3F;

    let rem = input.len() % 3;
    let start_of_rem = input.len() - rem;

    let mut input_index = 0;
    let mut output_index = 0;

    while input_index < start_of_rem {
        let b0 = input[input_index];
        let b1 = input[input_index + 1];
        let b2 = input[input_index + 2];

        output[output_index] = encode_table[(b0 >> 2) as usize];
        output[output_index + 1] = encode_table[((b0 << 4 | b1 >> 4) & LOW_SIX_BITS_U8) as usize];
        output[output_index + 2] = encode_table[((b1 << 2 | b2 >> 6) & LOW_SIX_BITS_U8) as usize];
        output[output_index + 3] = encode_table[(b2 & LOW_SIX_BITS_U8) as usize];

        input_index += 3;
        output_index += 4;
    }

    if rem == 2 {
        output[output_index] = encode_table[(input[start_of_rem] >> 2) as usize];
        output[output_index + 1] = encode_table
            [((input[start_of_rem] << 4 | input[start_of_rem + 1] >> 4) & LOW_SIX_BITS_U8) as usize];
        output[output_index + 2] =
            encode_table[((input[start_of_rem + 1] << 2) & LOW_SIX_BITS_U8) as usize];
        output_index += 3;
    } else if rem == 1 {
        output[output_index] = encode_table[(input[start_of_rem] >> 2) as usize];
        output[output_index + 1] =
            encode_table[((input[start_of_rem] << 4) & LOW_SIX_BITS_U8) as usize];
        output_index += 2;
    }

    output_index
}

/// Write `=` padding after `unpadded_output_len` symbols. Returns the number of
/// padding bytes written.
///
/// upstream: src/encode.rs:133-143 `add_padding`, with the output subslice
/// replaced by absolute indexing from `unpadded_output_len`.
pub fn add_padding(unpadded_output_len: usize, output: &mut [u8]) -> usize {
    let pad_bytes = (4 - (unpadded_output_len % 4)) % 4;

    let mut i = 0;
    while i < pad_bytes {
        output[unpadded_output_len + i] = PAD_BYTE;
        i += 1;
    }

    pad_bytes
}

/// Encode with padding into an exact-size output slice. Returns bytes written.
///
/// upstream: src/encode.rs:69-90 `encode_with_padding`.
/// Precondition: `output.len() == encoded_len(input.len()).unwrap()`.
pub fn encode_slice(input: &[u8], output: &mut [u8], encode_table: &[u8; 64]) -> usize {
    let b64_bytes_written = encode_quads(input, output, encode_table);
    let padding_bytes = add_padding(b64_bytes_written, output);
    b64_bytes_written + padding_bytes
}

/// Encode to a fresh buffer. The only failure is `usize` overflow of the
/// encoded length (upstream panics here via `.expect()`; we return `Err` so
/// that the no-panic theorem is unconditional — PORT.md).
///
/// upstream: `Engine::encode` (src/engine/mod.rs:115) modulo String/Vec plumbing.
pub fn encode_alloc(input: &[u8], encode_table: &[u8; 64]) -> Result<Vec<u8>, EncodeError> {
    let encoded_size = match encoded_len(input.len()) {
        Some(n) => n,
        None => return Err(EncodeError::LengthOverflow),
    };

    let mut buf: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < encoded_size {
        buf.push(0);
        i += 1;
    }

    let written = encode_slice(input, &mut buf, encode_table);
    // written == encoded_size by construction (upstream debug_asserts this)
    let _ = written;
    Ok(buf)
}

// ---------------------------------------------------------------------------
// Decode.
// ---------------------------------------------------------------------------

/// Conservative upper bound on decoded length.
///
/// upstream: src/engine/general_purpose/decode.rs:14-20 `GeneralPurposeEstimate::new`.
pub fn decoded_len_estimate(encoded_len: usize) -> usize {
    let rem = encoded_len % 4;
    (encoded_len / 4 + if rem > 0 { 1 } else { 0 }) * 3
}

/// Length of the complete, non-terminal quads of `input`. The last quad — even
/// if complete — is excluded, because it may contain padding and is handled by
/// `decode_suffix`.
///
/// upstream: src/engine/general_purpose/decode.rs:131-163 `complete_quads_len`,
/// minus the output-size check (callers here always allocate
/// `decoded_len_estimate` bytes — PORT.md).
fn complete_quads_len(
    input: &[u8],
    input_len_rem: usize,
    decode_table: &[u8; 256],
) -> Result<usize, DecodeError> {
    // detect a trailing invalid byte, like a newline, as a user convenience
    if input_len_rem == 1 {
        let last_byte = input[input.len() - 1];
        // exclude pad bytes; might be part of padding that extends from earlier in the input
        if last_byte != PAD_BYTE && decode_table[last_byte as usize] == INVALID_VALUE {
            return Err(DecodeError::InvalidByte(input.len() - 1, last_byte));
        }
    }

    // skip last quad, even if it's complete, as it may have padding
    // (upstream writes this with two saturating_subs; only the second can
    // actually saturate, on empty input)
    let after_rem = input.len() - input_len_rem;
    let input_complete_nonterminal_quads_len = if input_len_rem == 0 {
        if after_rem >= 4 {
            after_rem - 4
        } else {
            0
        }
    } else {
        after_rem
    };

    Ok(input_complete_nonterminal_quads_len)
}

/// Decode 4 symbols at `input[input_index..input_index+4]` into 3 bytes at
/// `output[output_index..output_index+3]`.
///
/// upstream: src/engine/general_purpose/decode.rs:256-298 `decode_chunk_4`,
/// with `copy_from_slice(&accum.to_be_bytes()[..3])` replaced by per-byte
/// shifts/writes and absolute indexing.
fn decode_chunk_4(
    input: &[u8],
    input_index: usize,
    output: &mut [u8],
    output_index: usize,
    decode_table: &[u8; 256],
) -> Result<(), DecodeError> {
    let b0 = input[input_index];
    let morsel = decode_table[b0 as usize];
    if morsel == INVALID_VALUE {
        return Err(DecodeError::InvalidByte(input_index, b0));
    }
    let mut accum = (morsel as u32) << 26;

    let b1 = input[input_index + 1];
    let morsel = decode_table[b1 as usize];
    if morsel == INVALID_VALUE {
        return Err(DecodeError::InvalidByte(input_index + 1, b1));
    }
    accum |= (morsel as u32) << 20;

    let b2 = input[input_index + 2];
    let morsel = decode_table[b2 as usize];
    if morsel == INVALID_VALUE {
        return Err(DecodeError::InvalidByte(input_index + 2, b2));
    }
    accum |= (morsel as u32) << 14;

    let b3 = input[input_index + 3];
    let morsel = decode_table[b3 as usize];
    if morsel == INVALID_VALUE {
        return Err(DecodeError::InvalidByte(input_index + 3, b3));
    }
    accum |= (morsel as u32) << 8;

    output[output_index] = (accum >> 24) as u8;
    output[output_index + 1] = (accum >> 16) as u8;
    output[output_index + 2] = (accum >> 8) as u8;

    Ok(())
}

/// Decode the last 0-4 bytes, checking padding placement, canonical padding
/// count, and trailing bits. Returns the total number of decoded bytes
/// (including the `output_index` bytes already written).
///
/// upstream: src/engine/general_purpose/decode_suffix.rs:11-165 `decode_suffix`,
/// specialized to `DecodePaddingMode::RequireCanonical` and
/// `decode_allow_trailing_bits = false` (the `STANDARD` engine config), with
/// the iterator loop rewritten as an index loop, the output written via
/// absolute indexing, and `DecodeMetadata` (decoded length + padding offset)
/// reduced to the decoded length (PORT.md).
/// State of the left-to-right scan of the final (≤ 4 byte) input group.
///
/// Aeneas-shape note (PORT.md row 14): upstream scans with a `for` loop whose
/// body exits early via `return`/`continue` and writes `morsels: [u8; 4]` at a
/// dynamic index — three constructs Aeneas's loop translation rejects (the
/// loop fixed-point computation fails). Since the suffix has at most 4 bytes,
/// the scan is unrolled: `suffix_scan_step` is the loop body as a pure state
/// transformer (sticky error instead of early return; four scalars instead of
/// the indexed array), applied up to four times in `decode_suffix`. The
/// resulting state after k steps is identical to upstream's after k
/// iterations.
struct SuffixScan {
    morsels_in_leftover: usize,
    padding_bytes_count: usize,
    /// offset from input_index; meaningful when padding_bytes_count > 0
    first_padding_offset: usize,
    last_symbol: u8,
    last_symbol_value: u8,
    morsel0: u8,
    morsel1: u8,
    morsel2: u8,
    morsel3: u8,
    err: Option<DecodeError>,
}

/// One iteration of upstream's suffix scan loop
/// (upstream: src/engine/general_purpose/decode_suffix.rs:32-88).
fn suffix_scan_step(
    st: SuffixScan,
    input_index: usize,
    leftover_index: usize,
    b: u8,
    decode_table: &[u8; 256],
) -> SuffixScan {
    let mut st = st;
    if st.err.is_some() {
        // a previous step errored: upstream would already have returned
        return st;
    }

    if b == PAD_BYTE {
        // '=' padding. Padding after zero or one non-padding characters in
        // the current quad is invalid (upstream error case #2).
        if leftover_index < 2 {
            let bad_padding_index = input_index + leftover_index;
            st.err = Some(DecodeError::InvalidByte(bad_padding_index, b));
        } else {
            if st.padding_bytes_count == 0 {
                st.first_padding_offset = leftover_index;
            }
            st.padding_bytes_count += 1;
        }
    } else if st.padding_bytes_count > 0 {
        // Non-padding after padding (upstream error case #1): report at the
        // first padding byte's offset.
        st.err = Some(DecodeError::InvalidByte(
            input_index + st.first_padding_offset,
            PAD_BYTE,
        ));
    } else {
        let morsel = decode_table[b as usize];
        if morsel == INVALID_VALUE {
            st.err = Some(DecodeError::InvalidByte(input_index + leftover_index, b));
        } else {
            st.last_symbol = b;
            st.last_symbol_value = morsel;
            if st.morsels_in_leftover == 0 {
                st.morsel0 = morsel;
            } else if st.morsels_in_leftover == 1 {
                st.morsel1 = morsel;
            } else if st.morsels_in_leftover == 2 {
                st.morsel2 = morsel;
            } else {
                st.morsel3 = morsel;
            }
            st.morsels_in_leftover += 1;
        }
    }
    st
}

fn decode_suffix(
    input: &[u8],
    input_index: usize,
    output: &mut [u8],
    output_index_start: usize,
    decode_table: &[u8; 256],
) -> Result<usize, DecodeError> {
    let mut output_index = output_index_start;

    // Decode any leftovers that might not be a complete input chunk of 4
    // bytes. Callers guarantee input.len() - input_index <= 4 (upstream
    // debug_asserts this); the scan is unrolled to four steps (see
    // SuffixScan).
    let mut st = SuffixScan {
        morsels_in_leftover: 0,
        padding_bytes_count: 0,
        first_padding_offset: 0,
        last_symbol: 0,
        last_symbol_value: 0,
        morsel0: 0,
        morsel1: 0,
        morsel2: 0,
        morsel3: 0,
        err: None,
    };
    let len = input.len();
    if input_index < len {
        st = suffix_scan_step(st, input_index, 0, input[input_index], decode_table);
    }
    if input_index + 1 < len {
        st = suffix_scan_step(st, input_index, 1, input[input_index + 1], decode_table);
    }
    if input_index + 2 < len {
        st = suffix_scan_step(st, input_index, 2, input[input_index + 2], decode_table);
    }
    if input_index + 3 < len {
        st = suffix_scan_step(st, input_index, 3, input[input_index + 3], decode_table);
    }

    let morsels_in_leftover = st.morsels_in_leftover;
    let padding_bytes_count = st.padding_bytes_count;
    let last_symbol = st.last_symbol;
    let last_symbol_value = st.last_symbol_value;
    let morsel0 = st.morsel0;
    let morsel1 = st.morsel1;
    let morsel2 = st.morsel2;
    let morsel3 = st.morsel3;

    if let Some(e) = st.err {
        return Err(e);
    }

    // A single trailing symbol (after the above checks) is an invalid length.
    if !input.is_empty() && morsels_in_leftover < 2 {
        return Err(DecodeError::InvalidLength(input_index + morsels_in_leftover));
    }

    // DecodePaddingMode::RequireCanonical (the only mode in this port):
    // allow empty input
    if (padding_bytes_count + morsels_in_leftover) % 4 != 0 {
        return Err(DecodeError::InvalidPadding);
    }

    let leftover_bytes_to_append = morsels_in_leftover * 6 / 8;
    // Put the up to 24 useful bits as the high bits of a u32.
    let mut leftover_num = ((morsel0 as u32) << 26)
        | ((morsel1 as u32) << 20)
        | ((morsel2 as u32) << 14)
        | ((morsel3 as u32) << 8);

    // If there are bits set outside the bits we care about, the last symbol
    // encodes trailing bits that would not be included in the output: reject
    // (decode_allow_trailing_bits = false in this port).
    let mask = !0u32 >> (leftover_bytes_to_append * 8);
    if (leftover_num & mask) != 0 {
        // last morsel is at `morsels_in_leftover` - 1
        return Err(DecodeError::InvalidLastSymbol(
            input_index + morsels_in_leftover - 1,
            last_symbol,
            last_symbol_value,
        ));
    }

    let mut k = 0;
    while k < leftover_bytes_to_append {
        let hi_byte = (leftover_num >> 24) as u8;
        leftover_num <<= 8;
        output[output_index] = hi_byte;
        output_index += 1;
        k += 1;
    }

    Ok(output_index)
}

/// Decode into an exact-size output slice. Returns the number of decoded bytes.
///
/// upstream: src/engine/general_purpose/decode.rs:35-121 `decode_helper`, with
/// the 8-symbol unrolled chunk loop (lines 46-88) dropped — the 4-symbol quad
/// loop is bit-for-bit equivalent — and subslice plumbing replaced by absolute
/// indices.
///
/// Precondition: `output.len() >= decoded_len_estimate(input.len())`.
pub fn decode_slice(
    input: &[u8],
    output: &mut [u8],
    decode_table: &[u8; 256],
) -> Result<usize, DecodeError> {
    let rem = input.len() % 4;
    // (match instead of `?`: the Try-operator desugaring extracts to an
    // axiomatized external — PORT.md row 15)
    let input_complete_nonterminal_quads_len =
        match complete_quads_len(input, rem, decode_table) {
            Ok(n) => n,
            Err(e) => return Err(e),
        };

    // Aeneas-shape note (PORT.md row 15): upstream's loop body propagates the
    // chunk error with `?` (an early function exit inside the loop, which the
    // Aeneas loop translation does not support); the error is carried in
    // `quad_err` instead and returned after the loop — observationally
    // identical, the loop stops at the first error.
    let mut quad_err: Option<DecodeError> = None;
    let mut input_index = 0;
    let mut output_index = 0;
    while quad_err.is_none() && input_index < input_complete_nonterminal_quads_len {
        match decode_chunk_4(input, input_index, output, output_index, decode_table) {
            Ok(()) => {
                input_index += 4;
                output_index += 3;
            }
            Err(e) => {
                quad_err = Some(e);
            }
        }
    }
    if let Some(e) = quad_err {
        return Err(e);
    }

    decode_suffix(input, input_index, output, output_index, decode_table)
}

/// Decode to a fresh buffer.
///
/// upstream: `Engine::decode` (src/engine/mod.rs:244) modulo Vec plumbing
/// (estimate-sized buffer, then shrink to the decoded length; the shrink is a
/// push loop instead of `truncate` — PORT.md).
pub fn decode_alloc(input: &[u8], decode_table: &[u8; 256]) -> Result<Vec<u8>, DecodeError> {
    let estimate = decoded_len_estimate(input.len());

    let mut buf: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < estimate {
        buf.push(0);
        i += 1;
    }

    // (match instead of `?` — see decode_slice)
    let len = match decode_slice(input, &mut buf, decode_table) {
        Ok(n) => n,
        Err(e) => return Err(e),
    };

    let mut out: Vec<u8> = Vec::new();
    let mut j = 0;
    while j < len {
        out.push(buf[j]);
        j += 1;
    }
    Ok(out)
}
