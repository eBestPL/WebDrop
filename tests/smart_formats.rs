use image::{GenericImageView, Rgba, RgbaImage};
use image_optimizer::optimizer::{OutputFormat, PngMode, Settings, optimize};
use std::{fs, path::Path, sync::atomic::AtomicBool};

fn run(path: &Path, output: &Path, settings: &Settings) -> image_optimizer::optimizer::Outcome {
    image_optimizer::png_worker::configure_executable(env!("CARGO_BIN_EXE_image-optimizer").into());
    optimize(path, output, settings, &AtomicBool::new(false)).unwrap()
}

fn decode_any(path: &Path, dir: &Path) -> image::DynamicImage {
    let result = run(
        path,
        dir,
        &Settings {
            format: OutputFormat::Png,
            png_mode: PngMode::Lossless,
            max_width: None,
            ..Default::default()
        },
    );
    image::open(result.path).unwrap()
}

#[test]
fn smart_saves_one_smallest_file_and_preserves_width_alpha_and_source() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("logo.png");
    let pixels = RgbaImage::from_fn(240, 96, |x, y| {
        if x < 8 || y < 8 || x > 230 || y > 85 {
            Rgba([255, 255, 255, 0])
        } else if (x + 2 * y) % 43 < 20 {
            Rgba([0, 154, 151, 255])
        } else {
            Rgba([170, 205, 0, 255])
        }
    });
    pixels.save(&input).unwrap();
    let source = fs::read(&input).unwrap();
    let settings = Settings {
        format: OutputFormat::Smart,
        max_width: Some(137),
        ..Default::default()
    };
    let output = dir.path().join("smart");
    let smart = run(&input, &output, &settings);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 1);
    assert_eq!(fs::read(&input).unwrap(), source);
    assert_eq!((smart.width, smart.height), (137, 55));
    assert_ne!(smart.path.extension().unwrap(), "jpg");
    assert_ne!(smart.path.extension().unwrap(), "jxl");
    let report = smart
        .notes
        .iter()
        .find(|n| n.starts_with("Inteligentny: wybrano"))
        .unwrap();
    for format in ["PNG:", "WebP:", "AVIF:"] {
        assert!(report.contains(format), "{report}");
    }
    assert!(!report.contains("JPG:") && !report.contains("JPEG XL:"));
    // Reported per-format minima agree with the actual winner's length.
    for item in report
        .split("Porównano: ")
        .nth(1)
        .unwrap()
        .trim_end_matches('.')
        .split("; ")
    {
        let count: u64 = item
            .rsplit_once(": ")
            .unwrap()
            .1
            .trim_end_matches(" B")
            .parse()
            .unwrap();
        assert!(smart.output_bytes <= count);
    }
    let png = run(
        &input,
        &dir.path().join("png"),
        &Settings {
            format: OutputFormat::Png,
            ..settings.clone()
        },
    );
    assert!(smart.output_bytes <= png.output_bytes);
    let image = decode_any(&smart.path, &dir.path().join("decode")).to_rgba8();
    assert_eq!(image.dimensions(), (137, 55));
    assert_eq!(image.get_pixel(0, 0)[3], 0);
    assert_eq!(image.get_pixel(70, 27)[3], 255);
}

#[test]
fn smart_checks_jpeg_for_opaque_images_and_jxl_only_when_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("photo.png");
    RgbaImage::from_fn(64, 48, |x, y| {
        Rgba([
            (x * 3) as u8,
            (y * 5) as u8,
            ((x * 7 + y * 11) % 256) as u8,
            255,
        ])
    })
    .save(&input)
    .unwrap();
    let settings = Settings {
        format: OutputFormat::Smart,
        max_width: None,
        ..Default::default()
    };
    let regular = run(&input, &dir.path().join("regular"), &settings);
    let all = run(
        &input,
        &dir.path().join("all"),
        &Settings {
            smart_jxl: true,
            ..settings
        },
    );
    assert!(regular.notes.iter().any(|n| n.contains("JPG:")));
    assert!(!regular.notes.iter().any(|n| n.contains("JPEG XL:")));
    assert!(
        all.notes.iter().any(|n| n.contains("JPEG XL:")),
        "{:?}",
        all.notes
    );
    assert!(all.output_bytes <= regular.output_bytes);
    assert_eq!(
        decode_any(&all.path, &dir.path().join("decode")).dimensions(),
        (64, 48)
    );
}

