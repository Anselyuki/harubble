use anyhow::Result;
use harubble_core::audio::FlacMetadata;
use harubble_core::{save_audio, tag_flac, OutputFormat};

fn build_test_wav() -> Vec<u8> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(&mut cursor, spec).expect("wav writer");
        for sample in [0_i16, 1024, -1024, 512, -512] {
            writer.write_sample(sample).expect("sample");
        }
        writer.finalize().expect("finalize");
    }
    cursor.into_inner()
}

fn build_float_wav(samples: &[f32]) -> Vec<u8> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::new(&mut cursor, spec).expect("wav writer");
        for sample in samples {
            writer.write_sample(*sample).expect("sample");
        }
        writer.finalize().expect("finalize");
    }
    cursor.into_inner()
}

fn make_nonstandard_40_byte_float_fmt_chunk(mut wav: Vec<u8>) -> Vec<u8> {
    let fmt_offset = wav
        .windows(4)
        .position(|window| window == b"fmt ")
        .expect("fmt chunk");
    let payload_offset = fmt_offset + 8;
    let fmt_len = u32::from_le_bytes(
        wav[fmt_offset + 4..fmt_offset + 8]
            .try_into()
            .expect("fmt length"),
    ) as usize;
    assert_eq!(fmt_len, 40);

    wav[payload_offset..payload_offset + 2].copy_from_slice(&3_u16.to_le_bytes());
    wav[payload_offset + 16..payload_offset + 40].fill(0);
    wav
}

#[test]
fn writes_flac_vorbis_comments_after_wav_conversion() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let wav_bytes = build_test_wav();
    let flac_path = save_audio(&wav_bytes, temp_dir.path(), "test-song", OutputFormat::Flac)?;

    let artists = vec![String::from("Test Artist")];
    let album_artists = vec![String::from("Test Album Artist")];

    tag_flac(
        &flac_path,
        &FlacMetadata {
            title: "Test Song",
            artists: &artists,
            album: "Test Album",
            album_artists: &album_artists,
            track_number: Some(2),
            total_tracks: Some(9),
            disc_number: Some(1),
            total_discs: Some(1),
            cover: None,
        },
    )?;

    let tag = metaflac::Tag::read_from_path(&flac_path)?;
    let comments = tag
        .vorbis_comments()
        .ok_or_else(|| anyhow::anyhow!("missing vorbis comments"))?;

    assert_eq!(
        comments.title().map(|items| items.as_slice()),
        Some([String::from("Test Song")].as_slice())
    );
    assert_eq!(
        comments.artist().map(|items| items.as_slice()),
        Some([String::from("Test Artist")].as_slice())
    );
    assert_eq!(
        comments.album().map(|items| items.as_slice()),
        Some([String::from("Test Album")].as_slice())
    );
    assert_eq!(
        comments.album_artist().map(|items| items.as_slice()),
        Some([String::from("Test Album Artist")].as_slice())
    );
    assert_eq!(comments.track(), Some(2));
    assert_eq!(comments.total_tracks(), Some(9));
    assert_eq!(comments.get("DISCNUMBER"), Some(&vec![String::from("1")]));
    assert_eq!(comments.get("TOTALDISCS"), Some(&vec![String::from("1")]));

    Ok(())
}

#[test]
fn converts_float_wav_to_compatible_24_bit_flac() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let wav_bytes = build_float_wav(&[-1.0, 1.0, -0.5, 0.5, 0.0, 0.0]);

    let flac_path = save_audio(
        &wav_bytes,
        temp_dir.path(),
        "float-source",
        OutputFormat::Flac,
    )?;

    let tag = metaflac::Tag::read_from_path(&flac_path)?;
    let stream_info = tag.get_blocks(metaflac::BlockType::StreamInfo).next();
    let Some(metaflac::Block::StreamInfo(stream_info)) = stream_info else {
        anyhow::bail!("missing FLAC stream info");
    };
    assert_eq!(stream_info.bits_per_sample, 24);
    assert_eq!(stream_info.sample_rate, 48_000);
    assert_eq!(stream_info.num_channels, 2);

    let artists = vec![String::from("Float Artist")];
    let album_artists = vec![String::from("Float Album Artist")];
    tag_flac(
        &flac_path,
        &FlacMetadata {
            title: "Float Source",
            artists: &artists,
            album: "Float Album",
            album_artists: &album_artists,
            track_number: Some(1),
            total_tracks: Some(1),
            disc_number: None,
            total_discs: None,
            cover: None,
        },
    )?;

    let tagged = metaflac::Tag::read_from_path(&flac_path)?;
    let comments = tagged
        .vorbis_comments()
        .ok_or_else(|| anyhow::anyhow!("missing vorbis comments"))?;
    assert_eq!(
        comments.title().map(|items| items.as_slice()),
        Some([String::from("Float Source")].as_slice())
    );
    assert_eq!(
        comments.artist().map(|items| items.as_slice()),
        Some([String::from("Float Artist")].as_slice())
    );

    Ok(())
}

#[test]
fn converts_float_wav_with_nonstandard_40_byte_fmt_chunk() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let wav_bytes =
        make_nonstandard_40_byte_float_fmt_chunk(build_float_wav(&[-1.0, 1.0, -0.25, 0.25]));

    let flac_path = save_audio(
        &wav_bytes,
        temp_dir.path(),
        "oversized-fmt-source",
        OutputFormat::Flac,
    )?;

    let tag = metaflac::Tag::read_from_path(&flac_path)?;
    let stream_info = tag.get_blocks(metaflac::BlockType::StreamInfo).next();
    let Some(metaflac::Block::StreamInfo(stream_info)) = stream_info else {
        anyhow::bail!("missing FLAC stream info");
    };
    assert_eq!(stream_info.bits_per_sample, 24);
    assert_eq!(stream_info.sample_rate, 48_000);
    assert_eq!(stream_info.num_channels, 2);

    Ok(())
}

#[test]
fn rejects_non_finite_float_wav_samples() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let wav_bytes = build_float_wav(&[0.0, f32::NAN]);

    let error = save_audio(
        &wav_bytes,
        temp_dir.path(),
        "invalid-float-source",
        OutputFormat::Flac,
    )
    .expect_err("non-finite samples must not be encoded");

    assert!(
        format!("{error:#}").contains("WAV contains a non-finite float sample"),
        "{error:#}"
    );
    Ok(())
}
