//! Collect dropped folders off the UI thread before staging any conversion.
use image_optimizer::optimizer;
use std::{
    collections::HashSet,
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
};

#[derive(Default)]
pub(super) struct Collected {
    pub files: Vec<PathBuf>,
    pub unreadable: usize,
    pub first_error: Option<String>,
}

impl Collected {
    fn error(&mut self, path: &std::path::Path, error: std::io::Error) {
        self.unreadable += 1;
        self.first_error
            .get_or_insert_with(|| format!("{}: {error}", path.display()));
    }
}

pub(super) struct Scan {
    pub result: Receiver<Collected>,
    cancel: Arc<AtomicBool>,
}

impl Scan {
    pub fn start(paths: Vec<PathBuf>, ctx: eframe::egui::Context) -> std::io::Result<Self> {
        let (send, result) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        std::thread::Builder::new()
            .name("image-folder-scan".into())
            .spawn(move || {
                let collected = collect(paths, &worker_cancel);
                let _ = send.send(collected);
                ctx.request_repaint();
            })?;
        Ok(Self { result, cancel })
    }
}

impl Drop for Scan {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

fn collect(paths: Vec<PathBuf>, cancel: &AtomicBool) -> Collected {
    let mut result = Collected::default();
    let mut seen = HashSet::new();
    let mut directories = HashSet::new();
    // Explicit files retain existing behavior (including errors for unsupported files).
    let mut stack: Vec<_> = paths.into_iter().rev().map(|p| (p, true)).collect();
    while let Some((path, explicit)) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            return Collected::default();
        }
        let metadata = if explicit {
            fs::metadata(&path)
        } else {
            fs::symlink_metadata(&path)
        };
        let metadata = match metadata {
            Ok(metadata) => metadata,
            Err(error) => {
                result.error(&path, error);
                continue;
            }
        };
        // Never follow links found inside a folder, including directory loops.
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            let identity = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            if !directories.insert(identity) {
                continue;
            }
            let entries = match fs::read_dir(&path) {
                Ok(entries) => entries,
                Err(error) => {
                    result.error(&path, error);
                    continue;
                }
            };
            let mut children = Vec::new();
            for entry in entries {
                if cancel.load(Ordering::Relaxed) {
                    return Collected::default();
                }
                match entry {
                    Ok(entry) if !entry.file_name().to_string_lossy().starts_with('.') => {
                        children.push(entry.path());
                    }
                    Ok(_) => {}
                    Err(error) => result.error(&path, error),
                }
            }
            children.sort();
            stack.extend(children.into_iter().rev().map(|p| (p, false)));
        } else if metadata.is_file() && (explicit || optimizer::supported(&path)) {
            let identity = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            if seen.insert(identity) {
                result.files.push(path);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recursive_scan_filters_hidden_and_unsupported_entries_and_deduplicates_roots() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("nested/deeper")).unwrap();
        fs::create_dir(root.join(".hidden")).unwrap();
        for name in [
            "a.PNG",
            "nested/b.jpg",
            "nested/deeper/c.webp",
            "notes.txt",
            ".hidden/d.png",
            "._a.PNG",
        ] {
            fs::write(root.join(name), b"fixture").unwrap();
        }
        let result = collect(
            vec![root.to_owned(), root.join("nested"), root.join("a.PNG")],
            &AtomicBool::new(false),
        );
        assert_eq!(
            result.files,
            [
                root.join("a.PNG"),
                root.join("nested/b.jpg"),
                root.join("nested/deeper/c.webp")
            ]
        );
        assert_eq!(result.unreadable, 0);
    }

    #[test]
    fn empty_folders_cancellation_and_unreadable_inputs() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            collect(vec![dir.path().to_owned()], &AtomicBool::new(false))
                .files
                .is_empty()
        );
        let image = dir.path().join("image.png");
        fs::write(&image, b"fixture").unwrap();
        assert!(
            collect(vec![dir.path().to_owned()], &AtomicBool::new(true))
                .files
                .is_empty()
        );
        let missing = dir.path().join("missing");
        let result = collect(
            vec![missing.clone(), image.clone()],
            &AtomicBool::new(false),
        );
        assert_eq!(result.files, [image]);
        assert_eq!(result.unreadable, 1);
        assert!(
            result
                .first_error
                .unwrap()
                .contains(&missing.display().to_string())
        );
    }

    #[cfg(unix)]
    #[test]
    fn nested_symlinks_are_skipped_including_cycles_and_external_files() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("images");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("a.png"), b"fixture").unwrap();
        fs::write(dir.path().join("outside.jpg"), b"fixture").unwrap();
        symlink(&root, root.join("loop")).unwrap();
        symlink(dir.path().join("outside.jpg"), root.join("linked.jpg")).unwrap();
        let result = collect(vec![root.clone()], &AtomicBool::new(false));
        assert_eq!(result.files, [root.join("a.png")]);
        assert_eq!(result.unreadable, 0);
    }
}
