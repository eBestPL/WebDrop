//! Vector document export uses isolated, bounded helper processes.
use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{ImageDecoder, ImageFormat, ImageReader};
use std::io::Cursor;
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

fn mutool() -> Result<PathBuf> {
    let mut candidates = Vec::new();
    let name = if cfg!(windows) {
        "mutool.exe"
    } else {
        "mutool"
    };
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        candidates.push(parent.join(name));
    }
    if let Some(path) = std::env::var_os("PATH") {
        candidates.extend(
            std::env::split_paths(&path)
                .filter(|p| p.is_absolute())
                .map(|p| p.join(name)),
        );
    }
    #[cfg(target_os = "macos")]
    candidates.extend([
        PathBuf::from("/opt/homebrew/bin/mutool"),
        PathBuf::from("/usr/local/bin/mutool"),
    ]);
    candidates.into_iter().find(|p| p.is_file()).context("Konwersja do SVG wymaga MuPDF (mutool). Użyj pełnej paczki macOS lub zainstaluj MuPDF i dodaj mutool do PATH.")
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn size(path: &Path) -> u64 {
    fs::metadata(path).map_or(0, |m| m.len())
}

fn run(
    command: &mut Command,
    directory: &Path,
    output: &Path,
    cancel: &AtomicBool,
) -> Result<String> {
    ensure!(!cancel.load(Ordering::Relaxed), "Anulowano");
    let log_path = directory.join("conversion.log");
    let log = File::create(&log_path)?;
    command
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = Process(
        command
            .spawn()
            .context("Nie można uruchomić silnika konwersji SVG")?,
    );
    let start = Instant::now();
    loop {
        ensure!(!cancel.load(Ordering::Relaxed), "Anulowano");
        ensure!(
            start.elapsed() < Duration::from_secs(60),
            "Przekroczono limit czasu konwersji SVG (60 s)"
        );
        ensure!(
            size(&log_path) <= 1024 * 1024 && size(output) <= 128 * 1024 * 1024,
            "Przekroczono limit danych konwersji SVG"
        );
        if let Some(status) = child.0.try_wait()? {
            ensure!(
                size(&log_path) <= 1024 * 1024 && size(output) <= 128 * 1024 * 1024,
                "Przekroczono limit danych konwersji SVG"
            );
            // Engine diagnostics are bounded and already in English.
            let log = fs::read_to_string(log_path).unwrap_or_default();
            ensure!(
                status.success(),
                "SVG conversion failed: {}",
                log.chars().take(4096).collect::<String>().trim()
            );
            return Ok(log);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub fn convert(data: &[u8], eps: bool, page: u32, cancel: &AtomicBool) -> Result<Vec<u8>> {
    let engine = mutool()?;
    let temp = tempfile::Builder::new()
        .prefix("webdrop-vector-")
        .tempdir()?;
    let dir = temp.path();
    let pdf = dir.join("input.pdf");
    if eps {
        ensure!(page == 1, "EPS ma jedną stronę. Ustaw numer strony na 1.");
        let data = crate::eps::postscript(data)?;
        crate::eps::dimensions(data, 72)?;
        let source = dir.join("input.eps");
        fs::write(&source, data)?;
        let mut command = Command::new(crate::eps::ghostscript()?);
        command
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
                "-sDEVICE=pdfwrite",
                "-dFirstPage=1",
                "-dLastPage=1",
                "-dAutoRotatePages=/None",
            ])
            .arg(format!("-sOutputFile={}", pdf.display()))
            .arg("-f")
            .arg(source);
        run(&mut command, dir, &pdf, cancel)?;
    } else {
        ensure!(
            data[..data.len().min(1024)]
                .windows(5)
                .any(|s| s == b"%PDF-"),
            "Ten AI nie zawiera PDF (np. starszy PostScript). Zapisz go w Illustratorze z ‘Create PDF Compatible File’ lub wyeksportuj PDF."
        );
        fs::write(&pdf, data)?;
    }
    let mut count_command = Command::new(&engine);
    count_command
        .arg("show")
        .arg(&pdf)
        .arg("trailer/Root/Pages/Count");
    let log = run(&mut count_command, dir, &pdf, cancel)?;
    let pages = log
        .lines()
        .find_map(|s| s.trim().parse::<u32>().ok())
        .context("Nie można odczytać liczby stron PDF")?;
    ensure!(
        page > 0 && page <= pages,
        "Wybrana strona nie istnieje w PDF"
    );
    if !eps {
        let text_path = dir.join("page.txt");
        let mut text_command = Command::new(&engine);
        text_command
            .args(["convert", "-F", "text", "-O", "clip=no", "-o"])
            .arg(&text_path)
            .arg(&pdf)
            .arg(page.to_string());
        run(&mut text_command, dir, &text_path, cancel)?;
        let text: String = fs::read_to_string(&text_path)?
            .chars()
            .filter(char::is_ascii_alphabetic)
            .map(|c| c.to_ascii_lowercase())
            .collect();
        ensure!(
            !text.contains("withoutpdfcontent"),
            "Ten AI nie zawiera PDF (np. starszy PostScript). Zapisz go w Illustratorze z ‘Create PDF Compatible File’ lub wyeksportuj PDF."
        );
    }
    // MuPDF's SVG writer numbers files from 1 even for a selected later page.
    let output = dir.join("output1.svg");
    let mut command = Command::new(engine);
    command
        .args([
            "convert",
            "-F",
            "svg",
            "-O",
            "text=path,no-reuse-images",
            "-o",
        ])
        .arg(dir.join("output%d.svg"))
        .arg(&pdf)
        .arg(page.to_string());
    run(&mut command, dir, &output, cancel)?;
    let svg = fs::read(output).context("Silnik konwersji nie utworzył SVG")?;
    sanitize_generated_images(&svg, cancel)
}

// Only newly generated SVG reaches this function. User-supplied SVG is never rewritten.
fn sanitize_generated_images(data: &[u8], cancel: &AtomicBool) -> Result<Vec<u8>> {
    let source = std::str::from_utf8(data)?;
    let doc = roxmltree::Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            nodes_limit: 100_000,
            ..Default::default()
        },
    )?;
    let mut patches = Vec::new();
    for node in doc
        .descendants()
        .filter(|n| n.has_tag_name(("http://www.w3.org/2000/svg", "image")))
    {
        ensure!(!cancel.load(Ordering::Relaxed), "Anulowano");
        for attr in node.attributes().filter(|a| a.name() == "href") {
            let value = attr.value();
            let (format, encoded) =
                if let Some(encoded) = value.strip_prefix("data:image/jpeg;base64,") {
                    (ImageFormat::Jpeg, encoded)
                } else if let Some(encoded) = value.strip_prefix("data:image/png;base64,") {
                    (ImageFormat::Png, encoded)
                } else {
                    anyhow::bail!("Unsupported embedded image in generated SVG");
                };
            let encoded: String = encoded
                .chars()
                .filter(|c| !c.is_ascii_whitespace())
                .collect();
            let bytes = STANDARD.decode(encoded)?;
            let mut reader = ImageReader::with_format(Cursor::new(&bytes), format);
            let mut limits = image::Limits::default();
            limits.max_alloc = Some(512 * 1024 * 1024);
            limits.max_image_width = Some(65_535);
            limits.max_image_height = Some(65_535);
            reader.limits(limits);
            let mut decoder = reader.into_decoder()?;
            let (w, h) = decoder.dimensions();
            ensure!(
                u64::from(w) * u64::from(h) <= 64_000_000,
                "Obraz przekracza limit 64 megapikseli"
            );
            let icc = decoder.icc_profile()?;
            let (mime, clean) = if let Some(icc) = icc {
                // Preserve color appearance before removing an embedded profile.
                // PDF positioning, not EXIF orientation, defines the image placement.
                let image = image::DynamicImage::from_decoder(decoder)?;
                let rgba = crate::optimizer::to_srgb(&image, Some(&icc))?;
                let mut clean = Cursor::new(Vec::new());
                rgba.write_to(&mut clean, ImageFormat::Png)?;
                ("png", clean.into_inner())
            } else if format == ImageFormat::Jpeg {
                ("jpeg", crate::metadata::strip_jpeg(&bytes)?)
            } else {
                // Keep PNG coding chunks and transparency, discard ancillary metadata.
                let mut clean = bytes[..8].to_vec();
                let mut pos = 8;
                while pos < bytes.len() {
                    ensure!(pos + 12 <= bytes.len(), "Niepełny blok wyniku kodera");
                    let length = u32::from_be_bytes(bytes[pos..pos + 4].try_into()?) as usize;
                    let end = pos
                        .checked_add(length)
                        .and_then(|n| n.checked_add(12))
                        .context("Nieprawidłowy rozmiar bloku")?;
                    ensure!(end <= bytes.len(), "Niepełny blok wyniku kodera");
                    let kind = &bytes[pos + 4..pos + 8];
                    if kind[0].is_ascii_uppercase() || kind == b"tRNS" {
                        clean.extend_from_slice(&bytes[pos..end]);
                    }
                    pos = end;
                }
                ("png", clean)
            };
            patches.push((
                attr.range_value(),
                format!("data:image/{mime};base64,{}", STANDARD.encode(clean)),
            ));
        }
    }
    let mut result = source.to_string();
    for (range, value) in patches.into_iter().rev() {
        ensure!(
            result.len() - range.len() + value.len() <= 128 * 1024 * 1024,
            "Przekroczono limit danych konwersji SVG"
        );
        result.replace_range(range, &value);
    }
    Ok(result.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_jpeg_metadata_is_removed_without_changing_pixels_or_paths() {
        let mut jpeg = Cursor::new(Vec::new());
        image::RgbImage::from_pixel(4, 3, image::Rgb([25, 120, 190]))
            .write_to(&mut jpeg, ImageFormat::Jpeg)
            .unwrap();
        let mut bytes = jpeg.into_inner();
        let mut app = vec![0xff, 0xe1, 0, 14];
        app.extend_from_slice(b"Exif\0\0secret");
        bytes.splice(2..2, app);
        let source = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='100' height='100'><path d='M0 0L100 100'/><image href='data:image/jpeg;base64,{}'/></svg>",
            STANDARD.encode(&bytes)
        );
        let clean = sanitize_generated_images(source.as_bytes(), &AtomicBool::new(false)).unwrap();
        let clean = String::from_utf8(clean).unwrap();
        assert!(clean.contains("<path d='M0 0L100 100'/>"));
        let doc = roxmltree::Document::parse(&clean).unwrap();
        let uri = doc
            .descendants()
            .find(|n| n.has_tag_name(("http://www.w3.org/2000/svg", "image")))
            .unwrap()
            .attribute("href")
            .unwrap();
        let decoded = STANDARD
            .decode(uri.strip_prefix("data:image/jpeg;base64,").unwrap())
            .unwrap();
        assert!(!decoded.windows(6).any(|w| w == b"secret"));
        assert_eq!(
            image::load_from_memory(&decoded).unwrap().to_rgb8(),
            image::load_from_memory(&bytes).unwrap().to_rgb8()
        );
    }
}
