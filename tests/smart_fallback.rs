#![cfg(unix)]
use image_optimizer::optimizer::{OutputFormat, Settings, optimize};
use std::{fs, os::unix::fs::PermissionsExt, sync::atomic::AtomicBool};

#[test]
fn failed_optional_encoders_keep_a_valid_png_and_do_not_touch_the_source() {
    let dir = tempfile::tempdir().unwrap();
    let wrapper = dir.path().join("worker.sh");
    let executable = env!("CARGO_BIN_EXE_image-optimizer").replace('\'', "'\\''");
    fs::write(&wrapper, format!("#!/bin/sh\nif [ \"$1\" = '--internal-smart-encode' ]; then exit 1; fi\nexec '{executable}' \"$@\"\n")).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    image_optimizer::png_worker::configure_executable(wrapper);
    let input = dir.path().join("image.png");
    image::RgbaImage::from_pixel(20, 10, image::Rgba([40, 80, 160, 255]))
        .save(&input)
        .unwrap();
    let source = fs::read(&input).unwrap();
    let output = dir.path().join("out");
    let result = optimize(
        &input,
        &output,
        &Settings {
            format: OutputFormat::Smart,
            ..Default::default()
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(result.path.extension().unwrap(), "png");
    assert_eq!(
        image::open(&result.path).unwrap().to_rgba8(),
        image::open(&input).unwrap().to_rgba8()
    );
    assert!(
        result
            .notes
            .iter()
            .any(|n| n.contains("pominięto niedostępny koder WebP"))
    );
    assert!(
        result
            .notes
            .iter()
            .any(|n| n.contains("pominięto niedostępny koder AVIF"))
    );
    assert_eq!(fs::read_dir(output).unwrap().count(), 1);
    assert_eq!(fs::read(input).unwrap(), source);
}
