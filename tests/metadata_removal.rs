use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb, Rgba, RgbaImage};
use image_optimizer::optimizer::{OutputFormat, PngMode, Settings, WebpMode, optimize};
use std::{borrow::Cow, fs, io::Cursor, path::Path, sync::atomic::AtomicBool};

fn exif(orientation: u8) -> Vec<u8> {
    // Valid TIFF IFD with orientation and a GPS IFD pointer; GPS IFD is empty.
    let mut bytes = b"II\x2a\0\x08\0\0\0\x02\0\x12\x01\x03\0\x01\0\0\0\x01\0\0\0\x25\x88\x04\0\x01\0\0\0\x26\0\0\0\0\0\0\0\0\0\0\0\0\0".to_vec();
    bytes[18] = orientation;
    bytes
}

fn tagged_png(path: &Path, profile: bool, depth16: bool, gamma: Option<f32>) -> Vec<u8> {
    let mut info = png::Info::with_size(16, 8);
    info.color_type = png::ColorType::Rgba;
    info.bit_depth = if depth16 {
        png::BitDepth::Sixteen
    } else {
        png::BitDepth::Eight
    };
    info.exif_metadata = Some(Cow::Owned(exif(1)));
    info.pixel_dims = Some(png::PixelDimensions {
        xppu: 11811,
        yppu: 11811,
        unit: png::Unit::Meter,
    });
    info.icc_profile = profile.then(|| Cow::Owned(lcms2::Profile::new_srgb().icc().unwrap()));
    info.source_gamma = gamma.map(png::ScaledFloat::new);
    let mut data = Vec::new();
    {
        let mut enc = png::Encoder::with_info(&mut data, info).unwrap();
        enc.add_text_chunk("Author".into(), "PRIVATE-AUTHOR".into())
            .unwrap();
        enc.add_ztxt_chunk("Comment".into(), "PRIVATE-COMMENT".into())
            .unwrap();
        enc.add_itxt_chunk(
            "XML:com.adobe.xmp".into(),
            "<xmp>PRIVATE-LOCATION</xmp>".into(),
        )
        .unwrap();
        let mut writer = enc.write_header().unwrap();
        let pixels = if depth16 {
            [32123_u16, 22111, 54321, 12345]
                .iter()
                .flat_map(|x| x.to_be_bytes())
                .collect::<Vec<_>>()
                .repeat(128)
        } else {
            [128, 64, 32, 128].repeat(128)
        };
        writer.write_image_data(&pixels).unwrap();
    }
    fs::write(path, &data).unwrap();
    data
}

fn run(input: &Path, settings: Settings) -> (Vec<u8>, DynamicImage) {
    image_optimizer::png_worker::configure_executable(env!("CARGO_BIN_EXE_image-optimizer").into());
    let dir = tempfile::tempdir().unwrap();
    let original = fs::read(input).unwrap();
    let result = optimize(input, dir.path(), &settings, &AtomicBool::new(false)).unwrap();
    assert_eq!(fs::read(input).unwrap(), original);
    assert!(result.notes.iter().any(|s| s.contains("Metadane usunięte")));
    let data = fs::read(&result.path).unwrap();
    assert_clean(&data);
    (data, image::open(result.path).unwrap())
}

fn chunks(data: &[u8], webp: bool) -> Vec<([u8; 4], &[u8])> {
    let mut pos = if webp { 12 } else { 8 };
    let mut chunks = Vec::new();
    while pos < data.len() {
        let (kind, len, offset, extra) = if webp {
            (
                data[pos..pos + 4].try_into().unwrap(),
                u32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap()) as usize,
                8,
                0,
            )
        } else {
            (
                data[pos + 4..pos + 8].try_into().unwrap(),
                u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()) as usize,
                8,
                4,
            )
        };
        chunks.push((kind, &data[pos + offset..pos + offset + len]));
        pos += offset + len + extra + if webp { len % 2 } else { 0 };
    }
    chunks
}

fn assert_clean(data: &[u8]) {
    match image::guess_format(data).unwrap() {
        image::ImageFormat::Png => {
            for (kind, _) in chunks(data, false) {
                assert!(
                    matches!(&kind, b"IHDR" | b"PLTE" | b"tRNS" | b"IDAT" | b"IEND"),
                    "PNG chunk {kind:?}"
                );
            }
            let reader = png::Decoder::new(Cursor::new(data)).read_info().unwrap();
            let info = reader.info();
            assert!(
                info.icc_profile.is_none()
                    && info.exif_metadata.is_none()
                    && info.pixel_dims.is_none()
            );
        }
        image::ImageFormat::WebP => {
            for (kind, _) in chunks(data, true) {
                assert!(
                    matches!(&kind, b"VP8 " | b"VP8L" | b"VP8X" | b"ALPH"),
                    "WebP chunk {kind:?}"
                );
            }
        }
        image::ImageFormat::Jpeg => {
            // APP/COM are markers outside entropy data; stuffing makes this scan unambiguous.
            for pair in data.windows(2) {
                assert!(
                    !(pair[0] == 0xff && ((0xe0..=0xef).contains(&pair[1]) || pair[1] == 0xfe)),
                    "JPEG metadata marker {pair:?}"
                );
            }
        }
        _ => panic!("unexpected output"),
    }
}

