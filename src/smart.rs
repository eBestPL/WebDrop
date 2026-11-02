//! Compare actual encoded files at identical dimensions; only the winner is saved.
use super::*;
use image::ImageEncoder;

pub(super) struct Selection {
    pub bytes: Vec<u8>,
    pub format: OutputFormat,
    pub encoder: &'static str,
    pub notes: Vec<String>,
}

struct Candidates {
    best: Selection,
    sizes: Vec<(OutputFormat, usize)>,
    skipped: Vec<OutputFormat>,
}

impl Candidates {
    fn consider(
        &mut self,
        result: Result<Vec<u8>>,
        format: OutputFormat,
        encoder: &'static str,
        notes: &[String],
        cancel: &AtomicBool,
    ) -> Result<()> {
        check_cancel(cancel)?;
        let bytes = match result {
            Ok(bytes) => bytes,
            Err(_) => {
                if !self.skipped.contains(&format) {
                    self.skipped.push(format);
                }
                return Ok(());
            }
        };
        if let Some((_, size)) = self.sizes.iter_mut().find(|(f, _)| *f == format) {
            *size = (*size).min(bytes.len());
        } else {
            self.sizes.push((format, bytes.len()));
        }
        if bytes.len() < self.best.bytes.len() {
            self.best = Selection {
                bytes,
                format,
                encoder,
                notes: notes.to_vec(),
            };
        }
        Ok(())
    }
}

pub(super) fn encode(
    rgba: &RgbaImage,
    settings: &Settings,
    cancel: &AtomicBool,
) -> Result<Selection> {
    check_cancel(cancel)?;
    let mut config = settings.clone();
    config.png_mode = PngMode::Auto;
    let mut source_png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut source_png).write_image(
        rgba.as_raw(),
        rgba.width(),
        rgba.height(),
        image::ExtendedColorType::Rgba8,
    )?;
    let mut png_notes = Vec::new();
    let (png, _) = optimize_png(&source_png, &config, cancel, &mut png_notes)?;
    let (png, encoder) =
        auto_png_candidate(png, &config, cancel, &mut png_notes, settings.thorough)?;
    let palette = if encoder.starts_with("imagequant") {
        Some(png.clone())
    } else {
        None
    };
    let palette_notes = vec!["Inteligentny: użyto palety wysokiej jakości przed kodowaniem. Kolory mogą się minimalnie różnić.".into()];
    let mut candidates = Candidates {
        sizes: vec![(OutputFormat::Png, png.len())],
        skipped: Vec::new(),
        best: Selection {
            bytes: png,
            format: OutputFormat::Png,
            encoder,
            notes: png_notes,
        },
    };
    let graphic = palette.is_some() || few_colors(rgba);
    let opaque = rgba.pixels().all(|p| p[3] == 255);
    let mut timed_out = false;
    let mut png_backup = if !settings.thorough {
        Some((
            candidates.best.bytes.clone(),
            candidates.best.encoder,
            candidates.best.notes.clone(),
        ))
    } else {
        None
    };
    for (input_index, (input, notes)) in std::iter::once((source_png.as_slice(), &[][..]))
        .chain(palette.as_deref().map(|p| (p, palette_notes.as_slice())))
        .enumerate()
    {
        let mut kinds = Vec::new();
        if rgba.width() <= 16_383 && rgba.height() <= 16_383 {
            if settings.thorough || graphic {
                kinds.push(("webp-lossless", OutputFormat::Webp, "libwebp · bezstratny"));
            }
            if settings.thorough {
                kinds.push((
                    "webp-near",
                    OutputFormat::Webp,
                    "libwebp · prawie bezstratny",
                ));
            }
            // A high-quality palette also fits some photos with flat panels.
            // Always compare lossy WebP from the prepared original pixels.
            if settings.thorough || input_index == 0 {
                kinds.push(("webp-lossy", OutputFormat::Webp, "libwebp · stratny"));
            }
        }
        // Fast mode tests one AVIF and JPEG from the original pixels, avoiding
        // redundant lossy recompression of an already quantized palette.
        if settings.thorough || input_index == 0 {
            kinds.push(("avif-lossy", OutputFormat::Avif, "libavif · AOM"));
            if opaque {
                kinds.push(("jpeg", OutputFormat::Jpeg, "MozJPEG"));
            }
        }
        if settings.thorough {
            kinds.push(("avif-lossless", OutputFormat::Avif, "libavif · AOM"));
        }
        if settings.smart_jxl {
            if settings.thorough || !graphic {
                kinds.push(("jxl-lossy", OutputFormat::Jxl, "libjxl"));
            }
            if settings.thorough || graphic {
                kinds.push(("jxl-lossless", OutputFormat::Jxl, "libjxl"));
            }
        }
        for (kind, format, encoder) in kinds {
            check_cancel(cancel)?;
            let result = match crate::png_worker::smart(
                input,
                kind,
                settings.quality,
                settings.thorough,
                cancel,
            ) {
                Ok(crate::png_worker::Attempt::Finished(bytes)) => Ok(bytes),
                Ok(crate::png_worker::Attempt::TimedOut) => {
                    timed_out = true;
                    Err(anyhow::anyhow!("Encoder timeout"))
                }
                Err(error) => Err(error),
            };
            candidates.consider(result, format, encoder, notes, cancel)?;
        }
    }
    if let Some((png, encoder, mut notes)) = png_backup.take() {
        if png.len() <= candidates.best.bytes.len().saturating_mul(105) / 100 {
            let (bytes, optimized_encoder) = zopfli_png(png, settings, cancel, &mut notes)?;
            let encoder = if encoder.starts_with("imagequant") {
                if optimized_encoder == "OxiPNG + Zopfli" {
                    "imagequant + OxiPNG + Zopfli"
                } else {
                    "imagequant + OxiPNG"
                }
            } else {
                optimized_encoder
            };
            candidates.consider(Ok(bytes), OutputFormat::Png, encoder, &notes, cancel)?;
        }
    }
    if timed_out {
        candidates.best.notes.push("Inteligentny: część prób przekroczyła limit czasu. Wybrano najmniejszy z gotowych wyników.".into());
    }
    check_cancel(cancel)?;
    let comparison = candidates
        .sizes
        .iter()
        .map(|(f, n)| format!("{}: {n} B", f.label()))
        .collect::<Vec<_>>()
        .join("; ");
    candidates.best.notes.push(format!(
        "Inteligentny: wybrano {} ({} B). Porównano: {}.",
        candidates.best.format.label(),
        candidates.best.bytes.len(),
        comparison
    ));
    candidates.best.notes.push(
        if graphic {
            "Inteligentny: porównano także zapis bezstratny przydatny do grafik i logo."
        } else {
            "Inteligentny: rozpoznano zdjęcie. Porównano wydajne formaty fotograficzne."
        }
        .into(),
    );
    candidates.best.notes.push("Inteligentny: porównanie obejmuje tryby stratne. Wymiary są zachowane; wygląd może się nieznacznie różnić.".into());
    if settings.smart_jxl && candidates.best.format == OutputFormat::Jxl {
        candidates.best.notes.push(
            "Wybrano JPEG XL. Sprawdź obsługę tego formatu w docelowej aplikacji lub witrynie."
                .into(),
        );
    }
    for skipped in candidates.skipped {
        if !candidates
            .sizes
            .iter()
            .any(|(format, _)| *format == skipped)
        {
            candidates.best.notes.push(format!(
                "Inteligentny: pominięto niedostępny koder {}.",
                skipped.label()
            ));
        }
    }
    Ok(candidates.best)
}

