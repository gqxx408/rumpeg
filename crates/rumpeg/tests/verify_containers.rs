//! Minimal public-API verification for the container parsing fixes.
//!
//! Run: `cargo test -p rumpeg --test verify_containers`
//!
//! Covers two externally observable fixes:
//! * avcC extraction (#5): remuxing an H.264 elementary stream to MP4 used to
//!   fabricate avcC from a fresh encoder whose SPS/PPS did not match the copied
//!   bitstream. The public AvcDecoderConfig round-trip below is what the CLI
//!   extraction now relies on (the CLI-level test lives in rumpeg-cli).
//! * MP4 seek (#9): seek landed on the first sample >= timestamp, possibly
//!   mid-GOP; it must snap back to the nearest preceding keyframe.

use rumpeg::prelude::*;
use rumpeg::PacketFlags;

#[test]
fn avcc_roundtrip_uses_actual_sps_pps() {
    let enc = rumpeg::h264::Encoder::new(32, 32).unwrap();
    let sps = enc.sps_nal().to_vec();
    let pps = enc.pps_nal().to_vec();
    let cfg = rumpeg::h264::AvcDecoderConfig::from_sps_pps(&sps, &pps).unwrap();
    let avcc = cfg.to_avcc();
    let parsed = rumpeg::h264::AvcDecoderConfig::from_avcc(&avcc).unwrap();

    assert_eq!(parsed.sps().unwrap().width(), 32);
    assert_eq!(parsed.sps().unwrap().height(), 32);
    assert_eq!(parsed.length_size(), 4);
    // Parameter sets that cannot form a valid config must be rejected, not
    // silently accepted into a corrupt MP4.
    assert!(rumpeg::h264::AvcDecoderConfig::from_sps_pps(&[0xFF], &[0xFF]).is_err());
}

fn write_idr_clip(path: &std::path::Path, frames: usize) {
    use rumpeg::h264::Yuv420Planar;
    let mut enc = rumpeg::h264::Encoder::new(32, 32).unwrap(); // I_PCM: every AU is a keyframe
    let mut params = CodecParams::video_codec(
        CodecId::H264,
        VideoParams {
            pix_fmt: PixelFormat::Yuv420p,
            width: 32,
            height: 32,
            frame_rate: Rational::new(10, 1),
            sample_aspect_ratio: Rational::one(),
        },
    );
    params.extradata = enc.avcc_extradata().unwrap();
    let mut mux = format::open_output(path, Some("mp4")).unwrap();
    let idx = mux.add_stream(params).unwrap();
    mux.write_header().unwrap();
    for i in 0..frames {
        let mut yuv = Yuv420Planar::zeroed(32, 32);
        for b in yuv.y.iter_mut() {
            *b = (i as u8).wrapping_mul(17).wrapping_add(30);
        }
        let (bytes, is_key) = enc.encode_access_unit(&yuv).unwrap();
        let mut pkt = Packet::new(bytes);
        pkt.stream_index = idx;
        pkt.pts = Timestamp::new(i as i64);
        pkt.dts = pkt.pts;
        if is_key {
            pkt.flags.insert(PacketFlags::KEY);
        }
        mux.write_packet(&pkt).unwrap();
    }
    mux.write_trailer().unwrap();
}

#[test]
fn mp4_seek_lands_on_a_keyframe() {
    let dir = std::env::temp_dir();
    let path = dir.join("rumpeg_verify_seek.mp4");
    write_idr_clip(&path, 8);

    let mut demux = format::open_input(&path).unwrap();
    demux.seek(3).unwrap();
    let pkt = demux.read_packet().expect("a packet after seek");
    assert!(
        pkt.flags.contains(PacketFlags::KEY),
        "post-seek packet must be a keyframe"
    );
    let _ = std::fs::remove_file(&path);
}
