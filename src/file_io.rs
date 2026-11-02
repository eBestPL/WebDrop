//! Open only regular files, without waiting for a pipe writer on Unix.
use anyhow::{Context, Result, ensure};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

pub(crate) fn open_regular(path: &Path) -> Result<File> {
    // Reject devices before opening them. The descriptor check below also covers
    // a path replaced between this check and open(). Regular symlinks may be read.
    ensure!(
        std::fs::metadata(path)?.is_file(),
        "Input must be a regular file"
    );
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = options.open(path).context("Cannot open input")?;
    ensure!(file.metadata()?.is_file(), "Input must be a regular file");
    Ok(file)
}
