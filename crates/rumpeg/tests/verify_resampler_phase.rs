//! Minimal public-API verification for the resampler phase fix.
//!
//! Run: `cargo test -p rumpeg --test verify_resampler_phase`
//!
//! The old formula `out = floor(in/ratio)` kept `out*ratio <= in` true for
//! every frame, so the carried phase was clamped back to 0 each time and
//! `(in - out*ratio)` input samples were silently dropped per frame: long
//! streams progressively shortened and frame boundaries clicked.

use rumpeg::resample::Resampler;

#[test]
fn resampler_preserves_total_duration_across_frames() {
    // Pre-fix: ~0.8 samples lost per 1024-sample frame at 48k->44.1k
    // (200 frames = ~16800 missing samples).
    // Post-fix: out = ceil((in - phase)/ratio), phase carried in [0, ratio);
    // total output is within one sample of in * dst/src over 200 frames.
    for &(src, dst) in &[(48_000u32, 44_100u32), (44_100, 48_000), (48_000, 22_050)] {
        let mut r = Resampler::new(src, dst, 1).unwrap();
        let block = vec![0.5f32; 1024];
        let (mut total_in, mut total_out) = (0usize, 0usize);
        for _ in 0..200 {
            total_out += r.process_f32(&block).unwrap().len();
            total_in += block.len();
        }
        let expected = (total_in as f64 * dst as f64 / src as f64).round() as usize;
        assert!(
            total_out.abs_diff(expected) <= 1,
            "{src}->{dst}: expected ~{expected} samples, got {total_out}"
        );
    }
}

#[test]
fn resampler_same_rate_is_exact_passthrough() {
    let mut r = Resampler::new(48_000, 48_000, 2).unwrap();
    let input = [1.0f32, 2.0, 3.0, 4.0];
    assert_eq!(r.process_f32(&input).unwrap(), input);
}
