use anyhow::{Context, Result, bail, ensure};
use image::{DynamicImage, GenericImageView, ImageDecoder, ImageFormat, ImageReader, RgbaImage};
use std::{
    ffi::OsString,
    fs::{self, File},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

const MAX_INPUT_BYTES: u64 = 128 * 1024 * 1024;
const MAX_PIXELS: u64 = 64_000_000;
#[cfg(target_os = "macos")]
pub const INPUT_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "avif", "jxl", "ai", "pdf", "eps", "epsf", "tif", "tiff", "heic",
    "heif", "svg",
];

#[cfg(not(target_os = "macos"))]
pub const INPUT_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "avif", "jxl", "ai", "pdf", "eps", "epsf", "svg",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum TrimMode {
    Off,
    Transparent,
    Background,
}

impl TrimMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Wyłączony",
            Self::Transparent => "Przezroczyste marginesy",
            Self::Background => "Jednolite tło + alfa",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Hybrid,
    Webp,
    Avif,
    Jxl,
    Original,
    Png,
    Jpeg,
}

impl OutputFormat {
    pub fn output_label(self, extension: &str) -> &'static str {
        if extension.eq_ignore_ascii_case("svg")
            || (self == Self::Hybrid
                && ["ai", "eps", "epsf", "pdf"]
                    .iter()
                    .any(|ext| extension.eq_ignore_ascii_case(ext)))
        {
            "SVG"
        } else if self == Self::Hybrid {
            "WebP"
        } else {
            self.label()
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Hybrid => "Hybrydowy · SVG / WebP",
            Self::Webp => "WebP",
            Self::Avif => "AVIF",
            Self::Jxl => "JPEG XL",
            Self::Original => "Oryginalny",
            Self::Png => "PNG",
            Self::Jpeg => "JPG",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum PngMode {
    Auto,
    Lossless,
    Palette,
    Zopfli,
}

impl PngMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto · mniejszy plik",
            Self::Lossless => "OxiPNG · bezstratnie",
            Self::Palette => "Paleta · mniejszy plik",
            Self::Zopfli => "Zopfli · dokładniej, wolniej",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum WebpMode {
    Lossy,
    Lossless,
    NearLossless,
}

impl WebpMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lossy => "Stratny · zdjęcia",
            Self::Lossless => "Bezstratny · grafiki",
            Self::NearLossless => "Prawie bezstratny",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub format: OutputFormat,
    pub overwrite_originals: bool,
    pub max_width: Option<u32>,
    pub quality: u8,
    pub png_mode: PngMode,
    pub webp_mode: WebpMode,
    pub avif_lossless: bool,
    pub jxl_lossless: bool,
    pub thorough: bool,
    pub trim: TrimMode,
    pub trim_tolerance: u8,
    pub vector_dpi: u32,
    pub page: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            format: OutputFormat::Webp,
            overwrite_originals: false,
            max_width: Some(1500),
            quality: 82,
            png_mode: PngMode::Auto,
            webp_mode: WebpMode::Lossy,
            avif_lossless: false,
            jxl_lossless: false,
            thorough: false,
            trim: TrimMode::Off,
            trim_tolerance: 8,
            vector_dpi: 300,
            page: 1,
        }
    }
}

#[derive(Debug)]
pub struct Outcome {
    pub path: PathBuf,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub width: u32,
    pub height: u32,
    pub encoder: &'static str,
    pub notes: Vec<String>,
}

impl Outcome {
    pub fn savings_percent(&self) -> f64 {
        100.0 * (1.0 - self.output_bytes as f64 / self.input_bytes.max(1) as f64)
    }
}

pub fn downloads_dir() -> Result<PathBuf> {
    let dirs = directories::UserDirs::new().context("Nie można znaleźć folderu użytkownika")?;
    Ok(dirs
        .download_dir()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| dirs.home_dir().join("Downloads")))
}

pub fn supported(path: &Path) -> bool {
    path.extension().and_then(|v| v.to_str()).is_some_and(|v| {
        INPUT_EXTENSIONS
            .iter()
            .any(|ext| v.eq_ignore_ascii_case(ext))
    })
}

pub fn target_dimensions(width: u32, height: u32, max_width: Option<u32>) -> (u32, u32) {
    match max_width {
        Some(max) if max > 0 && width > max => (
            max,
            ((height as u64 * max as u64 + width as u64 / 2) / width as u64).max(1) as u32,
        ),
        _ => (width, height),
    }
}

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    ensure!(!cancel.load(Ordering::Relaxed), "Anulowano");
    Ok(())
}

