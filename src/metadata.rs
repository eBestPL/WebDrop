//! Color metadata is consumed before encoding, never copied to raster outputs.
use anyhow::{Context, Result, ensure};
use std::io::Cursor;

pub fn png_profile(data: &[u8], embedded: Option<Vec<u8>>) -> Result<Option<Vec<u8>>> {
    let reader = png::Decoder::new(Cursor::new(data)).read_info()?;
    let info = reader.info();
    // PNG color precedence: cICP > iCCP > sRGB > gAMA/cHRM.
    if let Some(cicp) = info.coding_independent_code_points {
        ensure!(
            cicp.transfer_function == 13
                && cicp.matrix_coefficients == 0
                && cicp.is_video_full_range_image
                && matches!(cicp.color_primaries, 1 | 12),
            "Ten PNG używa nieobsługiwanego profilu cICP/HDR. Wyeksportuj go do sRGB przed usunięciem metadanych."
        );
        if cicp.color_primaries == 1 {
            return Ok(None);
        }
        return Ok(Some(rgb_profile(
            png::SourceChromaticities::new(
                (0.3127, 0.3290),
                (0.680, 0.320),
                (0.265, 0.690),
                (0.150, 0.060),
            ),
            None,
            false,
        )?));
    }
    if embedded.is_some() {
        return Ok(embedded);
    }
    if info.srgb.is_some() {
        return Ok(None);
    }
    if info.gamma().is_none() && info.chromaticities().is_none() {
        return Ok(None);
    }
    let gamma = info.gamma().map(|g| f64::from(g.into_value()));
    if let Some(gamma) = gamma {
        ensure!(gamma.is_finite() && gamma > 0.0, "Nieprawidłowa gamma PNG");
    }
    let chroma = info.chromaticities().unwrap_or_else(|| {
        png::SourceChromaticities::new(
            (0.3127, 0.3290),
            (0.640, 0.330),
            (0.300, 0.600),
            (0.150, 0.060),
        )
    });
    Ok(Some(rgb_profile(
        chroma,
        gamma,
        matches!(
            info.color_type,
            png::ColorType::Grayscale | png::ColorType::GrayscaleAlpha
        ),
    )?))
}

fn rgb_profile(
    chroma: png::SourceChromaticities,
    gamma: Option<f64>,
    gray: bool,
) -> Result<Vec<u8>> {
    let xy = |v: (png::ScaledFloat, png::ScaledFloat)| lcms2::CIExyY {
        x: f64::from(v.0.into_value()),
        y: f64::from(v.1.into_value()),
        Y: 1.0,
    };
    let white = xy(chroma.white);
    let curve = if let Some(gamma) = gamma {
        lcms2::ToneCurve::new_parametric(1, &[1.0 / gamma])?
    } else {
        // sRGB electro-optical transfer function; used when only primaries are tagged.
        lcms2::ToneCurve::new_parametric(
            4,
            &[2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.04045],
        )?
    };
    let profile = if gray {
        lcms2::Profile::new_gray(&white, &curve)?
    } else {
        lcms2::Profile::new_rgb(
            &white,
            &lcms2::CIExyYTRIPLE {
                Red: xy(chroma.red),
                Green: xy(chroma.green),
                Blue: xy(chroma.blue),
            },
            &[&curve, &curve, &curve],
        )?
    };
    profile
        .icc()
        .context("Nie można przeliczyć kolorów PNG przed usunięciem metadanych")
}

pub fn srgb16(
    img: &image::DynamicImage,
    icc: Option<&[u8]>,
) -> Result<image::ImageBuffer<image::Rgba<u16>, Vec<u16>>> {
    let mut rgba = img.to_rgba16();
    if let Some(icc) = icc {
        let source = lcms2::Profile::new_icc(icc)?;
        let target = lcms2::Profile::new_srgb();
        let mut colors: Vec<[u16; 3]> = rgba.pixels().map(|p| [p[0], p[1], p[2]]).collect();
        match source.color_space() {
            lcms2::ColorSpaceSignature::RgbData => {
                lcms2::Transform::<[u16; 3], [u16; 3]>::new(
                    &source,
                    lcms2::PixelFormat::RGB_16,
                    &target,
                    lcms2::PixelFormat::RGB_16,
                    lcms2::Intent::Perceptual,
                )?
                .transform_in_place(&mut colors);
            }
            lcms2::ColorSpaceSignature::GrayData => {
                lcms2::Transform::<u16, [u16; 3]>::new(
                    &source,
                    lcms2::PixelFormat::GRAY_16,
                    &target,
                    lcms2::PixelFormat::RGB_16,
                    lcms2::Intent::Perceptual,
                )?
                .transform_pixels(&img.to_luma16().into_raw(), &mut colors);
            }
            _ => anyhow::bail!(
                "Ten profil ICC nie jest RGB ani grayscale. Wyeksportuj obraz do sRGB przed konwersją."
            ),
        }
        for (pixel, color) in rgba.pixels_mut().zip(colors) {
            pixel.0[..3].copy_from_slice(&color);
        }
    }
    Ok(rgba)
}