/// Headless worker entry; all inputs are prepared PNGs with no source metadata.
pub(super) fn worker_encode(
    data: &[u8],
    kind: &str,
    quality: u8,
    thorough: bool,
) -> Result<Vec<u8>> {
    let mut reader = ImageReader::with_format(Cursor::new(data), ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(512 * 1024 * 1024);
    limits.max_image_width = Some(65_535);
    limits.max_image_height = Some(65_535);
    reader.limits(limits);
    let decoder = reader.into_decoder()?;
    let (width, height) = decoder.dimensions();
    ensure!(
        u64::from(width) * u64::from(height) <= MAX_PIXELS,
        "Smart input exceeds pixel limit"
    );
    let rgba = DynamicImage::from_decoder(decoder)?.to_rgba8();
    let mut config = Settings {
        quality,
        thorough,
        ..Default::default()
    };
    match kind {
        "webp-lossless" | "webp-near" | "webp-lossy" => {
            config.webp_mode = match kind {
                "webp-lossless" => WebpMode::Lossless,
                "webp-near" => WebpMode::NearLossless,
                _ => WebpMode::Lossy,
            };
            let bytes = encode_webp(&rgba, &config)?;
            crate::metadata::verify_chunks(&bytes, true)?;
            Ok(bytes)
        }
        "avif-lossy" | "avif-lossless" => {
            config.avif_lossless = kind == "avif-lossless";
            crate::modern_codecs::encode(&rgba, OutputFormat::Avif, &config)
        }
        "jxl-lossy" | "jxl-lossless" => {
            config.jxl_lossless = kind == "jxl-lossless";
            crate::modern_codecs::encode(&rgba, OutputFormat::Jxl, &config)
        }
        "jpeg" => {
            ensure!(
                rgba.pixels().all(|p| p[3] == 255),
                "Smart JPEG cannot flatten alpha"
            );
            encode_jpeg(&rgba, &config)
        }
        _ => bail!("Unknown smart encoder"),
    }
}

// Bounded exact classification: stop as soon as a photo-like color count is seen.
fn few_colors(rgba: &RgbaImage) -> bool {
    let mut colors = std::collections::HashSet::new();
    for pixel in rgba.pixels() {
        let color = if pixel[3] == 0 { [0; 4] } else { pixel.0 };
        colors.insert(color);
        if colors.len() > 256 {
            return false;
        }
    }
    true
}