/// A complete job runs off the UI thread. Files are only committed after encoding succeeds.
pub fn optimize(
    input: &Path,
    output_dir: &Path,
    settings: &Settings,
    cancel: &AtomicBool,
) -> Result<Outcome> {
    check_cancel(cancel)?;
    ensure!(
        (1..=100).contains(&settings.quality),
        "Jakość musi być w zakresie 1–100"
    );
    ensure!(
        settings.max_width != Some(0),
        "Szerokość musi być większa od zera"
    );
    ensure!(
        supported(input),
        if cfg!(target_os = "macos") {
            "Obsługiwane formaty: PNG, JPG, WebP, AVIF, JXL, AI/PDF, EPS, TIFF, HEIC/HEIF, SVG (tylko trim)"
        } else {
            "Obsługiwane formaty: PNG, JPG, WebP, AVIF, JXL, AI/PDF, EPS, SVG (tylko trim)"
        }
    );
    ensure!(
        (72..=600).contains(&settings.vector_dpi),
        "DPI musi być w zakresie 72–600"
    );
    ensure!(settings.page > 0, "Numer strony zaczyna się od 1");
    ensure!(
        settings.trim_tolerance <= 64,
        "Tolerancja trim musi być w zakresie 0–64"
    );
    let file = File::open(input).context("Nie można otworzyć pliku")?;
    ensure!(file.metadata()?.is_file(), "Przeciągnij pliki, nie foldery");
    ensure!(
        file.metadata()?.len() <= MAX_INPUT_BYTES,
        "Plik przekracza limit 128 MB"
    );
    let mut data = Vec::new();
    file.take(MAX_INPUT_BYTES + 1).read_to_end(&mut data)?;
    ensure!(
        data.len() as u64 <= MAX_INPUT_BYTES,
        "Plik przekracza limit 128 MB"
    );
    let extension = input
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    // SVG is an unconditional separate path: no raster encoder, optimizer,
    // color conversion, background removal or resize may ever receive it.
    if extension == "svg" {
        let svg = crate::svg::process(&data, settings.trim != TrimMode::Off)?;
        check_cancel(cancel)?;
        let path = save_output(input, output_dir, "svg", &svg.data, &data, settings, cancel)?;
        return Ok(Outcome {
            path,
            input_bytes: data.len() as u64,
            output_bytes: svg.data.len() as u64,
            width: svg.width,
            height: svg.height,
            encoder: "SVG · bez optymalizacji",
            notes: vec![svg.note],
        });
    }
    if settings.format == OutputFormat::Hybrid
        && matches!(extension.as_str(), "ai" | "pdf" | "eps" | "epsf")
    {
        let converted = crate::vector::convert(
            &data,
            matches!(extension.as_str(), "eps" | "epsf"),
            settings.page,
            cancel,
        )?;
        let svg = crate::svg::process(&converted, settings.trim != TrimMode::Off)?;
        check_cancel(cancel)?;
        let path = save_output(input, output_dir, "svg", &svg.data, &data, settings, cancel)?;
        let mut notes = vec!["Tryb hybrydowy: eksport do SVG. Tekst zamieniony na krzywe; szerokość, jakość i DPI nie dotyczą SVG.".into()];
        if settings.trim != TrimMode::Off {
            notes.push(svg.note);
        }
        return Ok(Outcome {
            path,
            input_bytes: data.len() as u64,
            output_bytes: svg.data.len() as u64,
            width: svg.width,
            height: svg.height,
            encoder: "MuPDF · SVG",
            notes,
        });
    }
    let native = ["ai", "pdf", "tif", "tiff", "heic", "heif"].contains(&extension.as_str());
    let mut notes = Vec::new();
    let (img, icc, orientation, input_format) = if matches!(extension.as_str(), "avif" | "jxl") {
        let format = if extension == "avif" {
            OutputFormat::Avif
        } else {
            OutputFormat::Jxl
        };
        let (image, icc) = crate::modern_codecs::decode(&data, format)?;
        notes.push("AVIF/JXL: eksport jako 8-bitowy obraz SDR w sRGB".into());
        (
            image,
            icc,
            image::metadata::Orientation::NoTransforms,
            Some(format),
        )
    } else if matches!(extension.as_str(), "eps" | "epsf") {
        let image = crate::eps::decode(&data, settings.vector_dpi, settings.page, cancel)?;
        notes.push(format!(
            "Rasteryzacja EPS: {} DPI (Ghostscript)",
            settings.vector_dpi
        ));
        if settings.format == OutputFormat::Original {
            notes.push("Ten format wejściowy jest eksportowany do WebP".into());
        }
        (
            image,
            None,
            image::metadata::Orientation::NoTransforms,
            None,
        )
    } else if native {
        let pdf = matches!(extension.as_str(), "ai" | "pdf");
        if pdf {
            ensure!(
                data[..data.len().min(1024)]
                    .windows(5)
                    .any(|s| s == b"%PDF-"),
                "Ten AI nie zawiera PDF (np. starszy PostScript). Zapisz go w Illustratorze z ‘Create PDF Compatible File’ lub wyeksportuj PDF."
            );
        }
        let decoded = crate::native_decode::decode(&data, pdf, settings.vector_dpi, settings.page)?;
        if pdf {
            notes.push(format!("Rasteryzacja AI/PDF: {} DPI", settings.vector_dpi));
        }
        if decoded.pages > 1 {
            notes.push(format!(
                "Strona {} z {} (wybrana w ustawieniach)",
                settings.page, decoded.pages
            ));
        }
        if settings.format == OutputFormat::Original {
            notes.push("Ten format wejściowy jest eksportowany do WebP".into());
        }
        (
            decoded.image,
            None,
            image::metadata::Orientation::NoTransforms,
            None,
        )
    } else {
        let input_format = image::guess_format(&data).context("Nie rozpoznano obrazu")?;
        ensure!(
            matches!(
                input_format,
                ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP
            ),
            "Obsługiwane formaty: PNG, JPG i WebP"
        );
        reject_animation(&data, input_format)?;

        let mut reader = ImageReader::with_format(Cursor::new(&data), input_format);
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(512 * 1024 * 1024);
        limits.max_image_width = Some(65_535);
        limits.max_image_height = Some(65_535);
        reader.limits(limits);
        let mut decoder = reader.into_decoder().context("Nie można odczytać obrazu")?;
        let (w, h) = decoder.dimensions();
        ensure!(
            u64::from(w) * u64::from(h) <= MAX_PIXELS,
            "Obraz przekracza limit 64 megapikseli"
        );
        let orientation = decoder
            .orientation()
            .context("Nieprawidłowe dane orientacji EXIF")?;
        let icc = decoder
            .icc_profile()
            .context("Nieprawidłowy profil kolorów")?;
        let mut img = DynamicImage::from_decoder(decoder).context("Nie można zdekodować obrazu")?;
        img.apply_orientation(orientation);
        let icc = if input_format == ImageFormat::Png {
            crate::metadata::png_profile(&data, icc)?
        } else {
            icc
        };
        (
            img,
            icc,
            orientation,
            Some(match input_format {
                ImageFormat::Png => OutputFormat::Png,
                ImageFormat::Jpeg => OutputFormat::Jpeg,
                _ => OutputFormat::Webp,
            }),
        )
    };
    let original_dimensions = img.dimensions();
    let prepared = if settings.trim != TrimMode::Off {
        let rgba = to_srgb(&img, icc.as_deref())?;
        Some(trim_image(&rgba, settings.trim, settings.trim_tolerance))
    } else {
        None
    };
    let content_dimensions = prepared
        .as_ref()
        .map_or(original_dimensions, |rgba| rgba.dimensions());
    if content_dimensions != original_dimensions {
        notes.push(format!(
            "Trim: {} × {} do {} × {} px (przed skalowaniem)",
            original_dimensions.0,
            original_dimensions.1,
            content_dimensions.0,
            content_dimensions.1
        ));
    } else if settings.trim != TrimMode::Off {
        notes.push("Trim: brak pustych marginesów do usunięcia".into());
    }
    let dimensions = target_dimensions(
        content_dimensions.0,
        content_dimensions.1,
        settings.max_width,
    );
    let resized = dimensions != content_dimensions;
    let format = match settings.format {
        OutputFormat::Hybrid => OutputFormat::Webp,
        OutputFormat::Original => input_format.unwrap_or(OutputFormat::Webp),
        format => format,
    };
    if format == OutputFormat::Webp {
        ensure!(
            dimensions.0 <= 16_383 && dimensions.1 <= 16_383,
            "WebP obsługuje maksymalnie 16383 px na bok. Wybierz mniejszy rozmiar."
        );
    }
    check_cancel(cancel)?;

    // Only untagged/already-sRGB pixels may bypass color conversion. All optional
    // chunks are removed even when no resize or other pixel changes are requested.
    let direct_png = format == OutputFormat::Png
        && input_format == Some(OutputFormat::Png)
        && settings.trim == TrimMode::Off
        && !resized
        && orientation == image::metadata::Orientation::NoTransforms
        && icc.is_none()
        && settings.png_mode != PngMode::Palette;
    let (mut encoded, mut encoder) = if direct_png {
        optimize_png(&data, settings, cancel, &mut notes)?
    } else if format == OutputFormat::Png
        && !resized
        && settings.trim == TrimMode::Off
        && settings.png_mode != PngMode::Palette
        && matches!(
            img.color(),
            image::ColorType::L16
                | image::ColorType::La16
                | image::ColorType::Rgb16
                | image::ColorType::Rgba16
        )
    {
        let rgba = crate::metadata::srgb16(&img, icc.as_deref())?;
        let mut png = Cursor::new(Vec::new());
        DynamicImage::ImageRgba16(rgba).write_to(&mut png, ImageFormat::Png)?;
        optimize_png(&png.into_inner(), settings, cancel, &mut notes)?
    } else {
        let mut rgba = match prepared {
            Some(rgba) => rgba,
            None => to_srgb(&img, icc.as_deref())?,
        };
        // Drop the full-resolution decoded source before allocating resize buffers.
        drop(img);
        if resized {
            rgba = resize_rgba(&rgba, dimensions.0, dimensions.1);
        }
        check_cancel(cancel)?;
        match format {
            OutputFormat::Webp => (encode_webp(&rgba, settings)?, "libwebp"),
            OutputFormat::Jpeg => (encode_jpeg(&rgba, settings)?, "MozJPEG"),
            OutputFormat::Png => {
                let png = if settings.png_mode == PngMode::Palette {
                    encode_palette(&rgba, settings)?
                } else {
                    let mut buf = Cursor::new(Vec::new());
                    DynamicImage::ImageRgba8(rgba).write_to(&mut buf, ImageFormat::Png)?;
                    buf.into_inner()
                };
                optimize_png(&png, settings, cancel, &mut notes)?
            }
            OutputFormat::Avif => (
                crate::modern_codecs::encode(&rgba, format, settings)?,
                "libavif · AOM",
            ),
            OutputFormat::Jxl => (
                crate::modern_codecs::encode(&rgba, format, settings)?,
                "libjxl",
            ),
            _ => unreachable!(),
        }
    };
    if format == OutputFormat::Png && settings.png_mode == PngMode::Auto {
        (encoded, encoder) = auto_png(encoded, settings, cancel, &mut notes)?;
    }
    check_cancel(cancel)?;
    if format == OutputFormat::Png || format == OutputFormat::Webp {
        crate::metadata::verify_chunks(&encoded, format == OutputFormat::Webp)?;
    }
    notes.push("Metadane usunięte: EXIF/GPS, XMP/IPTC, komentarze i profile. Orientacja i kolory zastosowane przed zapisem.".into());
    let extension = match format {
        OutputFormat::Png => "png",
        OutputFormat::Jpeg => "jpg",
        OutputFormat::Avif => "avif",
        OutputFormat::Jxl => "jxl",
        _ => "webp",
    };
    let output_bytes = encoded.len() as u64;
    let path = save_output(
        input, output_dir, extension, &encoded, &data, settings, cancel,
    )?;
    Ok(Outcome {
        path,
        input_bytes: data.len() as u64,
        output_bytes,
        width: dimensions.0,
        height: dimensions.1,
        encoder,
        notes,
    })
}

