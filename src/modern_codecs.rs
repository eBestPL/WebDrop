//! AVIF / JPEG XL use bundled native codecs; only prepared RGBA pixels reach encoders.
use crate::optimizer::{OutputFormat, Settings};
use anyhow::{Result, bail, ensure};
use image::{DynamicImage, RgbaImage};

#[repr(C)]
#[derive(Default)]
struct NativeImage {
    pixels: *mut u8,
    icc: *mut u8,
    icc_len: usize,
    width: u32,
    height: u32,
    primaries: u32,
    transfer: u32,
    rotation: u32,
    mirror: u32,
    crop_x: u32,
    crop_y: u32,
    crop_w: u32,
    crop_h: u32,
}

unsafe extern "C" {
    fn wd_free(ptr: *mut u8);
    fn wd_avif_encode(
        pixels: *const u8,
        w: u32,
        h: u32,
        quality: i32,
        lossless: i32,
        thorough: i32,
        out: *mut *mut u8,
        len: *mut usize,
    ) -> i32;
    fn wd_jxl_encode(
        pixels: *const u8,
        w: u32,
        h: u32,
        quality: i32,
        lossless: i32,
        thorough: i32,
        out: *mut *mut u8,
        len: *mut usize,
    ) -> i32;
    fn wd_avif_decode(data: *const u8, len: usize, out: *mut NativeImage) -> i32;
    fn wd_jxl_decode(data: *const u8, len: usize, out: *mut NativeImage) -> i32;
}

struct Allocation(*mut u8);
impl Drop for Allocation {
    fn drop(&mut self) {
        // SAFETY: bridge allocations are exclusively owned and use malloc; NULL is valid.
        unsafe { wd_free(self.0) }
    }
}
fn check(code: i32) -> Result<()> {
    match code {
        0 => Ok(()),
        2 => bail!("AVIF/JXL: przekroczono limit rozmiaru lub pamięci"),
        3 => bail!("AVIF/JXL: za mało pamięci"),
        4 => bail!("Animowane AVIF/JXL nie są obsługiwane; animacja nie została zmieniona"),
        5 => {
            bail!("AVIF/JXL: nieobsługiwany profil kolorów lub HDR. Wyeksportuj obraz do SDR/sRGB.")
        }
        6 => bail!("Nie udało się zakodować AVIF/JXL"),
        _ => bail!("Nie można odczytać AVIF/JXL: uszkodzony lub nieobsługiwany obraz"),
    }
}

pub fn encode(image: &RgbaImage, format: OutputFormat, settings: &Settings) -> Result<Vec<u8>> {
    let (encoder, lossless) = match format {
        OutputFormat::Avif => (
            wd_avif_encode
                as unsafe extern "C" fn(
                    *const u8,
                    u32,
                    u32,
                    i32,
                    i32,
                    i32,
                    *mut *mut u8,
                    *mut usize,
                ) -> i32,
            settings.avif_lossless,
        ),
        OutputFormat::Jxl => (
            wd_jxl_encode
                as unsafe extern "C" fn(
                    *const u8,
                    u32,
                    u32,
                    i32,
                    i32,
                    i32,
                    *mut *mut u8,
                    *mut usize,
                ) -> i32,
            settings.jxl_lossless,
        ),
        _ => unreachable!(),
    };
    let mut ptr = std::ptr::null_mut();
    let mut len = 0;
    // SAFETY: RGBA8 contains exactly width*height*4 bytes; output storage is valid.
    let code = unsafe {
        encoder(
            image.as_ptr(),
            image.width(),
            image.height(),
            i32::from(settings.quality),
            i32::from(lossless),
            i32::from(settings.thorough),
            &mut ptr,
            &mut len,
        )
    };
    let buffer = Allocation(ptr);
    check(code)?;
    ensure!(!buffer.0.is_null() && len > 0, "Pusty wynik AVIF/JXL");
    // SAFETY: the bridge returns an allocation of len bytes on success.
    Ok(unsafe { std::slice::from_raw_parts(buffer.0, len) }.to_vec())
}

pub fn decode(data: &[u8], format: OutputFormat) -> Result<(DynamicImage, Option<Vec<u8>>)> {
    let mut result = NativeImage::default();
    // SAFETY: input and output storage live throughout this synchronous call.
    let code = unsafe {
        match format {
            OutputFormat::Avif => wd_avif_decode(data.as_ptr(), data.len(), &mut result),
            OutputFormat::Jxl => wd_jxl_decode(data.as_ptr(), data.len(), &mut result),
            _ => unreachable!(),
        }
    };
    let pixels = Allocation(result.pixels);
    let profile = Allocation(result.icc);
    check(code)?;
    ensure!(
        !pixels.0.is_null()
            && result.width > 0
            && result.height > 0
            && u64::from(result.width) * u64::from(result.height) <= 64_000_000,
        "Nieprawidłowy wynik AVIF/JXL"
    );
    // SAFETY: dimensions are checked before allocation in the bridge.
    let bytes = unsafe {
        std::slice::from_raw_parts(pixels.0, result.width as usize * result.height as usize * 4)
    }
    .to_vec();
    let mut image = DynamicImage::ImageRgba8(
        RgbaImage::from_raw(result.width, result.height, bytes).expect("validated RGBA length"),
    );
    if (result.crop_x, result.crop_y, result.crop_w, result.crop_h)
        != (0, 0, result.width, result.height)
    {
        ensure!(
            result.crop_w > 0
                && result.crop_h > 0
                && u64::from(result.crop_x) + u64::from(result.crop_w) <= u64::from(result.width)
                && u64::from(result.crop_y) + u64::from(result.crop_h) <= u64::from(result.height),
            "Nieprawidłowy wynik AVIF/JXL"
        );
        image = image.crop_imm(result.crop_x, result.crop_y, result.crop_w, result.crop_h);
    }
    image = match result.rotation {
        1 => image.rotate270(),
        2 => image.rotate180(),
        3 => image.rotate90(),
        _ => image,
    };
    image = match result.mirror {
        1 => image.flipv(),
        2 => image.fliph(),
        _ => image,
    };
    let icc = if result.icc_len > 0 {
        ensure!(
            !profile.0.is_null() && result.icc_len <= 4 * 1024 * 1024,
            "Nieprawidłowy wynik AVIF/JXL"
        );
        // SAFETY: native profile copy has exactly icc_len bytes.
        Some(unsafe { std::slice::from_raw_parts(profile.0, result.icc_len) }.to_vec())
    } else if format == OutputFormat::Avif {
        crate::metadata::avif_profile(result.primaries, result.transfer)?
    } else {
        None
    }; // libjxl's CMS has already converted to sRGB.
    Ok((image, icc))
}
