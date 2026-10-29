use image::{Rgba, RgbaImage};
use image_optimizer::optimizer::{OutputFormat, Settings, TrimMode, WebpMode, optimize, supported};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

fn settings(format: OutputFormat, lossless: bool) -> Settings {
    Settings {
        format,
        max_width: None,
        avif_lossless: lossless,
        jxl_lossless: lossless,
        ..Default::default()
    }
}
fn run(input: &Path, output: &Path, settings: &Settings) -> PathBuf {
    optimize(input, output, settings, &AtomicBool::new(false))
        .unwrap()
        .path
}
fn sample(path: &Path) -> RgbaImage {
    let image = RgbaImage::from_fn(96, 64, |x, y| {
        Rgba([
            (x * 2 + y) as u8,
            (y * 3) as u8,
            (x + y * 2) as u8,
            match x % 4 {
                0 => 0,
                1 => 64,
                2 => 128,
                _ => 255,
            },
        ])
    });
    image.save(path).unwrap();
    image
}

#[test]
fn lossless_roundtrip_preserves_every_rgba_byte_and_source() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("logo ze spacją.png");
    let pixels = sample(&input);
    let before = fs::read(&input).unwrap();
    for (format, ext) in [(OutputFormat::Avif, "avif"), (OutputFormat::Jxl, "jxl")] {
        let encoded = run(&input, &dir.path().join(ext), &settings(format, true));
        assert_eq!(encoded.extension().unwrap(), ext);
        assert_eq!(encoded.file_stem(), input.file_stem());
        let decoded = run(
            &encoded,
            &dir.path().join(format!("{ext}-png")),
            &settings(OutputFormat::Png, true),
        );
        assert_eq!(
            image::open(decoded).unwrap().to_rgba8(),
            pixels,
            "{ext} lossless RGBA"
        );
        assert_eq!(fs::read(&input).unwrap(), before);
        let second = run(&input, encoded.parent().unwrap(), &settings(format, true));
        assert_ne!(encoded, second);
        assert_eq!(fs::read(&encoded).unwrap(), fs::read(second).unwrap());
    }
}

#[test]
fn lossy_quality_changes_data_and_alpha_remains_exact() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("photo.png");
    let pixels = sample(&input);
    for format in [OutputFormat::Avif, OutputFormat::Jxl] {
        let mut low = settings(format, false);
        low.quality = 35;
        let mut high = low.clone();
        high.quality = 92;
        let a = run(&input, dir.path(), &low);
        let b = run(&input, dir.path(), &high);
        assert_ne!(fs::read(&a).unwrap(), fs::read(&b).unwrap());
        let decoded = run(
            &b,
            &dir.path().join("png"),
            &settings(OutputFormat::Png, false),
        );
        let decoded = image::open(decoded).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), pixels.dimensions());
        assert_eq!(
            decoded.pixels().map(|p| p[3]).collect::<Vec<_>>(),
            pixels.pixels().map(|p| p[3]).collect::<Vec<_>>()
        );
        let error = pixels
            .pixels()
            .zip(decoded.pixels())
            .filter(|(p, _)| p[3] > 0)
            .flat_map(|(a, b)| (0..3).map(move |c| f64::from(a[c].abs_diff(b[c]))))
            .collect::<Vec<_>>();
        assert!(error.iter().sum::<f64>() / (error.len() as f64) < 15.0);
    }
}

#[test]
fn trim_resize_original_mode_and_webp_import_work_for_both_formats() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("logo.png");
    RgbaImage::from_fn(200, 120, |x, y| {
        if (30..170).contains(&x) && (20..100).contains(&y) {
            Rgba([180, 20, 80, 255])
        } else {
            Rgba([0, 0, 0, 0])
        }
    })
    .save(&input)
    .unwrap();
    for format in [OutputFormat::Avif, OutputFormat::Jxl] {
        let mut config = settings(format, true);
        config.trim = TrimMode::Transparent;
        config.max_width = Some(70);
        let result = optimize(&input, dir.path(), &config, &AtomicBool::new(false)).unwrap();
        assert_eq!((result.width, result.height), (70, 40));
        let original = run(
            &result.path,
            dir.path(),
            &settings(OutputFormat::Original, true),
        );
        assert_eq!(original.extension(), result.path.extension());
        let webp = run(
            &result.path,
            dir.path(),
            &Settings {
                webp_mode: WebpMode::Lossless,
                ..Default::default()
            },
        );
        assert_eq!(image::open(webp).unwrap().to_rgba8().dimensions(), (70, 40));
    }
}

#[test]
fn modern_formats_keep_svg_unchanged_and_fail_cleanly_on_corrupt_or_cancelled_jobs() {
    let dir = tempfile::tempdir().unwrap();
    let svg = dir.path().join("logo.svg");
    let text = br#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><metadata>keep me</metadata><rect width="20" height="20"/></svg>"#;
    fs::write(&svg, text).unwrap();
    for (format, ext) in [(OutputFormat::Avif, "AVIF"), (OutputFormat::Jxl, "JXL")] {
        let config = settings(format, false);
        let output = run(&svg, dir.path(), &config);
        assert_eq!(output.extension().unwrap(), "svg");
        assert_eq!(fs::read(output).unwrap(), text);
        let bad = dir.path().join(format!("corrupt.{ext}"));
        assert!(supported(&bad));
        fs::write(&bad, b"not an image").unwrap();
        let outdir = dir.path().join("failures");
        assert!(optimize(&bad, &outdir, &config, &AtomicBool::new(false)).is_err());
        assert!(
            optimize(&svg, &outdir, &config, &AtomicBool::new(true))
                .unwrap_err()
                .to_string()
                .contains("Anulowano")
        );
        assert!(!outdir.exists());
    }
}

