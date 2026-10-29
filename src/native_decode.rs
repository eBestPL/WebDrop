#[cfg(target_os = "macos")]
use anyhow::ensure;
use anyhow::{Result, bail};
use image::DynamicImage;
#[cfg(target_os = "macos")]
use image::{RgbaImage, metadata::Orientation};

pub struct Decoded {
    pub image: DynamicImage,
    pub pages: u32,
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn io_decode(
        data: *const u8,
        len: usize,
        pdf: i32,
        dpi: u32,
        page: u32,
        output: *mut *mut u8,
        width: *mut u32,
        height: *mut u32,
        orientation: *mut u32,
        pages: *mut u32,
    ) -> i32;
    fn io_free(pixels: *mut u8);
}

#[cfg(target_os = "macos")]
pub fn decode(data: &[u8], pdf: bool, dpi: u32, page: u32) -> Result<Decoded> {
    let mut pixels = std::ptr::null_mut();
    let (mut w, mut h, mut orientation, mut pages) = (0, 0, 1, 1);
    // SAFETY: the input lives through the synchronous call. All out parameters
    // point to valid storage; io_decode bounds allocations and returns owned RGBA.
    let code = unsafe {
        io_decode(
            data.as_ptr(),
            data.len(),
            i32::from(pdf),
            dpi,
            page,
            &mut pixels,
            &mut w,
            &mut h,
            &mut orientation,
            &mut pages,
        )
    };
    match code {
        0 => {}
        2 => bail!("Obraz przekracza limit 64 megapikseli. Dla AI/PDF zmniejsz DPI."),
        3 => bail!("Za mało pamięci do odczytu obrazu"),
        4 => bail!("PDF/AI jest zabezpieczony hasłem. Zapisz odblokowaną kopię."),
        5 => bail!(
            "Brak strony {page}. Liczba stron/obrazów: {pages}. HEIC udostępnia obraz główny jako stronę 1."
        ),
        6 => bail!(
            "AI zapisano bez zawartości PDF. W Illustratorze włącz ‘Create PDF Compatible File’ lub wyeksportuj PDF."
        ),
        _ => bail!("macOS nie może odczytać obrazu. AI musi zawierać zgodną reprezentację PDF."),
    }
    struct Pixels(*mut u8);
    impl Drop for Pixels {
        fn drop(&mut self) {
            // SAFETY: this pointer is owned and came from io_decode's allocator.
            unsafe {
                io_free(self.0);
            }
        }
    }
    let owned = Pixels(pixels);
    ensure!(
        !owned.0.is_null() && w > 0 && h > 0 && u64::from(w) * u64::from(h) <= 64_000_000,
        "Nieprawidłowy wynik dekodera macOS"
    );
    // SAFETY: native decoder guarantees precisely width*height*4 readable bytes.
    let mut bytes =
        unsafe { std::slice::from_raw_parts(owned.0, w as usize * h as usize * 4) }.to_vec();
    for pixel in bytes.as_chunks_mut::<4>().0 {
        let alpha = u32::from(pixel[3]);
        if alpha > 0 && alpha < 255 {
            for color in &mut pixel[..3] {
                *color = ((u32::from(*color) * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
    let rgba = RgbaImage::from_raw(w, h, bytes)
        .ok_or_else(|| anyhow::anyhow!("Nieprawidłowy bufor macOS"))?;
    let mut image = DynamicImage::ImageRgba8(rgba);
    image.apply_orientation(
        Orientation::from_exif(orientation.min(255) as u8).unwrap_or(Orientation::NoTransforms),
    );
    Ok(Decoded { image, pages })
}

#[cfg(not(target_os = "macos"))]
pub fn decode(_data: &[u8], _pdf: bool, _dpi: u32, _page: u32) -> Result<Decoded> {
    bail!("AI/PDF, TIFF i HEIC wymagają macOS")
}