/// Crop only outer margins; matching colors inside the content remain untouched.
/// Uniform-background mode is enabled only if all four visible corners agree.
pub fn trim_image(img: &RgbaImage, mode: TrimMode, tolerance: u8) -> RgbaImage {
    if mode == TrimMode::Off || img.width() == 0 || img.height() == 0 {
        return img.clone();
    }
    let (w, h) = img.dimensions();
    let corners = [
        img.get_pixel(0, 0),
        img.get_pixel(w - 1, 0),
        img.get_pixel(0, h - 1),
        img.get_pixel(w - 1, h - 1),
    ];
    let background = if mode == TrimMode::Background
        && corners.iter().all(|p| p[3] > tolerance)
        && corners
            .iter()
            .all(|p| (0..4).all(|c| p[c].abs_diff(corners[0][c]) <= tolerance))
    {
        Some(corners[0])
    } else {
        None
    };
    let (mut left, mut top, mut right, mut bottom) = (w, h, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        let empty = p[3] <= tolerance
            || background.is_some_and(|bg| (0..4).all(|c| p[c].abs_diff(bg[c]) <= tolerance));
        if !empty {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
    }
    // An entirely blank image has no meaningful content boundary; keep it intact.
    if left == w || (left == 0 && top == 0 && right == w - 1 && bottom == h - 1) {
        return img.clone();
    }
    image::imageops::crop_imm(img, left, top, right - left + 1, bottom - top + 1).to_image()
}

fn png_encoder_name(mode: PngMode) -> &'static str {
    match mode {
        PngMode::Auto => "OxiPNG",
        PngMode::Lossless => "OxiPNG",
        PngMode::Palette => "imagequant + OxiPNG",
        PngMode::Zopfli => "OxiPNG + Zopfli",
    }
}

