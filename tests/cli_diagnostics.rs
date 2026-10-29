use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn cli(files: &[&Path], output: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_image-optimizer"))
        .env("LANG", "pl_PL.UTF-8")
        .env("LC_ALL", "pl_PL.UTF-8")
        .args(["--batch", "--output"])
        .arg(output)
        .args(extra)
        .args(files)
        .output()
        .unwrap()
}

#[test]
fn missing_inputs_and_missing_files_are_reported_in_english() {
    let dir = tempfile::tempdir().unwrap();
    let no_files = cli(&[], dir.path(), &[]);
    assert!(!no_files.status.success());
    assert!(String::from_utf8_lossy(&no_files.stderr).contains("Add at least one file to --batch"));

    let missing = dir.path().join("Nie można odczytać obrazu {0}.png");
    let result = cli(&[&missing], dir.path(), &[]);
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(!result.status.success());
    assert!(
        stderr.contains(&format!("{}: Cannot open the file:", missing.display())),
        "{stderr}"
    );
    assert!(stderr.contains("Some files could not be processed"));
}

#[test]
fn raster_success_includes_english_trim_metadata_and_savings_notes() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("Zażółć gęślą {0}.png");
    let mut pixels = image::RgbaImage::new(80, 60);
    for y in 10..50 {
        for x in 10..70 {
            pixels.put_pixel(x, y, image::Rgba([30, 80, 190, 255]));
        }
    }
    pixels.save(&source).unwrap();
    let result = cli(
        &[&source],
        &dir.path().join("out"),
        &["--trim", "transparent", "--width", "0"],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert!(stdout.contains(source.to_str().unwrap()));
    assert!(stdout.contains("% saved | libwebp"), "{stdout}");
    assert!(
        stdout.contains("Trim: 80 × 60 to 60 × 40 px (before resizing)"),
        "{stdout}"
    );
    assert!(stdout.contains("Metadata removed: EXIF/GPS"), "{stdout}");
    assert!(!stdout.contains("Metadane usunięte"));
}

#[test]
fn svg_copy_and_nested_trim_errors_use_english() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("znak.svg");
    fs::write(&source, r#"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80"><text x="10" y="20">Logo</text></svg>"#).unwrap();
    let copied = cli(&[&source], &dir.path().join("copy"), &[]);
    assert!(copied.status.success());
    let stdout = String::from_utf8(copied.stdout).unwrap();
    assert!(stdout.contains("SVG · no optimization"), "{stdout}");
    assert!(
        stdout.contains("SVG: byte-for-byte identical copy"),
        "{stdout}"
    );

    let rejected = cli(
        &[&source],
        &dir.path().join("trim"),
        &["--trim", "transparent"],
    );
    assert!(!rejected.status.success());
    let stderr = String::from_utf8(rejected.stderr).unwrap();
    assert!(
        stderr.contains("This SVG cannot be safely trimmed."),
        "{stderr}"
    );
    assert!(
        stderr.contains("Element requires manual trimming: text"),
        "{stderr}"
    );
    assert!(!stderr.contains("Nie można"));
}

#[cfg(target_os = "macos")]
#[test]
fn document_import_notes_use_english() {
    let dir = tempfile::tempdir().unwrap();
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/Optimizer-logo-test.ai");
    let result = cli(&[&source], dir.path(), &["--format", "original"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert!(stdout.contains("AI/PDF rendering: 300 DPI"), "{stdout}");
    assert!(
        stdout.contains("This input format is exported as WebP"),
        "{stdout}"
    );
    assert!(stdout.contains("Metadata removed:"), "{stdout}");
}
