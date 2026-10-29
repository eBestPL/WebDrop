//! EPS is interpreted in a separate Ghostscript process, never in the GUI process.
use anyhow::{Context, Result, bail, ensure};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use std::{
    fs::{self, File},
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

const MAX_PIXELS: u64 = 64_000_000;
const TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) fn postscript(data: &[u8]) -> Result<&[u8]> {
    // DOS EPS stores a PostScript stream followed by optional TIFF/WMF previews.
    let data = if data.starts_with(&[0xc5, 0xd0, 0xd3, 0xc6]) {
        ensure!(data.len() >= 30, "Nieprawidłowy nagłówek binarnego EPS");
        let offset = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
        let length = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
        ensure!(offset >= 30, "Nieprawidłowy nagłówek binarnego EPS");
        data.get(offset..offset.saturating_add(length))
            .context("Nieprawidłowy nagłówek binarnego EPS")?
    } else {
        data
    };
    ensure!(
        data.starts_with(b"%!PS"),
        "Plik nie zawiera poprawnego EPS/PostScript"
    );
    Ok(data)
}

pub(crate) fn dimensions(data: &[u8], dpi: u32) -> Result<(u32, u32)> {
    let mut standard = None;
    let mut precise = None;
    let mut nested = 0_u32;
    for line in data.split(|byte| *byte == b'\n' || *byte == b'\r') {
        if line.starts_with(b"%%BeginDocument") {
            nested += 1;
        } else if line.starts_with(b"%%EndDocument") {
            nested = nested.saturating_sub(1);
        } else if nested == 0 {
            let candidate = line
                .strip_prefix(b"%%HiResBoundingBox:")
                .map(|value| (value, &mut precise))
                .or_else(|| {
                    line.strip_prefix(b"%%BoundingBox:")
                        .map(|value| (value, &mut standard))
                });
            if let Some((value, target)) = candidate {
                let values: Vec<_> = String::from_utf8_lossy(value)
                    .split_whitespace()
                    .map(str::parse::<f64>)
                    .collect();
                if let [Ok(x1), Ok(y1), Ok(x2), Ok(y2)] = values.as_slice() {
                    ensure!(
                        [x1, y1, x2, y2].iter().all(|n| n.is_finite()) && x2 > x1 && y2 > y1,
                        "EPS ma nieprawidłowy obszar BoundingBox"
                    );
                    *target = Some((x2 - x1, y2 - y1));
                }
            }
        }
    }
    let (width, height) = precise
        .or(standard)
        .context("EPS nie określa obszaru BoundingBox")?;
    let width = (width * f64::from(dpi) / 72.0).ceil();
    let height = (height * f64::from(dpi) / 72.0).ceil();
    ensure!(
        width >= 1.0
            && height >= 1.0
            && width <= 65_535.0
            && height <= 65_535.0
            && width * height <= MAX_PIXELS as f64,
        "EPS przekracza limit 64 megapikseli lub 65535 px na bok. Zmniejsz DPI."
    );
    Ok((width as u32, height as u32))
}

pub(crate) fn ghostscript() -> Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        candidates.push(parent.join(if cfg!(windows) { "gswin64c.exe" } else { "gs" }));
    }
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path).filter(|p| p.is_absolute()) {
            for name in if cfg!(windows) {
                &["gswin64c.exe", "gs.exe"][..]
            } else {
                &["gs"][..]
            } {
                candidates.push(directory.join(name));
            }
        }
    }
    #[cfg(target_os = "macos")]
    candidates.extend([
        PathBuf::from("/opt/homebrew/bin/gs"),
        PathBuf::from("/usr/local/bin/gs"),
    ]);
    #[cfg(windows)]
    if let Some(root) = std::env::var_os("ProgramFiles")
        && let Ok(entries) = fs::read_dir(PathBuf::from(root).join("gs"))
    {
        let mut installed: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
        installed.sort();
        candidates.extend(
            installed
                .into_iter()
                .rev()
                .map(|p| p.join("bin/gswin64c.exe")),
        );
    }
    candidates.into_iter().find(|path| path.is_file()).context(
        "EPS wymaga Ghostscript. Zainstaluj aktualny Ghostscript lub użyj pełnej paczki WebDrop dla macOS.",
    )
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn file_size(path: &Path) -> u64 {
    fs::metadata(path).map_or(0, |metadata| metadata.len())
}