fn optimize_png(
    data: &[u8],
    settings: &Settings,
    cancel: &AtomicBool,
    notes: &mut Vec<String>,
) -> Result<(Vec<u8>, &'static str)> {
    let options = |level| {
        let mut options = oxipng::Options::from_preset(level);
        options.strip = oxipng::StripChunks::All;
        options.max_decompressed_size = Some(512 * 1024 * 1024);
        // Preserve even hidden RGB values and 16-bit precision in lossless modes.
        options
    };
    let compress = |input: &[u8], options: &oxipng::Options| {
        check_cancel(cancel)?;
        oxipng::optimize_from_memory(input, options).context("Błąd OxiPNG")
    };

    // A fast estimate can choose the wrong filter for flat graphics. Compare it
    // with full libdeflate trials; higher presets are not always smaller.
    let mut best = compress(data, &options(3))?;
    let strong = compress(data, &options(if settings.thorough { 6 } else { 5 }))?;
    if strong.len() < best.len() {
        best = strong;
    }
    if settings.png_mode == PngMode::Zopfli {
        return zopfli_png(best, settings, cancel, notes);
    }
    check_cancel(cancel)?;
    Ok((best, png_encoder_name(settings.png_mode)))
}

fn zopfli_png(
    mut best: Vec<u8>,
    settings: &Settings,
    cancel: &AtomicBool,
    notes: &mut Vec<String>,
) -> Result<(Vec<u8>, &'static str)> {
    let mut encoder = "OxiPNG + Zopfli";
    match crate::png_worker::optimize(&best, settings.thorough, cancel) {
        Ok(crate::png_worker::Attempt::Finished(candidate)) => {
            if candidate.len() < best.len() {
                best = candidate;
            }
        }
        Ok(crate::png_worker::Attempt::TimedOut) => {
            encoder = "OxiPNG";
            notes.push(
                "Zopfli osiągnął limit czasu. Zapisano gotowy wynik OxiPNG bez utraty jakości."
                    .into(),
            );
        }
        Err(_) => {
            check_cancel(cancel)?;
            encoder = "OxiPNG";
            notes.push("Zopfli nie zakończył się poprawnie. Zapisano gotowy wynik OxiPNG bez utraty jakości.".into());
        }
    }
    check_cancel(cancel)?;
    Ok((best, encoder))
}

