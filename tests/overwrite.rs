use image::{GenericImageView, Rgba, RgbaImage};
use image_optimizer::optimizer::{OutputFormat, Settings, TrimMode, optimize};
use std::{fs, path::Path, sync::atomic::AtomicBool};

fn source(path: &Path) -> Vec<u8> {
    let pixels = RgbaImage::from_fn(80, 40, |x, y| Rgba([x as u8, y as u8, 100, 255]));
    if path.extension().unwrap().eq_ignore_ascii_case("jpeg") {
        image::DynamicImage::ImageRgba8(pixels)
            .to_rgb8()
            .save(path)
            .unwrap();
    } else {
        pixels.save(path).unwrap();
    }
    fs::read(path).unwrap()
}

fn overwrite(format: OutputFormat) -> Settings {
    Settings {
        format,
        overwrite_originals: true,
        max_width: Some(40),
        ..Default::default()
    }
}

#[test]
fn replaces_only_the_source_and_keeps_its_exact_filename() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["Logo.PNG", "Photo.JPEG", "Logo.webp"] {
        let input = dir.path().join(name);
        source(&input);
        let sibling = dir.path().join(format!(
            "{} (1).png",
            input.file_stem().unwrap().to_str().unwrap()
        ));
        fs::write(&sibling, b"unrelated file").unwrap();
        let result = optimize(
            &input,
            &dir.path().join("unused"),
            &overwrite(OutputFormat::Original),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(result.path, input);
        assert_eq!(image::open(&input).unwrap().dimensions(), (40, 20));
        assert_eq!(fs::read(sibling).unwrap(), b"unrelated file");
        assert!(!dir.path().join("unused").exists());
    }
}

#[test]
fn conversion_keeps_source_and_does_not_replace_an_existing_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("logo.png");
    let original = source(&input);
    let existing = dir.path().join("logo.webp");
    fs::write(&existing, b"existing output").unwrap();
    let result = optimize(
        &input,
        dir.path(),
        &overwrite(OutputFormat::Webp),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(result.path.file_name().unwrap(), "logo (1).webp");
    assert_eq!(fs::read(input).unwrap(), original);
    assert_eq!(fs::read(existing).unwrap(), b"existing output");
}

#[test]
fn failure_or_cancellation_leaves_source_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("logo.png");
    let original = source(&input);
    assert!(
        optimize(
            &input,
            dir.path(),
            &overwrite(OutputFormat::Original),
            &AtomicBool::new(true)
        )
        .is_err()
    );
    assert_eq!(fs::read(&input).unwrap(), original);
    fs::write(&input, b"invalid PNG").unwrap();
    assert!(
        optimize(
            &input,
            dir.path(),
            &overwrite(OutputFormat::Original),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert_eq!(fs::read(&input).unwrap(), b"invalid PNG");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn svg_can_be_trimmed_in_place_and_is_otherwise_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("logo.svg");
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect x="20" y="30" width="50" height="40" fill="#aabbcc"/></svg>"##;
    fs::write(&input, svg).unwrap();
    let mut settings = overwrite(OutputFormat::Webp);
    let result = optimize(&input, dir.path(), &settings, &AtomicBool::new(false)).unwrap();
    assert_eq!(result.path, input);
    assert_eq!(fs::read(&input).unwrap(), svg);
    settings.trim = TrimMode::Transparent;
    let result = optimize(&input, dir.path(), &settings, &AtomicBool::new(false)).unwrap();
    assert_eq!(result.path, input);
    assert_ne!(fs::read(&input).unwrap(), svg);
    assert_eq!((result.width, result.height), (50, 40));
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn symlinks_and_read_only_originals_are_not_overwritten() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("logo.png");
    let original = source(&input);
    let link = dir.path().join("link.png");
    symlink(&input, &link).unwrap();
    assert!(
        optimize(
            &link,
            dir.path(),
            &overwrite(OutputFormat::Original),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    fs::set_permissions(&input, fs::Permissions::from_mode(0o444)).unwrap();
    assert!(
        optimize(
            &input,
            dir.path(),
            &overwrite(OutputFormat::Original),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert_eq!(fs::read(&input).unwrap(), original);
    fs::set_permissions(&input, fs::Permissions::from_mode(0o644)).unwrap();
}