#[test]
fn direct_png_strips_all_metadata_including_density_for_lossless_and_zopfli() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private.png");
    let original = tagged_png(&path, false, false, None);
    let kinds: Vec<_> = chunks(&original, false).into_iter().map(|c| c.0).collect();
    for kind in [*b"eXIf", *b"pHYs", *b"tEXt", *b"zTXt", *b"iTXt"] {
        assert!(kinds.contains(&kind));
    }
    for format in [OutputFormat::Png, OutputFormat::Original] {
        for png_mode in [PngMode::Lossless, PngMode::Zopfli] {
            let (_, decoded) = run(
                &path,
                Settings {
                    format,
                    png_mode,
                    max_width: None,
                    ..Default::default()
                },
            );
            assert_eq!(decoded.to_rgba8(), image::open(&path).unwrap().to_rgba8());
        }
    }
}

#[test]
fn tagged_png_is_clean_for_all_encoders_and_resize_paths() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private.png");
    tagged_png(&path, true, false, None);
    for max_width in [None, Some(8)] {
        for format in [
            OutputFormat::Webp,
            OutputFormat::Png,
            OutputFormat::Jpeg,
            OutputFormat::Original,
        ] {
            for mode in 0..4 {
                let (_, decoded) = run(
                    &path,
                    Settings {
                        format,
                        max_width,
                        png_mode: [
                            PngMode::Lossless,
                            PngMode::Palette,
                            PngMode::Zopfli,
                            PngMode::Auto,
                        ][mode],
                        webp_mode: [WebpMode::Lossy, WebpMode::Lossless, WebpMode::NearLossless]
                            [mode % 3],
                        ..Default::default()
                    },
                );
                assert_eq!(
                    decoded.dimensions(),
                    if max_width.is_some() { (8, 4) } else { (16, 8) }
                );
            }
        }
    }
}

#[test]
fn tagged_16_bit_png_keeps_precision_and_alpha_without_embedding_profile() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("16bit.png");
    tagged_png(&path, true, true, None);
    let (data, decoded) = run(
        &path,
        Settings {
            format: OutputFormat::Original,
            ..Default::default()
        },
    );
    assert_eq!(data[24], 16, "PNG bit depth");
    for p in decoded.to_rgba16().pixels() {
        for (channel, value) in p.0.iter().zip([32123_u16, 22111, 54321, 12345]) {
            assert!(channel.abs_diff(value) <= 2, "{p:?}");
        }
        assert_eq!(p[3], 12345);
    }
}

#[test]
fn gamma_is_applied_before_removing_color_chunks() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("linear.png");
    tagged_png(&path, false, false, Some(1.0));
    for format in [OutputFormat::Png, OutputFormat::Webp] {
        let (_, image) = run(
            &path,
            Settings {
                format,
                webp_mode: WebpMode::Lossless,
                ..Default::default()
            },
        );
        let p = image.to_rgba8().get_pixel(0, 0).0;
        assert!(
            p[0].abs_diff(188) <= 1 && p[1].abs_diff(137) <= 1 && p[2].abs_diff(99) <= 1,
            "{p:?}"
        );
        assert_eq!(p[3], 128);
    }
}

fn jpeg_segment(out: &mut Vec<u8>, marker: u8, payload: &[u8]) {
    out.extend([0xff, marker]);
    out.extend(((payload.len() + 2) as u16).to_be_bytes());
    out.extend(payload);
}

#[test]
fn jpeg_exif_gps_xmp_iptc_icc_and_comments_are_removed_after_orientation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private.jpg");
    ImageBuffer::from_pixel(24, 12, Rgb([128_u8, 64, 32]))
        .save(&path)
        .unwrap();
    let jpeg = fs::read(&path).unwrap();
    let mut tagged = vec![0xff, 0xd8];
    jpeg_segment(
        &mut tagged,
        0xe1,
        &[b"Exif\0\0".as_slice(), &exif(6)].concat(),
    );
    jpeg_segment(
        &mut tagged,
        0xe1,
        b"http://ns.adobe.com/xap/1.0/\0<xmp>PRIVATE</xmp>",
    );
    jpeg_segment(&mut tagged, 0xed, b"Photoshop 3.0\0PRIVATE-IPTC");
    jpeg_segment(
        &mut tagged,
        0xe2,
        &[
            b"ICC_PROFILE\0\x01\x01".as_slice(),
            &lcms2::Profile::new_srgb().icc().unwrap(),
        ]
        .concat(),
    );
    jpeg_segment(&mut tagged, 0xfe, b"PRIVATE-COMMENT");
    tagged.extend(&jpeg[2..]);
    fs::write(&path, tagged).unwrap();
    for format in [
        OutputFormat::Jpeg,
        OutputFormat::Original,
        OutputFormat::Png,
        OutputFormat::Webp,
    ] {
        let (_, decoded) = run(
            &path,
            Settings {
                format,
                max_width: None,
                ..Default::default()
            },
        );
        assert_eq!(decoded.dimensions(), (12, 24));
    }
}