/// Compare encodings of exactly the same resized/trimmed image. Never relax the
/// requested width. Zopfli runs only once, after choosing the smaller candidate.
fn auto_png(
    mut best: Vec<u8>,
    settings: &Settings,
    cancel: &AtomicBool,
    notes: &mut Vec<String>,
) -> Result<(Vec<u8>, &'static str)> {
    check_cancel(cancel)?;
    let img = image::load_from_memory_with_format(&best, ImageFormat::Png)?;
    let mut palette_selected = false;
    // Keep a 16-bit lossless source at full precision. Resizing already produces 8-bit pixels.
    if img.color().bits_per_pixel() / u16::from(img.color().channel_count()) <= 8 {
        // Target the quality floor as well: targeting 100 retains many near-identical
        // edge colors and needlessly inflates resized logos. This is imagequant's
        // perceptual score, not a percentage of the original image's quality.
        let palette = encode_palette_quality(&img.to_rgba8(), settings, 95, 95);
        check_cancel(cancel)?;
        match palette {
            Ok(palette) => {
                let mut palette_settings = settings.clone();
                palette_settings.png_mode = PngMode::Palette;
                let (candidate, _) = optimize_png(&palette, &palette_settings, cancel, notes)?;
                if candidate.len() < best.len() {
                    best = candidate;
                    palette_selected = true;
                }
            }
            Err(error)
                if matches!(
                    error.downcast_ref::<imagequant::Error>(),
                    Some(imagequant::Error::QualityTooLow)
                ) => {}
            Err(error) => return Err(error),
        }
    }
    drop(img);
    let (best, encoder) = zopfli_png(best, settings, cancel, notes)?;
    if palette_selected {
        notes.push("Auto PNG: wybrano mniejszą paletę przy wysokiej jakości. Kolory mogą się minimalnie różnić; wybrana szerokość została zachowana.".into());
        Ok((
            best,
            if encoder == "OxiPNG + Zopfli" {
                "imagequant + OxiPNG + Zopfli"
            } else {
                "imagequant + OxiPNG"
            },
        ))
    } else {
        notes.push("Auto PNG: wybrano kompresję bezstratną. Paleta nie dała mniejszego pliku przy wymaganej jakości.".into());
        Ok((best, encoder))
    }
}

