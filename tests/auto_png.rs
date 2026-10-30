use image::{GenericImageView, Rgba, RgbaImage};
use image_optimizer::optimizer::{OutputFormat, PngMode, Settings, optimize};
use std::{fs, path::Path, sync::atomic::AtomicBool};

fn run(
    input: &Path,
    output: &Path,
    mode: PngMode,
    width: Option<u32>,
) -> image_optimizer::optimizer::Outcome {
    image_optimizer::png_worker::configure_executable(env!("CARGO_BIN_EXE_image-optimizer").into());
    optimize(
        input,
        output,
        &Settings {
            format: OutputFormat::Png,
            png_mode: mode,
            max_width: width,
            ..Default::default()
        },
        &AtomicBool::new(false),
    )
    .unwrap()
}

#[test]
fn auto_uses_a_smaller_palette_but_respects_custom_width_and_transparency() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("logo.png");
    // Flat, diagonal logo edges gain many intermediate colors after resizing.
    RgbaImage::from_fn(600, 240, |x, y| {
        if x < 20 || x > 580 || y < 20 || y > 220 {
            Rgba([0, 0, 0, 0])
        } else if (x + 2 * y) % 97 < 45 {
            Rgba([0, 154, 151, 255])
        } else {
            Rgba([170, 205, 0, 255])
        }
    })
    .save(&input)
    .unwrap();
    let original = fs::read(&input).unwrap();
    let baseline = run(&input, dir.path(), PngMode::Zopfli, Some(317));
    let auto = run(&input, dir.path(), PngMode::Auto, Some(317));
    assert_eq!((auto.width, auto.height), (317, 127));
    let decoded = image::open(&auto.path).unwrap().to_rgba8();
    assert_eq!(decoded.dimensions(), (317, 127));
    assert!(
        auto.output_bytes < baseline.output_bytes,
        "{} >= {}",
        auto.output_bytes,
        baseline.output_bytes
    );
    assert!(auto.encoder.starts_with("imagequant"), "{}", auto.encoder);
    assert!(auto.notes.iter().any(|n| n.contains("Kolory mogą")));
    assert_eq!(decoded.get_pixel(0, 0)[3], 0);
    assert_eq!(decoded.get_pixel(158, 63)[3], 255);
    assert_eq!(fs::read(&input).unwrap(), original);
}

#[test]
fn auto_rejects_a_low_quality_palette_and_keeps_lossless_pixels() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("noise.png");
    let mut rng = 123456789u32;
    let pixels = RgbaImage::from_fn(48, 48, |_, _| {
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            rng as u8
        };
        Rgba([next(), next(), next(), 255])
    });
    pixels.save(&input).unwrap();
    let auto = run(&input, dir.path(), PngMode::Auto, None);
    assert!(!auto.encoder.starts_with("imagequant"));
    assert_eq!(image::open(&auto.path).unwrap().to_rgba8(), pixels);
    assert!(
        auto.notes
            .iter()
            .any(|n| n.contains("wybrano kompresję bezstratną"))
    );
}

#[test]
fn auto_preserves_sixteen_bit_precision_and_never_upscales() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("16bit.png");
    let pixels =
        image::ImageBuffer::from_fn(32, 16, |x, y| image::Luma([(x * 1234 + y * 357) as u16]));
    image::DynamicImage::ImageLuma16(pixels.clone())
        .save(&input)
        .unwrap();
    let auto = run(&input, dir.path(), PngMode::Auto, Some(1234));
    let decoded = image::open(&auto.path).unwrap();
    assert_eq!(decoded.dimensions(), (32, 16));
    assert_eq!(decoded.color(), image::ColorType::L16);
    assert_eq!(decoded.to_luma16(), pixels);
}

#[test]
fn cli_defaults_to_auto_and_accepts_a_custom_width() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("small.png");
    RgbaImage::from_pixel(64, 32, Rgba([50, 100, 150, 255]))
        .save(&input)
        .unwrap();
    let output = dir.path().join("out");
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_image-optimizer"))
        .args(["--batch", "--format", "png", "--width", "37", "--output"])
        .arg(&output)
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("Auto PNG: selected"));
    assert_eq!(
        image::open(output.join("small.png")).unwrap().dimensions(),
        (37, 19)
    );
}
