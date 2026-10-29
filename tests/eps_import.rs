use image_optimizer::optimizer::{OutputFormat, Settings, TrimMode, WebpMode, optimize, supported};
use std::{fs, path::Path, sync::atomic::AtomicBool};

const EPS: &[u8] = include_bytes!("fixtures/Optimizer-logo-test.eps");

fn settings() -> Settings {
    Settings {
        vector_dpi: 72,
        max_width: None,
        webp_mode: WebpMode::Lossless,
        ..Default::default()
    }
}

#[test]
fn eps_preserves_transparency_and_trims_before_resizing_without_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("logo żółte.EPS");
    fs::write(&input, EPS).unwrap();
    assert!(supported(&input));
    assert!(supported(Path::new("logo.epsf")));
    let plain = optimize(&input, dir.path(), &settings(), &AtomicBool::new(false)).unwrap();
    assert_eq!((plain.width, plain.height), (200, 100));
    let image = image::open(&plain.path).unwrap().to_rgba8();
    assert_eq!(image.get_pixel(0, 0).0[3], 0);
    assert_eq!(image.get_pixel(45, 30).0[3], 255);
    let result = optimize(
        &input,
        dir.path(),
        &Settings {
            trim: TrimMode::Background,
            max_width: Some(60),
            format: OutputFormat::Original,
            ..settings()
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!((result.width, result.height), (60, 25));
    assert_eq!(result.path.extension().unwrap(), "webp");
    assert!(result.notes.iter().any(|n| n.contains("EPS: 72 DPI")));
    let data = fs::read(result.path).unwrap();
    for marker in [b"EXIF", b"XMP ", b"ICCP"] {
        assert!(!data.windows(4).any(|w| w == marker));
    }
    assert_eq!(fs::read(input).unwrap(), EPS);
}

#[test]
fn binary_eps_and_dpi_are_supported_and_invalid_page_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("binary.epsf");
    let mut data = vec![0; 30];
    data[..4].copy_from_slice(&[0xc5, 0xd0, 0xd3, 0xc6]);
    data[4..8].copy_from_slice(&30_u32.to_le_bytes());
    data[8..12].copy_from_slice(&(EPS.len() as u32).to_le_bytes());
    data.extend(EPS);
    data.extend(b"fake TIFF preview");
    fs::write(&input, data).unwrap();
    let result = optimize(
        &input,
        dir.path(),
        &Settings {
            format: OutputFormat::Png,
            vector_dpi: 144,
            ..settings()
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!((result.width, result.height), (400, 200));
    assert_eq!(result.path.extension().unwrap(), "png");
    let error = optimize(
        &input,
        dir.path(),
        &Settings {
            page: 2,
            ..settings()
        },
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert!(error.to_string().contains("jedną stronę"));
}

#[test]
fn malformed_oversized_and_external_file_access_do_not_produce_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("bad.eps");
    let output = dir.path().join("results");
    let private = dir.path().join("not-an-image.txt");
    fs::write(&private, b"test fixture").unwrap();
    for data in [
        b"not EPS".to_vec(),
        b"%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 90000 90000\nshowpage".to_vec(),
        b"%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 100 100\nnot_a_postscript_operator\n".to_vec(),
        format!("%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 100 100\n({}) (r) file closefile\nshowpage\n", private.display()).into_bytes(),
    ] {
        fs::write(&input, data).unwrap();
        assert!(optimize(&input, &output, &settings(), &AtomicBool::new(false)).is_err());
        assert!(!output.exists());
    }
}

#[test]
fn eps_cli_uses_english_notes_and_errors() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("logo.eps");
    fs::write(&input, EPS).unwrap();
    let command = |page: &str| {
        std::process::Command::new(env!("CARGO_BIN_EXE_image-optimizer"))
            .args(["--batch", "--vector-dpi", "72", "--page", page, "--output"])
            .arg(dir.path().join("output"))
            .arg(&input)
            .output()
            .unwrap()
    };
    let result = command("1");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8_lossy(&result.stdout).contains("EPS rendering: 72 DPI (Ghostscript)")
    );
    let error = command("2");
    assert!(!error.status.success());
    assert!(String::from_utf8_lossy(&error.stderr).contains("EPS has one page"));
}
