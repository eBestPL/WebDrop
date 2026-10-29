use image::{DynamicImage, GenericImageView, ImageEncoder, Rgba, RgbaImage};
use image_optimizer::optimizer::{
    OutputFormat, PngMode, Settings, WebpMode, optimize, target_dimensions,
};
use std::{fs, path::Path, sync::atomic::AtomicBool};

fn png(path: &Path, width: u32, height: u32) -> RgbaImage {
    let image = RgbaImage::from_fn(width, height, |x, y| {
        Rgba([
            (x % 256) as u8,
            (y % 256) as u8,
            ((x + y) % 256) as u8,
            if x < width / 3 {
                0
            } else if x < width / 2 {
                128
            } else {
                255
            },
        ])
    });
    image.save(path).unwrap();
    image
}

fn run(path: &Path, output: &Path, settings: &Settings) -> image_optimizer::optimizer::Outcome {
    optimize(path, output, settings, &AtomicBool::new(false)).unwrap()
}

#[test]
fn default_batch_converts_png_and_jpeg_to_1500_webp_without_touching_sources() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("Downloads");
    let first = dir.path().join("zdjęcie.png");
    let second = dir.path().join("photo.jpg");
    let pixels = png(&first, 1800, 900);
    DynamicImage::ImageRgba8(pixels)
        .to_rgb8()
        .save(&second)
        .unwrap();
    for input in [first, second] {
        let original = fs::read(&input).unwrap();
        let result = run(&input, &output, &Settings::default());
        assert_eq!(result.path.file_stem(), input.file_stem());
        assert_eq!(result.path.extension().unwrap(), "webp");
        assert_eq!(image::open(&result.path).unwrap().dimensions(), (1500, 750));
        assert_eq!(fs::read(&input).unwrap(), original);
    }
}

#[test]
fn resize_presets_keep_aspect_ratio_and_never_upscale() {
    for width in [1500, 1000, 700, 500] {
        assert_eq!(
            target_dimensions(3000, 2000, Some(width)),
            (
                width,
                (2000_u64 * width as u64 + 1500).div_euclid(3000) as u32
            )
        );
        assert_eq!(target_dimensions(320, 640, Some(width)), (320, 640));
    }
    assert_eq!(target_dimensions(3000, 1, Some(500)), (500, 1));
    assert_eq!(target_dimensions(3000, 2000, None), (3000, 2000));
}

#[test]
fn lossless_webp_preserves_all_pixels_including_transparent_rgb() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("alpha.png");
    let original = png(&input, 80, 40);
    let settings = Settings {
        webp_mode: WebpMode::Lossless,
        ..Default::default()
    };
    let result = run(&input, dir.path(), &settings);
    assert_eq!(image::open(result.path).unwrap().to_rgba8(), original);
}

#[test]
fn resized_alpha_has_no_dark_fringe() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("edge.png");
    RgbaImage::from_fn(100, 50, |x, _| {
        if x < 50 {
            Rgba([0, 0, 0, 0])
        } else {
            Rgba([255, 255, 255, 255])
        }
    })
    .save(&input)
    .unwrap();
    let result = run(
        &input,
        dir.path(),
        &Settings {
            max_width: Some(30),
            webp_mode: WebpMode::Lossless,
            ..Default::default()
        },
    );
    let decoded = image::open(result.path).unwrap().to_rgba8();
    assert!(decoded.pixels().any(|p| p[3] > 0 && p[3] < 255));
    for p in decoded.pixels().filter(|p| p[3] > 0) {
        assert!(p[0] > 250 && p[1] > 250 && p[2] > 250);
    }
}

#[test]
fn all_png_engines_produce_decodable_outputs_and_lossless_engines_preserve_pixels() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("test.png");
    let original = png(&input, 64, 32);
    for mode in [PngMode::Lossless, PngMode::Palette, PngMode::Zopfli] {
        let result = run(
            &input,
            dir.path(),
            &Settings {
                format: OutputFormat::Png,
                png_mode: mode,
                ..Default::default()
            },
        );
        let decoded = image::open(result.path).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), original.dimensions());
        if mode != PngMode::Palette {
            assert_eq!(decoded, original);
        } else {
            assert_eq!(decoded.get_pixel(0, 0)[3], 0);
        }
    }
}

#[test]
fn png_16_bit_is_preserved_without_resize() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("16bit.png");
    let pixels =
        image::ImageBuffer::from_fn(8, 4, |x, y| image::Luma([(x * 7000 + y * 1234) as u16]));
    DynamicImage::ImageLuma16(pixels.clone())
        .save(&input)
        .unwrap();
    let result = run(
        &input,
        dir.path(),
        &Settings {
            format: OutputFormat::Original,
            ..Default::default()
        },
    );
    assert_eq!(image::open(result.path).unwrap().to_luma16(), pixels);
}

#[test]
fn jpeg_composites_transparency_on_white() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("transparent.png");
    RgbaImage::from_pixel(32, 32, Rgba([255, 0, 0, 0]))
        .save(&input)
        .unwrap();
    let result = run(
        &input,
        dir.path(),
        &Settings {
            format: OutputFormat::Jpeg,
            ..Default::default()
        },
    );
    let image = image::open(result.path).unwrap().to_rgb8();
    assert!(image.pixels().all(|p| p.0.iter().all(|v| *v >= 250)));
}

