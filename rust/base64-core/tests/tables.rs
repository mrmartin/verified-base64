//! Table fidelity: the literal tables in base64-core equal the tables upstream
//! builds from its alphabets with `encode_table`/`decode_table`
//! (upstream/rust-base64/src/engine/general_purpose/mod.rs:196-228).
//!
//! Upstream's table-construction fns are pub(crate), so we re-derive them here
//! from the public alphabet strings, mirroring the upstream const fns exactly.

fn build_encode_table(symbols: &[u8]) -> [u8; 64] {
    assert_eq!(symbols.len(), 64);
    let mut t = [0u8; 64];
    let mut i = 0;
    while i < 64 {
        t[i] = symbols[i];
        i += 1;
    }
    t
}

fn build_decode_table(symbols: &[u8]) -> [u8; 256] {
    assert_eq!(symbols.len(), 64);
    let mut t = [base64_core::INVALID_VALUE; 256];
    let mut i = 0;
    while i < 64 {
        t[symbols[i] as usize] = i as u8;
        i += 1;
    }
    t
}

#[test]
fn standard_tables_match_upstream_alphabet() {
    let symbols = base64::alphabet::STANDARD.as_str().as_bytes();
    assert_eq!(build_encode_table(symbols), base64_core::STANDARD_ENCODE);
    assert_eq!(build_decode_table(symbols), base64_core::STANDARD_DECODE);
}

#[test]
fn url_safe_tables_match_upstream_alphabet() {
    let symbols = base64::alphabet::URL_SAFE.as_str().as_bytes();
    assert_eq!(build_encode_table(symbols), base64_core::URL_SAFE_ENCODE);
    assert_eq!(build_decode_table(symbols), base64_core::URL_SAFE_DECODE);
}

#[test]
fn pad_byte_matches() {
    // upstream PAD_BYTE is pub(crate); '=' is fixed by RFC 4648
    assert_eq!(base64_core::PAD_BYTE, b'=');
}
