//! Finder's Open With and Dock drops arrive as Apple Events, not CLI arguments.
use eframe::egui;
use std::{path::PathBuf, sync::Mutex};

struct Inbox {
    paths: Vec<PathBuf>,
    context: Option<egui::Context>,
}

static INBOX: Mutex<Inbox> = Mutex::new(Inbox {
    paths: Vec::new(),
    context: None,
});

pub fn install() {
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn io_install_open_documents(callback: extern "C" fn(*const std::ffi::c_char));
        }
        // SAFETY: main calls this on the main thread before starting AppKit.
        unsafe { io_install_open_documents(receive_path) };
    }
}

#[cfg(target_os = "macos")]
extern "C" fn receive_path(path: *const std::ffi::c_char) {
    use std::{
        ffi::{CStr, OsStr},
        os::unix::ffi::OsStrExt,
    };
    if path.is_null() {
        return;
    }
    // SAFETY: native callback supplies a valid C string for this call.
    // Copy filesystem bytes immediately; do not interpret percent escapes.
    let path = PathBuf::from(OsStr::from_bytes(
        unsafe { CStr::from_ptr(path) }.to_bytes(),
    ));
    let context = {
        let mut inbox = INBOX.lock().unwrap_or_else(|error| error.into_inner());
        inbox.paths.push(path);
        inbox.context.clone()
    };
    if let Some(context) = context {
        context.request_repaint();
    }
}

pub fn attach(context: &egui::Context) {
    let mut inbox = INBOX.lock().unwrap_or_else(|error| error.into_inner());
    inbox.context = Some(context.clone());
    if !inbox.paths.is_empty() {
        context.request_repaint();
    }
}

pub fn take() -> Vec<PathBuf> {
    let mut inbox = INBOX.lock().unwrap_or_else(|error| error.into_inner());
    std::mem::take(&mut inbox.paths)
}
