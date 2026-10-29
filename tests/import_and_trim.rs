use image::{GenericImageView, Rgba, RgbaImage};
use image_optimizer::optimizer::{
    OutputFormat, Settings, TrimMode, WebpMode, optimize, trim_image,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

fn run(input: &Path, output: &Path, settings: &Settings) -> image_optimizer::optimizer::Outcome {
    optimize(input, output, settings, &AtomicBool::new(false)).unwrap()
}
fn lossless() -> Settings {
    Settings {
        webp_mode: WebpMode::Lossless,
        ..Default::default()
    }
}

#[test]
fn trims_alpha_ignoring_hidden_rgb_and_preserves_visible_pixels() {
    let mut image = RgbaImage::from_pixel(40, 30, Rgba([255, 20, 50, 0]));
    for y in 10..20 {
        for x in 8..32 {
            image.put_pixel(x, y, Rgba([60, 120, 180, 255]));
        }
    }
    let result = trim_image(&image, TrimMode::Transparent, 0);
    assert_eq!(result.dimensions(), (24, 10));
    assert!(result.pixels().all(|p| p.0 == [60, 120, 180, 255]));
}

#[test]
fn uniform_background_trim_keeps_interior_white_and_alpha() {
    let mut image = RgbaImage::from_pixel(40, 30, Rgba([255, 255, 255, 255]));
    for y in 10..20 {
        for x in 8..32 {
            image.put_pixel(x, y, Rgba([5, 10, 20, 255]));
        }
    }
    image.put_pixel(20, 15, Rgba([255, 255, 255, 255]));
    let result = trim_image(&image, TrimMode::Background, 8);
    assert_eq!(result.dimensions(), (24, 10));
    assert_eq!(result.get_pixel(12, 5).0, [255, 255, 255, 255]);
    assert_eq!(trim_image(&image, TrimMode::Transparent, 8), image);
}

#[test]
fn differing_corners_blank_images_and_edge_content_are_safe() {
    let blank = RgbaImage::from_pixel(40, 30, Rgba([255, 255, 255, 255]));
    assert_eq!(trim_image(&blank, TrimMode::Background, 8), blank);
    let blank = RgbaImage::from_pixel(40, 30, Rgba([10, 0, 50, 0]));
    assert_eq!(trim_image(&blank, TrimMode::Transparent, 8), blank);
    let mut image = RgbaImage::from_pixel(40, 30, Rgba([255, 255, 255, 255]));
    image.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
    assert_eq!(trim_image(&image, TrimMode::Background, 8), image);
}

#[test]
fn trim_precedes_resize_and_can_be_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("logo.png");
    let mut pixels = RgbaImage::new(200, 100);
    for y in 25..75 {
        for x in 40..160 {
            pixels.put_pixel(x, y, Rgba([80, 120, 180, 255]));
        }
    }
    pixels.save(&input).unwrap();
    let original = fs::read(&input).unwrap();
    let result = run(
        &input,
        dir.path(),
        &Settings {
            trim: TrimMode::Transparent,
            max_width: Some(60),
            ..lossless()
        },
    );
    assert_eq!((result.width, result.height), (60, 25));
    assert!(
        result
            .notes
            .iter()
            .any(|n| n.contains("Trim: 200 × 100 do 120 × 50"))
    );
    let result = run(
        &input,
        dir.path(),
        &Settings {
            max_width: Some(60),
            ..lossless()
        },
    );
    assert_eq!((result.width, result.height), (60, 30));
    assert_eq!(fs::read(input).unwrap(), original);
}

#[test]
fn tolerance_is_applied_to_alpha_and_background() {
    let mut image = RgbaImage::from_pixel(12, 10, Rgba([255, 255, 255, 255]));
    image.put_pixel(3, 3, Rgba([250, 250, 250, 255]));
    image.put_pixel(6, 6, Rgba([20, 20, 20, 255]));
    assert_eq!(
        trim_image(&image, TrimMode::Background, 0).dimensions(),
        (4, 4)
    );
    assert_eq!(
        trim_image(&image, TrimMode::Background, 8).dimensions(),
        (1, 1)
    );
}

fn make_pdf(path: &Path, streams: &[&str], rotated: bool) {
    let kids = (0..streams.len())
        .map(|i| format!("{} 0 R", 3 + i * 2))
        .collect::<Vec<_>>()
        .join(" ");
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        format!("<< /Type /Pages /Kids [{kids}] /Count {} >>", streams.len()),
    ];
    let font = 3 + streams.len() * 2;
    for (i, stream) in streams.iter().enumerate() {
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << /Font << /F1 {font} 0 R >> >> /Rotate {} /Contents {} 0 R >>", if rotated { 90 } else { 0 }, 4 + i * 2));
        objects.push(format!(
            "<< /Length {} >>\nstream\n{stream}\nendstream",
            stream.len()
        ));
    }
    objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into());
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0];
    for (i, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", i + 1).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
    for offset in &offsets[1..] {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );
    fs::write(path, pdf).unwrap();
}