// Parse the actual AVIF item information and properties, not byte-string matches
// against compressed payloads. Fresh AVIF exports must have no metadata items or ICC.
fn assert_avif_metadata_free(data: &[u8]) {
    fn boxes(mut data: &[u8], item_types: &mut Vec<[u8; 4]>) {
        while data.len() >= 8 {
            let len = u32::from_be_bytes(data[..4].try_into().unwrap()) as usize;
            assert!(len >= 8 && len <= data.len());
            let kind = &data[4..8];
            let body = &data[8..len];
            match kind {
                b"meta" => boxes(&body[4..], item_types),
                b"iprp" | b"ipco" => boxes(body, item_types),
                b"iinf" => boxes(&body[if body[0] == 0 { 6 } else { 8 }..], item_types),
                b"infe" => {
                    assert!(matches!(body[0], 2 | 3));
                    let offset = if body[0] == 2 { 8 } else { 10 };
                    item_types.push(body[offset..offset + 4].try_into().unwrap());
                }
                b"colr" => assert_eq!(&body[..4], b"nclx"),
                b"uuid" | b"Exif" | b"xml " => panic!("metadata box"),
                _ => {}
            }
            data = &data[len..];
        }
        assert!(data.is_empty());
    }
    let mut types = Vec::new();
    boxes(data, &mut types);
    assert!(!types.is_empty());
    assert!(
        types.iter().all(|t| t == b"av01"),
        "unexpected items: {types:?}"
    );
}

#[test]
fn output_contains_no_source_metadata_or_jxl_container_boxes() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("tagged.png");
    let file = fs::File::create(&input).unwrap();
    let mut encoder = png::Encoder::new(file, 32, 24);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .add_text_chunk("Author".into(), "WEBDROP_SECRET_METADATA".into())
        .unwrap();
    encoder
        .add_itxt_chunk("XML:com.adobe.xmp".into(), "WEBDROP_SECRET_METADATA".into())
        .unwrap();
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&vec![128; 32 * 24 * 4])
        .unwrap();
    for format in [OutputFormat::Avif, OutputFormat::Jxl] {
        for lossless in [false, true] {
            let result = run(&input, dir.path(), &settings(format, lossless));
            let bytes = fs::read(&result).unwrap();
            assert!(!bytes.windows(23).any(|w| w == b"WEBDROP_SECRET_METADATA"));
            if format == OutputFormat::Avif {
                assert_avif_metadata_free(&bytes);
            } else {
                assert_eq!(&bytes[..2], &[0xff, 0x0a]);
            } // bare codestream: no Exif/XMP/JPEG reconstruction boxes
            let reencoded = run(
                &result,
                dir.path(),
                &settings(OutputFormat::Original, lossless),
            );
            let bytes = fs::read(reencoded).unwrap();
            if format == OutputFormat::Avif {
                assert_avif_metadata_free(&bytes);
            } else {
                assert_eq!(&bytes[..2], &[0xff, 0x0a]);
            }
        }
    }
}

#[test]
fn imported_orientation_is_applied_and_exif_xmp_are_discarded() {
    let dir = tempfile::tempdir().unwrap();
    let source = RgbaImage::from_fn(48, 24, |x, y| {
        Rgba([
            if x < 24 { 220 } else { 20 },
            if y < 12 { 180 } else { 40 },
            70,
            255,
        ])
    });
    let expected = image::imageops::rotate90(&source);
    for (format, ext) in [(OutputFormat::Avif, "avif"), (OutputFormat::Jxl, "jxl")] {
        let input = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/fixtures/modern-rotated.{ext}"));
        let original = fs::read(&input).unwrap();
        assert!(original.windows(16).any(|w| w == b"PRIVATE-LOCATION"));
        let png = run(&input, dir.path(), &settings(OutputFormat::Png, true));
        assert_eq!(
            image::open(png).unwrap().to_rgba8(),
            expected,
            "orientation {ext}"
        );
        let result = run(&input, dir.path(), &settings(format, true));
        let bytes = fs::read(result).unwrap();
        if format == OutputFormat::Avif {
            assert_avif_metadata_free(&bytes);
        } else {
            assert_eq!(&bytes[..2], &[0xff, 0x0a]);
        }
        assert_eq!(fs::read(&input).unwrap(), original);
    }
}

#[test]
fn animations_hdr_and_truncated_modern_images_are_not_silently_flattened() {
    let dir = tempfile::tempdir().unwrap();
    for ext in ["avif", "jxl"] {
        for case in ["animated", "hdr"] {
            let input = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/modern-{case}.{ext}"));
            let result = optimize(
                &input,
                &dir.path().join("output"),
                &Settings::default(),
                &AtomicBool::new(false),
            );
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains(if case == "animated" {
                    "Animowane"
                } else {
                    "HDR"
                }),
                "{ext}: {error}"
            );
        }
        let valid = fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/modern-rotated.{ext}")),
        )
        .unwrap();
        let input = dir.path().join(format!("truncated.{ext}"));
        fs::write(&input, &valid[..valid.len() / 2]).unwrap();
        assert!(
            optimize(
                &input,
                &dir.path().join("output"),
                &Settings::default(),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
    assert!(!dir.path().join("output").exists());
}