fn riff_chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut result = kind.to_vec();
    result.extend((payload.len() as u32).to_le_bytes());
    result.extend(payload);
    if payload.len() % 2 == 1 {
        result.push(0);
    }
    result
}

#[test]
fn webp_metadata_is_removed_in_all_modes_without_losing_alpha() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("plain.png");
    RgbaImage::from_pixel(16, 8, Rgba([128, 64, 32, 128]))
        .save(&input)
        .unwrap();
    let (base, _) = run(
        &input,
        Settings {
            webp_mode: WebpMode::Lossless,
            ..Default::default()
        },
    );
    let mut body = riff_chunk(b"VP8X", &[0x3c, 0, 0, 0, 15, 0, 0, 7, 0, 0]);
    body.extend(riff_chunk(
        b"ICCP",
        &lcms2::Profile::new_srgb().icc().unwrap(),
    ));
    for (kind, data) in chunks(&base, true) {
        if kind != *b"VP8X" {
            body.extend(riff_chunk(&kind, data));
        }
    }
    body.extend(riff_chunk(b"EXIF", &exif(1)));
    body.extend(riff_chunk(b"XMP ", b"<xmp>PRIVATE-LOCATION</xmp>"));
    let mut tagged = b"RIFF".to_vec();
    tagged.extend(((body.len() + 4) as u32).to_le_bytes());
    tagged.extend(b"WEBP");
    tagged.extend(body);
    let path = dir.path().join("private.webp");
    fs::write(&path, tagged).unwrap();
    for webp_mode in [WebpMode::Lossy, WebpMode::Lossless, WebpMode::NearLossless] {
        let (_, decoded) = run(
            &path,
            Settings {
                webp_mode,
                ..Default::default()
            },
        );
        assert!(decoded.to_rgba8().pixels().all(|p| p[3] == 128));
        if webp_mode == WebpMode::Lossless {
            assert_eq!(decoded.to_rgba8().get_pixel(0, 0).0, [128, 64, 32, 128]);
        }
    }
}

#[test]
fn cicp_color_is_applied_before_stripping_and_unsupported_hdr_is_not_saved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("color.png");
    for (primaries, transfer, supported) in [(1, 13, true), (12, 13, true), (9, 16, false)] {
        let mut info = png::Info::with_size(2, 2);
        info.color_type = png::ColorType::Rgba;
        let mut bytes = Vec::new();
        {
            let mut enc = png::Encoder::with_info(&mut bytes, info)
                .unwrap()
                .write_header()
                .unwrap();
            enc.write_chunk(png::chunk::cICP, &[primaries, transfer, 0, 1])
                .unwrap();
            enc.write_image_data(&[128, 64, 32, 128].repeat(4)).unwrap();
        }
        assert!(
            chunks(&bytes, false)
                .iter()
                .any(|(kind, _)| kind == b"cICP")
        );
        fs::write(&path, &bytes).unwrap();
        let settings = Settings {
            format: OutputFormat::Png,
            ..Default::default()
        };
        if supported {
            let (_, decoded) = run(&path, settings);
            let pixel = decoded.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(pixel[3], 128);
            if primaries == 1 {
                assert_eq!(pixel, [128, 64, 32, 128]);
            } else {
                assert!(
                    pixel[0] > 128 && pixel[1] < 64 && pixel[2] < 32,
                    "P3 not converted: {pixel:?}"
                );
            }
        } else {
            let out = dir.path().join("rejected");
            let err = optimize(&path, &out, &settings, &AtomicBool::new(false)).unwrap_err();
            assert!(err.to_string().contains("cICP/HDR"));
            assert!(!out.exists());
        }
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn grayscale_16_bit_icc_is_applied_without_truncation_and_keeps_alpha() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gray.png");
    let mut info = png::Info::with_size(2, 2);
    info.bit_depth = png::BitDepth::Sixteen;
    info.color_type = png::ColorType::GrayscaleAlpha;
    let profile = lcms2::Profile::new_gray(
        &lcms2::CIExyY {
            x: 0.3127,
            y: 0.329,
            Y: 1.0,
        },
        &lcms2::ToneCurve::new(1.0),
    )
    .unwrap();
    info.icc_profile = Some(Cow::Owned(profile.icc().unwrap()));
    let mut bytes = Vec::new();
    {
        let mut writer = png::Encoder::with_info(&mut bytes, info)
            .unwrap()
            .write_header()
            .unwrap();
        writer
            .write_image_data(
                &[32768_u16, 12345]
                    .iter()
                    .flat_map(|v| v.to_be_bytes())
                    .collect::<Vec<_>>()
                    .repeat(4),
            )
            .unwrap();
    }
    fs::write(&path, bytes).unwrap();
    let (encoded, decoded) = run(
        &path,
        Settings {
            format: OutputFormat::Png,
            ..Default::default()
        },
    );
    assert_eq!(encoded[24], 16);
    let pixel = decoded.to_rgba16().get_pixel(0, 0).0;
    for value in &pixel[..3] {
        assert!((48100..=48300).contains(value), "{pixel:?}");
    }
    assert_eq!(pixel[3], 12345);
}
