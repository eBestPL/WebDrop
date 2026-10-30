use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Mode {
    #[default]
    Downloads,
    SameFolder,
    OverwriteOriginals,
    Custom,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Destination {
    pub mode: Mode,
    pub custom: Option<PathBuf>,
}

impl Destination {
    pub fn configured(&self) -> bool {
        self.mode != Mode::Custom || self.custom.as_ref().is_some_and(|p| p.is_absolute())
    }

    pub fn resolve(
        &self,
        input: &Path,
        downloads: &Result<PathBuf, String>,
    ) -> Result<PathBuf, String> {
        match self.mode {
            Mode::Downloads => downloads.clone(),
            Mode::Custom => self
                .custom
                .clone()
                .filter(|p| p.is_absolute())
                .ok_or_else(|| "Wybierz folder wynikowy.".into()),
            Mode::SameFolder | Mode::OverwriteOriginals => {
                let parent = input
                    .parent()
                    .ok_or_else(|| "Nie można znaleźć folderu źródłowego.".to_owned())?;
                if parent.is_absolute() {
                    Ok(parent.to_owned())
                } else {
                    std::env::current_dir()
                        .map(|cwd| cwd.join(parent))
                        .map_err(|err| err.to_string())
                }
            }
        }
    }

    pub fn load() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::load_domain(c"local.imageoptimizer.desktop")
        }
        #[cfg(not(target_os = "macos"))]
        {
            crate::preferences::load("destination").unwrap_or_default()
        }
    }

    pub fn save(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.save_domain(c"local.imageoptimizer.desktop")
        }
        #[cfg(not(target_os = "macos"))]
        {
            crate::preferences::save("destination", self).is_ok()
        }
    }

    #[cfg(target_os = "macos")]
    fn load_domain(domain: &std::ffi::CStr) -> Self {
        use std::os::unix::ffi::OsStringExt;
        let mut mode = 0;
        let mut bytes = vec![0; 4096];
        // SAFETY: the bridge receives valid buffers and never writes beyond capacity.
        let length = unsafe {
            io_load_output_preference(domain.as_ptr(), &mut mode, bytes.as_mut_ptr(), bytes.len())
        };
        let custom = if length > 0 && length <= bytes.len() {
            bytes.truncate(length);
            let path = PathBuf::from(std::ffi::OsString::from_vec(bytes));
            path.is_absolute().then_some(path)
        } else {
            None
        };
        Self {
            mode: match mode {
                1 => Mode::SameFolder,
                2 => Mode::Custom,
                3 => Mode::OverwriteOriginals,
                _ => Mode::Downloads,
            },
            custom,
        }
    }

    #[cfg(target_os = "macos")]
    fn save_domain(&self, domain: &std::ffi::CStr) -> bool {
        use std::os::unix::ffi::OsStrExt;
        let path = self
            .custom
            .as_ref()
            .map_or(&[][..], |p| p.as_os_str().as_bytes());
        let mode = match self.mode {
            Mode::Downloads => 0,
            Mode::SameFolder => 1,
            Mode::Custom => 2,
            Mode::OverwriteOriginals => 3,
        };
        // SAFETY: domain is NUL-terminated; path points to length readable bytes for this call.
        unsafe { io_save_output_preference(domain.as_ptr(), mode, path.as_ptr(), path.len()) }
    }
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn io_load_output_preference(
        domain: *const std::ffi::c_char,
        mode: *mut i32,
        path: *mut u8,
        capacity: usize,
    ) -> usize;
    fn io_save_output_preference(
        domain: *const std::ffi::c_char,
        mode: i32,
        path: *const u8,
        length: usize,
    ) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destination_modes_resolve_without_silent_fallback() {
        let mut dest = Destination::default();
        let downloads = Ok(PathBuf::from("/tmp/downloads"));
        assert_eq!(
            dest.resolve(Path::new("/tmp/source/logo.png"), &downloads)
                .unwrap(),
            Path::new("/tmp/downloads")
        );
        dest.mode = Mode::SameFolder;
        assert_eq!(
            dest.resolve(Path::new("/tmp/source/logo.png"), &downloads)
                .unwrap(),
            Path::new("/tmp/source")
        );
        assert_eq!(
            dest.resolve(Path::new("logo.png"), &downloads).unwrap(),
            std::env::current_dir().unwrap()
        );
        dest.mode = Mode::Custom;
        assert!(!dest.configured());
        assert!(dest.resolve(Path::new("logo.png"), &downloads).is_err());
        dest.custom = Some("relative".into());
        assert!(!dest.configured());
        dest.custom = Some("/tmp/Zażółć gęślą 🖼".into());
        assert!(dest.configured());
        assert_eq!(
            dest.resolve(Path::new("logo.png"), &Err("no downloads".into()))
                .unwrap(),
            dest.custom.clone().unwrap()
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn preferences_roundtrip_preserves_mode_and_custom_path() {
        // A unique test domain keeps the user's preferences untouched.
        let domain = std::ffi::CString::new(format!(
            "local.imageoptimizer.destination-test.{}",
            std::process::id()
        ))
        .unwrap();
        let mut dest = Destination::default();
        assert_eq!(Destination::load_domain(&domain), dest);
        dest.custom = Some("/tmp/Zażółć gęślą 🖼/exports".into());
        for mode in [
            Mode::Custom,
            Mode::SameFolder,
            Mode::Downloads,
            Mode::OverwriteOriginals,
        ] {
            dest.mode = mode;
            assert!(dest.save_domain(&domain));
            assert_eq!(Destination::load_domain(&domain), dest);
        }
        // Default with no custom path removes the test preference key.
        assert!(Destination::default().save_domain(&domain));
        assert_eq!(Destination::load_domain(&domain), Destination::default());
    }
}
