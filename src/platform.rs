use std::{ffi::OsStr, io, path::Path, process::Command};

pub fn open(target: impl AsRef<OsStr>) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "windows")]
    let program = "explorer.exe";
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let program = "xdg-open";
    Command::new(program).arg(target).spawn().map(|_| ())
}

pub fn reveal(path: &Path) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg("-R").arg(path).spawn().map(|_| ())
    }
    #[cfg(target_os = "windows")]
    {
        let mut selected = std::ffi::OsString::from("/select,");
        selected.push(path);
        Command::new("explorer.exe")
            .arg(selected)
            .spawn()
            .map(|_| ())
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        open(path.parent().unwrap_or(path))
    }
}

#[cfg(target_os = "windows")]
pub fn system_language() -> String {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetUserDefaultLocaleName(buffer: *mut u16, capacity: i32) -> i32;
    }
    let mut buffer = [0_u16; 85];
    // SAFETY: the buffer is writable and its capacity is passed in UTF-16 units.
    let length = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
    if length > 1 {
        String::from_utf16_lossy(&buffer[..length as usize - 1])
    } else {
        "en".into()
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn system_language() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|key| std::env::var(key).ok().filter(|v| !v.is_empty()))
        .unwrap_or_else(|| "en".into())
}