fn render(
    program: &Path,
    data: &[u8],
    size: (u32, u32),
    dpi: u32,
    cancel: &AtomicBool,
    timeout: Duration,
) -> Result<Vec<u8>> {
    let temp = tempfile::Builder::new().prefix("webdrop-eps-").tempdir()?;
    let input = temp.path().join("input.eps");
    let output = temp.path().join("render.png");
    let log_path = temp.path().join("ghostscript.log");
    fs::write(&input, data)?;
    let log = File::create(&log_path)?;
    let mut command = Command::new(program);
    command
        .current_dir(temp.path())
        .env_remove("GS_OPTIONS")
        .env_remove("GS_LIB")
        .env_remove("GS_FONTPATH")
        .env_remove("GS_DEVICE")
        .args([
            "-dSAFER",
            "-dBATCH",
            "-dNOPAUSE",
            "-dNOPROMPT",
            "-dEPSCrop",
            "-dFIXEDMEDIA",
            "-sDEVICE=pngalpha",
            "-dTextAlphaBits=4",
            "-dGraphicsAlphaBits=4",
            "-dFirstPage=1",
            "-dLastPage=1",
            "-dNumRenderingThreads=1",
            "-dMaxBitmap=67108864",
        ])
        .arg(format!("-r{dpi}"))
        .arg(format!("-g{}x{}", size.0, size.1))
        .arg(format!("-sOutputFile={}", output.display()))
        .arg("-f")
        .arg(&input)
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = Process(
        command
            .spawn()
            .context("Nie można uruchomić Ghostscript dla EPS")?,
    );
    let started = Instant::now();
    loop {
        ensure!(!cancel.load(Ordering::Relaxed), "Anulowano");
        ensure!(
            started.elapsed() < timeout,
            "Przekroczono limit czasu odczytu EPS (60 s)"
        );
        ensure!(
            file_size(&log_path) <= 1024 * 1024
                && file_size(&output) <= MAX_PIXELS * 5 + 1024 * 1024,
            "Przekroczono limit danych renderowania EPS"
        );
        if let Some(status) = child.0.try_wait()? {
            if !status.success() {
                let mut details = String::new();
                File::open(&log_path)?
                    .take(4096)
                    .read_to_string(&mut details)
                    .ok();
                bail!("Nie można odczytać EPS: {}", details.trim());
            }
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    ensure!(
        file_size(&output) <= MAX_PIXELS * 5 + 1024 * 1024,
        "Przekroczono limit danych renderowania EPS"
    );
    fs::read(output).context("Ghostscript nie utworzył obrazu EPS")
}

pub fn decode(data: &[u8], dpi: u32, page: u32, cancel: &AtomicBool) -> Result<DynamicImage> {
    ensure!(page == 1, "EPS ma jedną stronę. Ustaw numer strony na 1.");
    ensure!(!cancel.load(Ordering::Relaxed), "Anulowano");
    let data = postscript(data)?;
    let size = dimensions(data, dpi)?;
    let bytes = render(&ghostscript()?, data, size, dpi, cancel, TIMEOUT)?;
    let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(512 * 1024 * 1024);
    limits.max_image_width = Some(65_535);
    limits.max_image_height = Some(65_535);
    reader.limits(limits);
    let decoder = reader.into_decoder().context("Nieprawidłowy obraz z EPS")?;
    let (width, height) = decoder.dimensions();
    ensure!(
        u64::from(width) * u64::from(height) <= MAX_PIXELS,
        "Obraz przekracza limit 64 megapikseli"
    );
    DynamicImage::from_decoder(decoder).context("Nieprawidłowy obraz z EPS")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_support_fractional_atend_and_ignore_nested_documents() {
        let data = b"%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: (atend)\n%%BeginDocument: embedded.eps\n%%BoundingBox: 0 0 9000 9000\n%%EndDocument\n%%BoundingBox: -10 20 190 120\n%%HiResBoundingBox: -10.5 20.5 189.5 120.5\n";
        assert_eq!(dimensions(data, 144).unwrap(), (400, 200));
        assert!(dimensions(b"%%BoundingBox: 0 0 99999 99999", 300).is_err());
        assert!(dimensions(b"%%BoundingBox: 0 0 NaN 10", 72).is_err());
        assert!(dimensions(b"%%BoundingBox: 20 0 10 10", 72).is_err());
        assert!(dimensions(b"%%BoundingBox: (atend)", 72).is_err());
    }
    #[test]
    fn dos_eps_offsets_are_checked_and_previews_are_not_interpreted() {
        let ps = b"%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 100 50\n";
        let mut data = vec![0; 30];
        data[..4].copy_from_slice(&[0xc5, 0xd0, 0xd3, 0xc6]);
        data[4..8].copy_from_slice(&30_u32.to_le_bytes());
        data[8..12].copy_from_slice(&(ps.len() as u32).to_le_bytes());
        data.extend(ps);
        data.extend(b"preview data");
        assert_eq!(postscript(&data).unwrap(), ps);
        data[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(postscript(&data).is_err());
        assert!(postscript(b"not EPS").is_err());
    }
    #[test]
    fn interpreter_can_be_cancelled_and_times_out() {
        let program = ghostscript().expect("Install Ghostscript to run EPS tests");
        let infinite = b"%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 10 10\n{} loop\n";
        let timeout = render(
            &program,
            infinite,
            (10, 10),
            72,
            &AtomicBool::new(false),
            Duration::from_millis(150),
        )
        .unwrap_err();
        assert!(timeout.to_string().contains("limit czasu"));
        let cancel = AtomicBool::new(false);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(100));
                cancel.store(true, Ordering::Relaxed);
            });
            assert_eq!(
                render(&program, infinite, (10, 10), 72, &cancel, TIMEOUT)
                    .unwrap_err()
                    .to_string(),
                "Anulowano"
            );
        });
    }
}
