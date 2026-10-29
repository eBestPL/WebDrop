use image_optimizer::optimizer::{OutputFormat, Settings, TrimMode, WebpMode, optimize};
use std::{fs, path::Path, sync::atomic::AtomicBool};
fn settings() -> Settings {
    Settings {
        format: OutputFormat::Hybrid,
        max_width: Some(60),
        webp_mode: WebpMode::Lossless,
        ..Default::default()
    }
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
fn mixed_batch_routes_vectors_to_svg_and_rasters_to_webp() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let ai = dir.join("logo.AI");
    fs::write(&ai, include_bytes!("fixtures/Optimizer-logo-test.ai")).unwrap();
    let pdf = dir.join("logo.pdf");
    fs::copy(&ai, &pdf).unwrap();
    let eps = dir.join("logo.eps");
    fs::write(&eps, include_bytes!("fixtures/Optimizer-logo-test.eps")).unwrap();
    let png = dir.join("photo.png");
    image::RgbaImage::from_pixel(200, 100, image::Rgba([20, 120, 180, 255]))
        .save(&png)
        .unwrap();
    for input in [&ai, &pdf, &eps, &png] {
        let before = fs::read(input).unwrap();
        let result = optimize(
            input,
            &dir.join("output"),
            &settings(),
            &AtomicBool::new(false),
        )
        .unwrap();
        if input == &png {
            assert_eq!(result.path.extension().unwrap(), "webp");
            assert_eq!((result.width, result.height), (60, 30));
        } else {
            assert_eq!(result.path.extension().unwrap(), "svg");
            assert!(
                result.width > 60,
                "raster resize must not affect vector geometry"
            );
            let svg = fs::read_to_string(&result.path).unwrap();
            assert!(svg.contains("<path"));
            assert!(!svg.contains("<image"));
            assert!(!svg.contains("<text"));
            assert!(!svg.contains("Synthetic WebDrop EPS logo"));
        }
        assert_eq!(fs::read(input).unwrap(), before);
    }
}

#[test]
fn vector_trim_is_geometry_only_and_preserves_existing_results() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("logo.epsf");
    let eps = include_bytes!("fixtures/Optimizer-logo-test.eps");
    let mut data = vec![0; 30];
    data[..4].copy_from_slice(&[0xc5, 0xd0, 0xd3, 0xc6]);
    data[4..8].copy_from_slice(&30_u32.to_le_bytes());
    data[8..12].copy_from_slice(&(eps.len() as u32).to_le_bytes());
    data.extend(eps);
    fs::write(&input, data).unwrap();
    let plain = optimize(&input, temp.path(), &settings(), &AtomicBool::new(false)).unwrap();
    assert_eq!((plain.width, plain.height), (200, 100));
    let before = fs::read(&plain.path).unwrap();
    let trimmed = optimize(
        &input,
        temp.path(),
        &Settings {
            trim: TrimMode::Background,
            ..settings()
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!((trimmed.width, trimmed.height), (120, 50));
    assert_ne!(trimmed.path, plain.path);
    assert_eq!(fs::read(&plain.path).unwrap(), before);
    let tree = |bytes: &[u8]| usvg::Tree::from_data(bytes, &usvg::Options::default()).unwrap();
    let plain_tree = tree(&before);
    let trimmed_tree = tree(&fs::read(trimmed.path).unwrap());
    let render = |tree: &usvg::Tree| {
        let size = tree.size().to_int_size();
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
        resvg::render(
            tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );
        pixmap
    };
    let plain_pixels = render(&plain_tree);
    let trimmed_pixels = render(&trimmed_tree);
    for y in 0..50 {
        for x in 0..120 {
            assert_eq!(
                plain_pixels.pixel(x + 40, y + 25),
                trimmed_pixels.pixel(x, y)
            );
        }
    }
}

#[test]
fn pdf_page_selection_rotation_invalid_page_and_placeholder() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("pages.pdf");
    make_pdf(
        &input,
        &["1 0 0 rg 20 30 40 50 re f", "0 0 1 rg 40 10 80 20 re f"],
        true,
    );
    let result = optimize(
        &input,
        temp.path(),
        &Settings {
            page: 2,
            ..settings()
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!((result.width, result.height), (100, 200));
    let svg = fs::read_to_string(result.path).unwrap();
    assert!(svg.contains("#0000ff"));
    assert!(!svg.contains("#ff0000"));
    assert!(
        optimize(
            &input,
            temp.path(),
            &Settings {
                page: 3,
                ..settings()
            },
            &AtomicBool::new(false)
        )
        .unwrap_err()
        .to_string()
        .contains("strona")
    );
    make_pdf(
        &input,
        &[
            "BT /F1 12 Tf 10 10 Td (This is an Adobe Illustrator File saved without PDF Content.) Tj ET",
        ],
        false,
    );
    assert!(
        optimize(&input, temp.path(), &settings(), &AtomicBool::new(false))
            .unwrap_err()
            .to_string()
            .contains("Create PDF Compatible File")
    );
}

#[test]
fn hybrid_cli_notes_errors_and_cancellation() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("logo.ai");
    fs::write(&input, include_bytes!("fixtures/Optimizer-logo-test.ai")).unwrap();
    let command = |page: &str| {
        std::process::Command::new(env!("CARGO_BIN_EXE_image-optimizer"))
            .args(["--batch", "--format", "hybrid", "--page", page, "--output"])
            .arg(temp.path().join("out"))
            .arg(&input)
            .output()
            .unwrap()
    };
    let ok = command("1");
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stderr)
    );
    assert!(String::from_utf8_lossy(&ok.stdout).contains("Hybrid mode: SVG export"));
    let bad = command("9");
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("The selected page does not exist"));
    let cancelled = temp.path().join("cancelled");
    assert!(optimize(&input, &cancelled, &settings(), &AtomicBool::new(true)).is_err());
    assert!(!cancelled.exists());
    assert_eq!(OutputFormat::Hybrid.output_label("EPSF"), "SVG");
    assert_eq!(OutputFormat::Hybrid.output_label("jpg"), "WebP");
}