fn encode_webp(img: &RgbaImage, settings: &Settings) -> Result<Vec<u8>> {
    let mut config = crate::webp_encoder::config()?;
    config.method = if settings.thorough { 6 } else { 4 };
    config.thread_level = 1;
    config.alpha_quality = 100;
    config.exact = 1;
    config.use_sharp_yuv = 1;
    match settings.webp_mode {
        WebpMode::Lossy => {
            config.quality = settings.quality as f32;
        }
        WebpMode::Lossless => {
            config.lossless = 1;
            config.quality = if settings.thorough { 100.0 } else { 75.0 };
        }
        WebpMode::NearLossless => {
            config.lossless = 1;
            config.quality = 90.0;
            config.near_lossless = settings.quality as i32;
        }
    }
    crate::webp_encoder::encode(img, &config)
}

fn encode_jpeg(img: &RgbaImage, settings: &Settings) -> Result<Vec<u8>> {
    // JPEG has no alpha channel; composite transparent pixels on white.
    let rgb: Vec<u8> = img
        .pixels()
        .flat_map(|p| {
            let a = u32::from(p[3]);
            [0, 1, 2].map(|c| ((u32::from(p[c]) * a + 255 * (255 - a) + 127) / 255) as u8)
        })
        .collect();
    let mut compressor = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    compressor.set_size(img.width() as usize, img.height() as usize);
    compressor.set_quality(settings.quality as f32);
    compressor.set_progressive_mode();
    compressor.set_optimize_coding(true);
    compressor.set_optimize_scans(true);
    let mut writer = compressor.start_compress(Vec::new())?;
    writer.write_scanlines(&rgb)?;
    crate::metadata::strip_jpeg(&writer.finish().context("Błąd kodera MozJPEG")?)
}

fn encode_palette(img: &RgbaImage, settings: &Settings) -> Result<Vec<u8>> {
    encode_palette_quality(img, settings, 0, settings.quality)
}

fn encode_palette_quality(
    img: &RgbaImage,
    settings: &Settings,
    minimum: u8,
    target: u8,
) -> Result<Vec<u8>> {
    let mut attributes = imagequant::new();
    attributes.set_speed(if settings.thorough { 1 } else { 4 })?;
    attributes.set_quality(minimum, target)?;
    let pixels: Vec<imagequant::RGBA> = img
        .pixels()
        .map(|p| imagequant::RGBA::new(p[0], p[1], p[2], p[3]))
        .collect();
    let mut image =
        attributes.new_image(pixels, img.width() as usize, img.height() as usize, 0.0)?;
    let mut result = attributes.quantize(&mut image)?;
    result.set_dithering_level(1.0)?;
    let (palette, indices) = result.remapped(&mut image)?;
    if minimum > 0 && result.remapping_quality().unwrap_or(0) < minimum {
        return Err(imagequant::Error::QualityTooLow.into());
    }
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, img.width(), img.height());
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_palette(
            palette
                .iter()
                .flat_map(|p| [p.r, p.g, p.b])
                .collect::<Vec<_>>(),
        );
        encoder.set_trns(palette.iter().map(|p| p.a).collect::<Vec<_>>());
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&indices)?;
        writer.finish()?;
    }
    Ok(bytes)
}