#[test]
fn svg_is_untouched_and_cancelled_smart_job_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("logo.svg");
    let data=br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><!-- preserve me --><path d="M0 0h100v50H0z"/></svg>"#;
    fs::write(&input, data).unwrap();
    let settings = Settings {
        format: OutputFormat::Smart,
        ..Default::default()
    };
    let result = run(&input, &dir.path().join("out"), &settings);
    assert_eq!(fs::read(result.path).unwrap(), data);
    let png = dir.path().join("input.png");
    RgbaImage::from_pixel(10, 10, Rgba([50, 100, 200, 255]))
        .save(&png)
        .unwrap();
    let output = dir.path().join("cancelled");
    assert!(optimize(&png, &output, &settings, &AtomicBool::new(true)).is_err());
    assert!(!output.exists() || fs::read_dir(output).unwrap().count() == 0);
}

#[test]
fn cli_defaults_to_smart_and_diagnostics_are_english_and_do_not_expose_source_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("tagged.png");
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 16, 8);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .add_text_chunk("Comment".into(), "PRIVATE-SMART-METADATA".into())
            .unwrap();
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&vec![100; 16 * 8 * 3])
            .unwrap();
    }
    fs::write(&input, png).unwrap();
    let output = dir.path().join("out");
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_image-optimizer"))
        .args(["--batch", "--output"])
        .arg(&output)
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains("Smart: selected"), "{text}");
    assert!(!text.contains("Inteligentny:"));
    let files: Vec<_> = fs::read_dir(output)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    assert!(
        !fs::read(&files[0])
            .unwrap()
            .windows(22)
            .any(|b| b == b"PRIVATE-SMART-METADATA")
    );
}

#[test]
fn new_settings_are_ready_for_smart_web_export() {
    let settings = Settings::default();
    assert_eq!(settings.format, OutputFormat::Smart);
    assert_eq!(settings.max_width, Some(1500));
    assert_eq!(settings.quality, 82);
    assert!(!settings.thorough);
    assert!(!settings.smart_jxl);
}

#[test]
fn fast_smart_also_tests_lossy_webp_when_a_palette_is_possible() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("mixed-content.png");
    // A flat banner over detailed pixels: a palette is possible, but this does
    // not mean that lossless WebP is the smallest suitable encoding.
    let mut state = 12345_u32;
    RgbaImage::from_fn(192, 160, |x, y| {
        if x < 120 && y > 80 {
            Rgba([205, 195, 235, 255])
        } else {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let value = state as u8;
            Rgba([value, value, value, 255])
        }
    })
    .save(&input)
    .unwrap();
    let settings = Settings::default();
    let webp = run(
        &input,
        &dir.path().join("manual"),
        &Settings {
            format: OutputFormat::Webp,
            ..settings.clone()
        },
    );
    let smart = run(&input, &dir.path().join("smart"), &settings);
    let report = smart
        .notes
        .iter()
        .find(|n| n.starts_with("Inteligentny: wybrano"))
        .unwrap();
    let webp_size: u64 = report
        .split("WebP: ")
        .nth(1)
        .unwrap()
        .split(" B")
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        webp_size <= webp.output_bytes,
        "Smart WebP {webp_size} B, manual lossy WebP {} B; {report}",
        webp.output_bytes
    );
    assert!(smart.output_bytes <= webp.output_bytes);
    assert_eq!((smart.width, smart.height), (192, 160));
}