#[test]
fn webp_input_and_near_lossless_mode_work() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("test.png");
    png(&input, 100, 50);
    let first = run(&input, dir.path(), &Settings::default());
    let result = run(
        &first.path,
        dir.path(),
        &Settings {
            webp_mode: WebpMode::NearLossless,
            ..Default::default()
        },
    );
    assert_ne!(result.path, first.path);
    assert_eq!(image::open(result.path).unwrap().dimensions(), (100, 50));
}

#[test]
fn name_collisions_never_overwrite_existing_files_or_originals() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("same.png");
    let original = png(&input, 24, 12);
    fs::write(dir.path().join("same.webp"), b"existing").unwrap();
    let one = run(&input, dir.path(), &Settings::default());
    let two = run(&input, dir.path(), &Settings::default());
    assert_eq!(one.path.file_name().unwrap(), "same (1).webp");
    assert_eq!(two.path.file_name().unwrap(), "same (2).webp");
    assert_eq!(fs::read(dir.path().join("same.webp")).unwrap(), b"existing");
    assert_eq!(image::open(input).unwrap().to_rgba8(), original);
}

#[test]
fn concurrent_jobs_reserve_distinct_output_names() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("same.png");
    png(&input, 24, 12);
    let paths = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..4)
            .map(|_| scope.spawn(|| run(&input, dir.path(), &Settings::default()).path))
            .collect();
        threads
            .into_iter()
            .map(|t| t.join().unwrap())
            .collect::<std::collections::HashSet<_>>()
    });
    assert_eq!(paths.len(), 4);
    for path in paths {
        image::open(path).unwrap();
    }
}

#[test]
fn cancelled_and_corrupt_jobs_leave_no_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("broken.png");
    let output = dir.path().join("output");
    fs::write(&input, b"not a png").unwrap();
    assert!(
        optimize(
            &input,
            &output,
            &Settings::default(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    png(&input, 16, 16);
    assert!(
        optimize(
            &input,
            &output,
            &Settings::default(),
            &AtomicBool::new(true)
        )
        .is_err()
    );
    assert!(!output.exists());
}

#[test]
fn animated_png_is_rejected_instead_of_silently_losing_frames() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("animated.png");
    let file = fs::File::create(&input).unwrap();
    let mut encoder = png::Encoder::new(file, 2, 2);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_animated(2, 0).unwrap();
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&[255; 16]).unwrap();
    writer.write_image_data(&[0; 16]).unwrap();
    writer.finish().unwrap();
    let error = optimize(
        &input,
        &dir.path().join("out"),
        &Settings::default(),
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert!(error.to_string().contains("Animowane"));
}

#[test]
fn exif_orientation_is_applied_before_resizing() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("portrait.jpg");
    image::RgbImage::from_pixel(80, 40, image::Rgb([220, 80, 20]))
        .save(&input)
        .unwrap();
    let jpeg = fs::read(&input).unwrap();
    // EXIF little-endian TIFF, orientation = 6 (90 degrees clockwise).
    let exif: &[u8] = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
    let mut oriented = vec![0xff, 0xd8, 0xff, 0xe1];
    oriented.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
    oriented.extend_from_slice(exif);
    oriented.extend_from_slice(&jpeg[2..]);
    fs::write(&input, oriented).unwrap();
    let result = run(
        &input,
        dir.path(),
        &Settings {
            max_width: Some(20),
            ..Default::default()
        },
    );
    assert_eq!(image::open(result.path).unwrap().dimensions(), (20, 40));
}

#[test]
fn srgb_icc_profile_does_not_change_colors_or_alpha() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("profile.png");
    let pixels = RgbaImage::from_pixel(16, 16, Rgba([125, 85, 211, 128]));
    let mut encoder = image::codecs::png::PngEncoder::new(fs::File::create(&input).unwrap());
    encoder
        .set_icc_profile(lcms2::Profile::new_srgb().icc().unwrap())
        .unwrap();
    encoder
        .write_image(pixels.as_raw(), 16, 16, image::ExtendedColorType::Rgba8)
        .unwrap();
    let result = run(
        &input,
        dir.path(),
        &Settings {
            webp_mode: WebpMode::Lossless,
            ..Default::default()
        },
    );
    assert_eq!(image::open(result.path).unwrap().to_rgba8(), pixels);
}

#[test]
fn cli_batch_continues_after_bad_file_and_reports_failure() {
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("broken.png");
    let good = dir.path().join("good.png");
    let output = dir.path().join("out");
    fs::write(&bad, b"broken").unwrap();
    png(&good, 80, 40);
    let command = std::process::Command::new(env!("CARGO_BIN_EXE_image-optimizer"))
        .args(["--batch", "--width", "40", "--output"])
        .arg(&output)
        .arg(&bad)
        .arg(&good)
        .output()
        .unwrap();
    assert!(!command.status.success());
    let stderr = String::from_utf8_lossy(&command.stderr);
    assert!(stderr.contains("broken.png"));
    assert!(stderr.contains("Image format not recognized"), "{stderr}");
    assert!(
        stderr.contains("Some files could not be processed"),
        "{stderr}"
    );
    assert!(String::from_utf8_lossy(&command.stdout).contains("Metadata removed:"));
    assert_eq!(
        image::open(output.join("good.webp")).unwrap().dimensions(),
        (40, 20)
    );
    assert!(!output.join("broken.webp").exists());
}