/// Convert tagged RGB/gray images into web sRGB before stripping metadata.
pub(crate) fn to_srgb(img: &DynamicImage, icc: Option<&[u8]>) -> Result<RgbaImage> {
    let mut rgba = img.to_rgba8();
    if let Some(icc) = icc {
        let source = lcms2::Profile::new_icc(icc).context("Nieprawidłowy profil ICC")?;
        let target = lcms2::Profile::new_srgb();
        let mut colors: Vec<[u8; 3]> = rgba.pixels().map(|p| [p[0], p[1], p[2]]).collect();
        match source.color_space() {
            lcms2::ColorSpaceSignature::RgbData => {
                let transform = lcms2::Transform::<[u8; 3], [u8; 3]>::new(
                    &source,
                    lcms2::PixelFormat::RGB_8,
                    &target,
                    lcms2::PixelFormat::RGB_8,
                    lcms2::Intent::Perceptual,
                )?;
                transform.transform_in_place(&mut colors);
            }
            lcms2::ColorSpaceSignature::GrayData => {
                let gray: Vec<u8> = img.to_luma8().into_raw();
                let transform = lcms2::Transform::<u8, [u8; 3]>::new(
                    &source,
                    lcms2::PixelFormat::GRAY_8,
                    &target,
                    lcms2::PixelFormat::RGB_8,
                    lcms2::Intent::Perceptual,
                )?;
                transform.transform_pixels(&gray, &mut colors);
            }
            _ => bail!(
                "Ten profil ICC nie jest RGB ani grayscale. Wyeksportuj obraz do sRGB przed konwersją."
            ),
        }
        for (pixel, color) in rgba.pixels_mut().zip(colors) {
            pixel.0[..3].copy_from_slice(&color);
        }
    }
    Ok(rgba)
}

