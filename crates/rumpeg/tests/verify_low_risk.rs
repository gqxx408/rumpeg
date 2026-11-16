//! Minimal public-API verification for the low-risk fixes.
//!
//! Run: `cargo test -p rumpeg --test verify_low_risk`
//!
//! * Rational::rescale previously cast the i128 quotient with `as i64`,
//!   truncating the high bits and silently wrapping extreme rescales into a
//!   small/garbage timestamp.
//! * The WAV probe gave any RIFF file 25 points, so an AVI with no other
//!   candidate was reported as WAV and then failed to open with a misleading
//!   "not a WAVE file".

use rumpeg::format::{probe, MediaIo};
use rumpeg::prelude::*;

#[test]
fn rational_rescale_saturates_instead_of_wrapping() {
    // Pre-fix: i64::MAX rescaled to a finer time base wrapped to a small value.
    // Post-fix: clamped to i64::MAX / i64::MIN; normal values unaffected.
    let one = Rational::one();
    let micro = Rational::new(1, 1_000_000);
    assert_eq!(one.rescale(i64::MAX, micro), i64::MAX);
    assert_eq!(one.rescale(i64::MIN, micro), i64::MIN);
    assert_eq!(Rational::new(1, 48_000).rescale(48_000, micro), 1_000_000);
}

#[test]
fn wav_probe_rejects_non_wave_riff() {
    // Pre-fix: probe(AVI) returned format_name "wav".
    // Post-fix: only RIFF with a WAVE form type is claimed.
    let avi = b"RIFF\x28\x00\x00\x00AVI LIST";
    let mut io = MediaIo::from_bytes(avi.to_vec());
    if let Ok(r) = probe(&mut io) {
        assert_ne!(r.format_name, "wav", "AVI must not probe as WAV");
    }

    let wav = b"RIFF\x28\x00\x00\x00WAVEfmt ";
    let mut io = MediaIo::from_bytes(wav.to_vec());
    let r = probe(&mut io).expect("valid WAVE must be recognized");
    assert_eq!(r.format_name, "wav");
}
