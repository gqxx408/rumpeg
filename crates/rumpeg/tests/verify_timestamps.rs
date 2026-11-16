//! Minimal public-API verification for the timestamp fixes.
//!
//! Run: `cargo test -p rumpeg --test verify_timestamps`
//!
//! The AAC encoder previously used only the backend's internal sample counter
//! as packet pts, ignoring the input frames' pts — output started at tick 0
//! regardless of where the caller's timeline began. The fix records the first
//! input frame's pts as a base and offsets all output packets against it.
//!
//! (The H264 multi-frame-per-packet fix is covered by the same commit; its
//! observable path needs a container that puts several AUs in one packet,
//! which the current demuxers do not produce, so the AAC sample below is the
//! externally reachable proof.)

use rumpeg::codec::EncoderContext;
use rumpeg::prelude::*;

#[test]
fn aac_output_pts_tracks_input_frame_base() {
    // Pre-fix: first packet.pts = 0 (internal counter) even for frames at 9000.
    // Post-fix: base pts from the first input frame offsets all output packets.
    let params = CodecParams::audio_codec(
        CodecId::Aac,
        AudioParams {
            sample_fmt: SampleFormat::S16,
            sample_rate: 48_000,
            layout: ChannelLayout::STEREO,
            frame_size: 0,
        },
    );
    let mut enc = EncoderContext::open(&params).unwrap();

    let mut frame =
        AudioFrame::alloc_interleaved(SampleFormat::S16, 48_000, ChannelLayout::STEREO, 8192);
    frame.pts = Timestamp::new(9000);
    for c in frame.data_mut().chunks_exact_mut(2) {
        c.copy_from_slice(&1000i16.to_le_bytes());
    }

    // rusty_aac buffers internally; feed several frames so at least one packet
    // is emitted, then drain.
    for _ in 0..4 {
        enc.send_frame(&Frame::Audio(frame.clone())).unwrap();
    }
    enc.send_eof().unwrap();
    let pkt = enc
        .receive_packet()
        .expect("AAC must emit after several frames");
    assert!(
        pkt.pts.0 >= 9000,
        "output pts must start at the input frame's base (9000), got {}",
        pkt.pts.0
    );
}
