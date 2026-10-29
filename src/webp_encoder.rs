//! Small, ownership-safe boundary around the current, statically bundled libwebp.
use anyhow::{Result, ensure};
use image::RgbaImage;
use libwebp_sys as ffi;
use std::mem::MaybeUninit;

pub fn config() -> Result<ffi::WebPConfig> {
    let mut config = MaybeUninit::<ffi::WebPConfig>::uninit();
    // SAFETY: libwebp initializes this correctly sized struct on success. Use the
    // encoder ABI explicitly (the upstream convenience helper uses the decoder ABI).
    unsafe {
        ensure!(
            ffi::WebPConfigInitInternal(
                config.as_mut_ptr(),
                ffi::WebPPreset::WEBP_PRESET_DEFAULT,
                75.0,
                ffi::WEBP_ENCODER_ABI_VERSION as i32
            ) != 0,
            "Błąd konfiguracji WebP"
        );
        Ok(config.assume_init())
    }
}

struct Picture(ffi::WebPPicture);
impl Drop for Picture {
    fn drop(&mut self) {
        // SAFETY: picture was initialized by libwebp and is owned exclusively here.
        unsafe {
            ffi::WebPPictureFree(&mut self.0);
        }
    }
}
struct Memory(ffi::WebPMemoryWriter);
impl Drop for Memory {
    fn drop(&mut self) {
        // SAFETY: writer was initialized by libwebp and is owned exclusively here.
        unsafe {
            ffi::WebPMemoryWriterClear(&mut self.0);
        }
    }
}

pub fn encode(img: &RgbaImage, config: &ffi::WebPConfig) -> Result<Vec<u8>> {
    ensure!(
        img.width() > 0 && img.height() > 0 && img.width() <= 16_383 && img.height() <= 16_383,
        "Nieprawidłowy rozmiar WebP"
    );
    // SAFETY: all image buffers are owned RgbaImages with valid dimensions and
    // width*height*4 bytes. C-owned allocations have RAII guards on every error
    // path. The writer and image stay alive and stationary during WebPEncode;
    // libwebp joins its internal threads before returning.
    unsafe {
        ensure!(
            ffi::WebPValidateConfig(config) != 0,
            "Nieprawidłowe parametry WebP"
        );
        let mut raw_picture = MaybeUninit::<ffi::WebPPicture>::uninit();
        ensure!(
            ffi::WebPPictureInitInternal(
                raw_picture.as_mut_ptr(),
                ffi::WEBP_ENCODER_ABI_VERSION as i32
            ) != 0,
            "Błąd inicjalizacji WebP"
        );
        let mut picture = Picture(raw_picture.assume_init());
        picture.0.width = img.width() as i32;
        picture.0.height = img.height() as i32;
        picture.0.use_argb = 1;
        ensure!(
            ffi::WebPPictureImportRGBA(
                &mut picture.0,
                img.as_raw().as_ptr(),
                img.width() as i32 * 4
            ) != 0,
            "Za mało pamięci dla WebP"
        );
        let mut raw_memory = MaybeUninit::<ffi::WebPMemoryWriter>::uninit();
        ffi::WebPMemoryWriterInit(raw_memory.as_mut_ptr());
        let mut memory = Memory(raw_memory.assume_init());
        picture.0.writer = Some(ffi::WebPMemoryWrite);
        picture.0.custom_ptr = (&mut memory.0 as *mut ffi::WebPMemoryWriter).cast();
        ensure!(
            ffi::WebPEncode(config, &mut picture.0) != 0,
            "Błąd kodera WebP: {:?}",
            picture.0.error_code
        );
        ensure!(
            !memory.0.mem.is_null() && memory.0.size > 0 && memory.0.size <= isize::MAX as usize,
            "Pusty wynik WebP"
        );
        Ok(std::slice::from_raw_parts(memory.0.mem, memory.0.size).to_vec())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn bundled_webp_is_at_least_1_6() {
        // SAFETY: version query has no parameters or memory ownership.
        assert!(unsafe { libwebp_sys::WebPGetEncoderVersion() } >= 0x010600);
    }
}