#[test]
#[cfg(target_os = "macos")]
fn pdf_compatible_ai_renders_transparent_and_upright_then_trims() {
    let dir = tempfile::tempdir().unwrap();
    let ai = dir.path().join("logo.ai");
    make_pdf(&ai, &["1 0 0 rg 50 70 20 10 re f"], false);
    let settings = Settings {
        vector_dpi: 72,
        ..lossless()
    };
    let full = run(&ai, dir.path(), &settings);
    let pixels = image::open(full.path).unwrap().to_rgba8();
    assert_eq!(pixels.dimensions(), (200, 100));
    assert_eq!(pixels.get_pixel(55, 25).0, [255, 0, 0, 255]);
    assert_eq!(pixels.get_pixel(55, 75)[3], 0);
    let result = run(
        &ai,
        dir.path(),
        &Settings {
            trim: TrimMode::Background,
            ..settings
        },
    );
    assert_eq!(image::open(result.path).unwrap().dimensions(), (20, 10));
}

#[test]
#[cfg(target_os = "macos")]
fn pdf_page_selection_rotation_and_dpi_work() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("pages.pdf");
    make_pdf(
        &pdf,
        &["1 0 0 rg 50 70 20 10 re f", "0 0 1 rg 50 70 40 20 re f"],
        true,
    );
    let settings = Settings {
        trim: TrimMode::Transparent,
        vector_dpi: 144,
        page: 2,
        ..lossless()
    };
    let result = run(&pdf, dir.path(), &settings);
    assert_eq!((result.width, result.height), (40, 80));
    assert!(result.notes.iter().any(|n| n.contains("Strona 2 z 2")));
    let error = optimize(
        &pdf,
        dir.path(),
        &Settings {
            page: 3,
            ..settings
        },
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert!(error.to_string().contains("Brak strony"));
}

#[test]
fn unsupported_ai_and_illustrator_placeholder_are_explained() {
    let dir = tempfile::tempdir().unwrap();
    let ai = dir.path().join("legacy.ai");
    fs::write(&ai, b"%!PS-Adobe-3.0").unwrap();
    let error = optimize(&ai, dir.path(), &lossless(), &AtomicBool::new(false)).unwrap_err();
    assert!(error.to_string().contains("Create PDF Compatible File"));
    make_pdf(
        &ai,
        &[
            "BT /F1 12 Tf 10 10 Td (This is an Adobe Illustrator File saved without PDF Content.) Tj ET",
        ],
        false,
    );
    let error = optimize(&ai, dir.path(), &lossless(), &AtomicBool::new(false)).unwrap_err();
    assert!(error.to_string().contains("Create PDF Compatible File"));
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
#[cfg(target_os = "macos")]
fn tiff_and_heic_decode_with_correct_colors_and_orientation() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["colors.tiff", "colors.heic"] {
        let result = run(&fixture(name), dir.path(), &lossless());
        let pixels = image::open(result.path).unwrap().to_rgba8();
        assert_eq!(pixels.dimensions(), (80, 60));
        let top = pixels.get_pixel(20, 10);
        let bottom = pixels.get_pixel(20, 50);
        assert!(top[0] > 200 && top[2] < 60, "{name} top: {top:?}");
        assert!(
            bottom[2] > 200 && bottom[0] < 60,
            "{name} bottom: {bottom:?}"
        );
    }
    let result = run(
        &fixture("rotated.heic"),
        dir.path(),
        &Settings {
            max_width: Some(30),
            ..lossless()
        },
    );
    assert_eq!((result.width, result.height), (30, 40));
    let pixels = image::open(result.path).unwrap().to_rgba8();
    assert!(pixels.get_pixel(5, 20)[2] > 200);
    assert!(pixels.get_pixel(25, 20)[0] > 200);
}

#[test]
#[cfg(target_os = "macos")]
fn tiff_pages_and_original_format_fallback_are_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let result = run(
        &fixture("pages.tiff"),
        dir.path(),
        &Settings {
            page: 2,
            format: OutputFormat::Original,
            ..lossless()
        },
    );
    assert_eq!(result.path.extension().unwrap(), "webp");
    assert!(result.notes.iter().any(|n| n.contains("Strona 2 z 2")));
    assert!(
        result
            .notes
            .iter()
            .any(|n| n.contains("eksportowany do WebP"))
    );
}
