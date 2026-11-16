//! Minimal public-API verification for the Opus tail-frame fix.
//!
//! Run: `cargo test -p rumpeg --test verify_opus_tail`
//!
//! The old encoder zero-padded the tail of every `send_frame` to a full 20 ms
//! frame and encoded it — inserting silence mid-stream when input frames were
//! not frame-size aligned — and `send_eof` dropped the residual samples
//! entirely.

use rumpeg::codec::EncoderContext;
use rumpeg::prelude::*;

fn opus_encoder() -> EncoderContext {
    let params = CodecParams::audio_codec(
        CodecId::Opus,
        AudioParams {
            sample_fmt: SampleFormat::S16,
            sample_rate: 48_000,
            layout: ChannelLayout::MONO,
            frame_size: 0,
        },
    );
    EncoderContext::open(&params).unwrap()
}

fn s16_samples(n: usize) -> Frame {
    let mut f =
        AudioFrame::alloc_interleaved(SampleFormat::S16, 48_000, ChannelLayout::MONO, n as u32);
    for (i, c) in f.data_mut().chunks_exact_mut(2).enumerate() {
        c.copy_from_slice(&(((i % 90) as f32 * 180.0) as i16).to_le_bytes());
    }
    Frame::Audio(f)
}

#[test]
fn partial_frames_accumulate_without_midstream_silence() {
    // Pre-fix: each 480-sample call was padded to 960 and emitted a packet
    // (a silent gap every call). Post-fix: samples accumulate; a packet only
    // appears once a full frame is available.
    let mut enc = opus_encoder();
    enc.send_frame(&s16_samples(480)).unwrap();
    assert!(matches!(
        enc.receive_packet(),
        Err(Error::NeedMoreData) | Err(Error::Eof)
    ));
    enc.send_frame(&s16_samples(480)).unwrap();
    assert!(
        enc.receive_packet().is_ok(),
        "two halves must make one frame"
    );
}

#[test]
fn flush_emits_tail_packet() {
    // Pre-fix: send_eof dropped the residual samples entirely.
    // Post-fix: the tail is encoded at the smallest legal Opus frame length.
    let mut enc = opus_encoder();
    enc.send_frame(&s16_samples(200)).unwrap(); // < one 960-sample frame
    enc.send_eof().unwrap();
    assert!(
        enc.receive_packet().is_ok(),
        "tail frame must be emitted on flush"
    );
}
