use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

fn bounded(args: &[&std::ffi::OsStr]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_image-optimizer"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if start.elapsed() > Duration::from_secs(3) {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("Input blocked the process instead of being rejected");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(unix)]
#[test]
fn fifo_disguised_as_image_never_waits_for_a_writer() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("image.png");
    let cpath = std::ffi::CString::new(input.as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: NUL-terminated path points into our private temporary directory.
    assert_eq!(unsafe { libc::mkfifo(cpath.as_ptr(), 0o600) }, 0);
    let out = dir.path().join("out");
    let result = bounded(&[
        "--batch".as_ref(),
        "--output".as_ref(),
        out.as_os_str(),
        input.as_os_str(),
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("regular file"));
    assert!(!out.exists());
}

fn worker(input: &Path, output: &Path) -> Output {
    bounded(&[
        "--internal-smart-encode".as_ref(),
        input.as_os_str(),
        output.as_os_str(),
        "webp-lossy".as_ref(),
        "82".as_ref(),
        "normal".as_ref(),
    ])
}

#[test]
fn encoder_worker_cannot_overwrite_existing_files_or_follow_output_links() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.png");
    image::RgbaImage::from_pixel(8, 8, image::Rgba([20, 100, 200, 255]))
        .save(&input)
        .unwrap();
    let source = fs::read(&input).unwrap();
    assert!(!worker(&input, &input).status.success());
    assert_eq!(fs::read(&input).unwrap(), source);
    let output = dir.path().join("existing.webp");
    fs::write(&output, b"keep existing").unwrap();
    assert!(!worker(&input, &output).status.success());
    assert_eq!(fs::read(&output).unwrap(), b"keep existing");
    #[cfg(unix)]
    {
        let alias = dir.path().join("alias.webp");
        std::os::unix::fs::symlink(&output, &alias).unwrap();
        assert!(!worker(&input, &alias).status.success());
        assert_eq!(fs::read(output).unwrap(), b"keep existing");
    }
}

#[test]
fn encoder_rejects_oversized_dimensions_before_decoding_pixel_payload() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("huge.png");
    let output = dir.path().join("out.webp");
    let mut data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut data, 8000, 8001);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut header = encoder.write_header().unwrap();
        header.write_chunk(png::chunk::IDAT, &[0x78, 0x9c]).unwrap();
    }
    fs::write(&input, data).unwrap();
    let result = worker(&input, &output);
    assert!(!result.status.success());
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(error.contains("Smart input exceeds pixel limit"), "{error}");
    assert!(!output.exists());
}
