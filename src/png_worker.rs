//! Zopfli runs in a killable child process; a thread cannot provide a hard timeout.
use anyhow::{Context, Result, bail, ensure};
use std::{
    ffi::OsString,
    fs::{self, File},
    io::Read,
    num::NonZeroU64,
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

static EXECUTABLE: OnceLock<PathBuf> = OnceLock::new();
const MAX_BYTES: u64 = 128 * 1024 * 1024;
const POLL: Duration = Duration::from_millis(25);

/// Register the WebDrop executable at startup (also used by library integration tests).
pub fn configure_executable(path: PathBuf) {
    let _ = EXECUTABLE.set(path);
}

/// Private CLI entry, handled before GUI startup. Never writes to a user source file.
pub fn worker_entry() -> Option<Result<()>> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--internal-png-zopfli")) {
        return None;
    }
    Some((|| {
        let input = PathBuf::from(args.next().context("Missing PNG worker input")?);
        let output = PathBuf::from(args.next().context("Missing PNG worker output")?);
        let effort = args.next().context("Missing PNG worker effort")?;
        ensure!(args.next().is_none(), "Unexpected PNG worker arguments");
        ensure!(
            effort == "normal" || effort == "thorough",
            "Invalid PNG worker effort"
        );
        let mut data = Vec::new();
        File::open(input)?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut data)?;
        ensure!(
            data.len() as u64 <= MAX_BYTES,
            "PNG worker input exceeds limit"
        );
        let mut options = oxipng::Options::from_preset(3);
        options.strip = oxipng::StripChunks::All;
        options.max_decompressed_size = Some(512 * 1024 * 1024);
        // Pick a filter using cheap trials, then run Zopfli once, not once per filter.
        options.fast_evaluation = true;
        options.deflater = oxipng::Deflater::Zopfli(oxipng::ZopfliOptions {
            iteration_count: NonZeroU64::new(if effort == "thorough" { 15 } else { 5 }).unwrap(),
            iterations_without_improvement: NonZeroU64::new(3).unwrap(),
            ..Default::default()
        });
        let encoded = oxipng::optimize_from_memory(&data, &options)?;
        crate::metadata::verify_chunks(&encoded, false)?;
        fs::write(output, encoded)?;
        Ok(())
    })())
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        // kill + wait on every path: no detached encoder continues burning CPU.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait(child: &mut Process, cancel: &AtomicBool, timeout: Duration) -> Result<Option<ExitStatus>> {
    let started = Instant::now();
    loop {
        ensure!(!cancel.load(Ordering::Relaxed), "Anulowano");
        if let Some(status) = child.0.try_wait()? {
            return Ok(Some(status));
        }
        if started.elapsed() >= timeout {
            return Ok(None);
        }
        std::thread::sleep(POLL);
    }
}

pub(crate) enum Attempt {
    Finished(Vec<u8>),
    TimedOut,
}

pub(crate) fn optimize(data: &[u8], thorough: bool, cancel: &AtomicBool) -> Result<Attempt> {
    ensure!(!cancel.load(Ordering::Relaxed), "Anulowano");
    let program = EXECUTABLE
        .get()
        .context("PNG worker executable is not configured")?;
    let directory = tempfile::Builder::new()
        .prefix("webdrop-zopfli-")
        .tempdir()?;
    let input = directory.path().join("input.png");
    let output = directory.path().join("output.png");
    fs::write(&input, data)?;
    let mut command = Command::new(program);
    command
        .args([
            OsString::from("--internal-png-zopfli"),
            input.into_os_string(),
            output.clone().into_os_string(),
            OsString::from(if thorough { "thorough" } else { "normal" }),
        ])
        .env("RAYON_NUM_THREADS", "2")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = Process(command.spawn().context("Cannot start PNG worker")?);
    let status = wait(
        &mut child,
        cancel,
        Duration::from_secs(if thorough { 30 } else { 10 }),
    )?;
    // Reap before reading results or dropping the temporary directory.
    drop(child);
    match status {
        None => Ok(Attempt::TimedOut),
        Some(status) if status.success() => {
            let mut encoded = Vec::new();
            File::open(output)?
                .take(MAX_BYTES + 1)
                .read_to_end(&mut encoded)?;
            ensure!(
                encoded.len() as u64 <= MAX_BYTES,
                "PNG worker output exceeds limit"
            );
            crate::metadata::verify_chunks(&encoded, false)?;
            Ok(Attempt::Finished(encoded))
        }
        Some(_) => bail!("PNG worker failed"),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn sleeper() -> Process {
        Process(Command::new("/bin/sleep").arg("60").spawn().unwrap())
    }

    #[test]
    fn deadline_and_cancel_kill_and_reap_the_encoder() {
        for cancelled in [false, true] {
            let mut child = sleeper();
            let pid = child.0.id();
            let flag = Arc::new(AtomicBool::new(false));
            let other = flag.clone();
            let trigger = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(100));
                other.store(cancelled, Ordering::Relaxed);
            });
            let start = Instant::now();
            let result = wait(&mut child, &flag, Duration::from_millis(200));
            drop(child);
            trigger.join().unwrap();
            if cancelled {
                assert_eq!(result.unwrap_err().to_string(), "Anulowano");
            } else {
                assert!(result.unwrap().is_none());
            }
            assert!(start.elapsed() < Duration::from_secs(2));
            // A reaped process must no longer be alive.
            let status = Command::new("/bin/kill")
                .args(["-0", &pid.to_string()])
                .stderr(Stdio::null())
                .status()
                .unwrap();
            assert!(!status.success());
        }
    }
}