/// Final invariant for encoded PNG/WebP, including any future encoder paths.
/// Palette and transparency chunks describe pixels and are not metadata.
pub fn verify_chunks(data: &[u8], webp: bool) -> Result<()> {
    let mut pos = if webp { 12 } else { 8 };
    ensure!(data.len() >= pos, "Niepełny wynik kodera");
    while pos < data.len() {
        ensure!(pos + 8 <= data.len(), "Niepełny blok wyniku kodera");
        let (kind, len, overhead) = if webp {
            (
                &data[pos..pos + 4],
                u32::from_le_bytes(data[pos + 4..pos + 8].try_into()?) as usize,
                8,
            )
        } else {
            (
                &data[pos + 4..pos + 8],
                u32::from_be_bytes(data[pos..pos + 4].try_into()?) as usize,
                12,
            )
        };
        let allowed = if webp {
            matches!(kind, b"VP8 " | b"VP8L" | b"VP8X" | b"ALPH")
        } else {
            matches!(kind, b"IHDR" | b"PLTE" | b"tRNS" | b"IDAT" | b"IEND")
        };
        ensure!(
            allowed,
            "Koder pozostawił metadane; wynik nie został zapisany"
        );
        pos = pos
            .checked_add(len)
            .and_then(|p| p.checked_add(overhead + if webp { len % 2 } else { 0 }))
            .context("Nieprawidłowy rozmiar bloku")?;
        ensure!(pos <= data.len(), "Niepełny blok wyniku kodera");
    }
    Ok(())
}

/// Remove APP0–APP15 and COM, including generated JFIF density information.
/// Keep every coding marker and entropy-coded byte, including progressive scans.
pub fn strip_jpeg(data: &[u8]) -> Result<Vec<u8>> {
    ensure!(data.starts_with(&[0xff, 0xd8]), "Nieprawidłowy wynik JPEG");
    let mut out = data[..2].to_vec();
    let mut pos = 2;
    while pos < data.len() {
        let start = pos;
        ensure!(data[pos] == 0xff, "Nieprawidłowy znacznik JPEG");
        while pos < data.len() && data[pos] == 0xff {
            pos += 1;
        }
        let marker = *data.get(pos).context("Urwany znacznik JPEG")?;
        pos += 1;
        if marker == 0xd9 {
            out.extend_from_slice(&data[start..pos]);
            return Ok(out);
        }
        ensure!(pos + 2 <= data.len(), "Urwany blok JPEG");
        let len = u16::from_be_bytes(data[pos..pos + 2].try_into()?) as usize;
        ensure!(
            len >= 2 && pos + len <= data.len(),
            "Nieprawidłowy rozmiar bloku JPEG"
        );
        pos += len;
        if !(0xe0..=0xef).contains(&marker) && marker != 0xfe {
            out.extend_from_slice(&data[start..pos]);
        }
        if marker == 0xda {
            let scan_start = pos;
            while pos + 1 < data.len() {
                if data[pos] == 0xff {
                    let next = data[pos + 1];
                    if next == 0 || (0xd0..=0xd7).contains(&next) {
                        pos += 2;
                        continue;
                    }
                    if next != 0xff {
                        break;
                    }
                }
                pos += 1;
            }
            out.extend_from_slice(&data[scan_start..pos]);
        }
    }
    anyhow::bail!("Brak końca obrazu JPEG")
}

/// AVIF CICP is structural color signaling, not a copied source ICC profile.
pub fn avif_profile(primaries: u32, transfer: u32) -> Result<Option<Vec<u8>>> {
    ensure!(
        matches!(primaries, 1 | 2 | 12) && matches!(transfer, 1 | 2 | 6 | 13),
        "AVIF/JXL: nieobsługiwany profil kolorów lub HDR. Wyeksportuj obraz do SDR/sRGB."
    );
    if matches!(primaries, 1 | 2) && matches!(transfer, 2 | 13) {
        return Ok(None);
    }
    let chroma = if primaries == 12 {
        png::SourceChromaticities::new(
            (0.3127, 0.3290),
            (0.680, 0.320),
            (0.265, 0.690),
            (0.150, 0.060),
        )
    } else {
        png::SourceChromaticities::new(
            (0.3127, 0.3290),
            (0.640, 0.330),
            (0.300, 0.600),
            (0.150, 0.060),
        )
    };
    if matches!(transfer, 1 | 6) {
        let xy = |v: (png::ScaledFloat, png::ScaledFloat)| lcms2::CIExyY {
            x: f64::from(v.0.into_value()),
            y: f64::from(v.1.into_value()),
            Y: 1.0,
        };
        let curve = lcms2::ToneCurve::new_parametric(
            4,
            &[1.0 / 0.45, 1.0 / 1.099, 0.099 / 1.099, 1.0 / 4.5, 0.081],
        )?;
        return Ok(Some(
            lcms2::Profile::new_rgb(
                &xy(chroma.white),
                &lcms2::CIExyYTRIPLE {
                    Red: xy(chroma.red),
                    Green: xy(chroma.green),
                    Blue: xy(chroma.blue),
                },
                &[&curve, &curve, &curve],
            )?
            .icc()?,
        ));
    }
    Ok(Some(rgb_profile(chroma, None, false)?))
}