/// Lanczos3 in linear light with premultiplied alpha avoids dark transparency fringes.
fn resize_rgba(img: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    fn linear(v: u8) -> f32 {
        let v = v as f32 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    fn srgb(v: f32) -> u8 {
        let v = v.clamp(0.0, 1.0);
        let v = if v <= 0.0031308 {
            12.92 * v
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        (v * 255.0).round() as u8
    }
    let linear_image = image::Rgba32FImage::from_fn(img.width(), img.height(), |x, y| {
        let p = img.get_pixel(x, y);
        let a = p[3] as f32 / 255.0;
        image::Rgba([linear(p[0]) * a, linear(p[1]) * a, linear(p[2]) * a, a])
    });
    let resized = image::imageops::resize(
        &linear_image,
        width,
        height,
        image::imageops::FilterType::Lanczos3,
    );
    RgbaImage::from_fn(width, height, |x, y| {
        let p = resized.get_pixel(x, y);
        let a = p[3].clamp(0.0, 1.0);
        if a < 0.0001 {
            image::Rgba([0, 0, 0, 0])
        } else {
            image::Rgba([
                srgb(p[0] / a),
                srgb(p[1] / a),
                srgb(p[2] / a),
                (a * 255.0).round() as u8,
            ])
        }
    })
}

fn reject_animation(data: &[u8], format: ImageFormat) -> Result<()> {
    // Read container chunks rather than searching compressed payloads for magic strings.
    let mut offset: usize = match format {
        ImageFormat::Png => 8,
        ImageFormat::WebP => 12,
        _ => return Ok(()),
    };
    while offset.saturating_add(8) <= data.len() {
        let (size, kind, overhead) = if format == ImageFormat::Png {
            (
                u32::from_be_bytes(data[offset..offset + 4].try_into()?) as usize,
                &data[offset + 4..offset + 8],
                12,
            )
        } else {
            (
                u32::from_le_bytes(data[offset + 4..offset + 8].try_into()?) as usize,
                &data[offset..offset + 4],
                8,
            )
        };
        ensure!(
            kind != b"acTL" && kind != b"ANIM" && kind != b"ANMF",
            "Animowane PNG/WebP nie są obsługiwane; animacja nie została zmieniona"
        );
        let padding = if format == ImageFormat::WebP {
            size % 2
        } else {
            0
        };
        offset = offset
            .checked_add(size)
            .and_then(|v| v.checked_add(overhead + padding))
            .context("Nieprawidłowy rozmiar obrazu")?;
    }
    Ok(())
}

fn save_output(
    input: &Path,
    output_dir: &Path,
    extension: &str,
    encoded: &[u8],
    original: &[u8],
    settings: &Settings,
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    let source_extension = input
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    let same_format =
        source_extension == extension || (extension == "jpg" && source_extension == "jpeg");
    if !settings.overwrite_originals || !same_format {
        return save_unique(input, output_dir, extension, encoded, cancel);
    }

    // Stage the complete result beside the original: a same-filesystem rename
    // replaces it only after encoding and writing succeed, never by truncation.
    let metadata = fs::symlink_metadata(input)?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "Nadpisywanie dowiązań symbolicznych nie jest obsługiwane"
    );
    ensure!(
        !metadata.permissions().readonly(),
        "Oryginał jest tylko do odczytu"
    );
    let parent = input
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).context("Brak dostępu do folderu wynikowego")?;
    temporary
        .write_all(encoded)
        .context("Nie można zapisać obrazu")?;
    temporary
        .as_file()
        .set_permissions(metadata.permissions())?;
    temporary.as_file().sync_all()?;
    check_cancel(cancel)?;
    // Do not silently discard edits made while a slow encoder was running.
    let current_metadata = fs::symlink_metadata(input)?;
    ensure!(
        current_metadata.is_file() && !current_metadata.file_type().is_symlink(),
        "Oryginał zmienił się podczas konwersji. Dodaj plik ponownie."
    );
    let mut current = Vec::new();
    File::open(input)?
        .take(original.len() as u64 + 1)
        .read_to_end(&mut current)?;
    ensure!(
        current == original,
        "Oryginał zmienił się podczas konwersji. Dodaj plik ponownie."
    );
    check_cancel(cancel)?;
    temporary
        .persist(input)
        .map_err(|error| error.error)
        .context("Nie można nadpisać oryginału")?;
    Ok(input.to_path_buf())
}

fn save_unique(
    input: &Path,
    output_dir: &Path,
    extension: &str,
    data: &[u8],
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    fs::create_dir_all(output_dir).context("Nie można utworzyć folderu wynikowego")?;
    let mut temporary = tempfile::NamedTempFile::new_in(output_dir)
        .context("Brak dostępu do folderu wynikowego")?;
    temporary
        .write_all(data)
        .context("Nie można zapisać obrazu")?;
    temporary.as_file().sync_all()?;
    let stem = input.file_stem().context("Plik nie ma nazwy")?;
    for n in 0..10_000 {
        check_cancel(cancel)?;
        let mut name = OsString::from(stem);
        if n > 0 {
            name.push(format!(" ({n})"));
        }
        name.push(format!(".{extension}"));
        let path = output_dir.join(name);
        // Atomic no-clobber persistence protects originals and concurrent app instances.
        match temporary.persist_noclobber(&path) {
            Ok(_) => return Ok(path),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                temporary = error.file;
            }
            Err(error) => return Err(error.error).context("Nie można zapisać pliku wynikowego"),
        }
    }
    bail!("Zbyt wiele plików z tą samą nazwą")
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1000 {
        format!("{:.0} KB", bytes as f64 / 1000.0)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod overwrite_tests {
    use super::*;

    #[test]
    fn stale_source_and_cancelled_commit_are_preserved_without_temporary_files() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("source.png");
        fs::write(&input, b"edited source").unwrap();
        let settings = Settings {
            overwrite_originals: true,
            ..Default::default()
        };
        let stale = save_output(
            &input,
            dir.path(),
            "png",
            b"encoded result",
            b"old source",
            &settings,
            &AtomicBool::new(false),
        );
        assert!(
            stale
                .unwrap_err()
                .to_string()
                .contains("Oryginał zmienił się")
        );
        let cancelled = save_output(
            &input,
            dir.path(),
            "png",
            b"encoded result",
            b"edited source",
            &settings,
            &AtomicBool::new(true),
        );
        assert!(cancelled.is_err());
        assert_eq!(fs::read(&input).unwrap(), b"edited source");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
