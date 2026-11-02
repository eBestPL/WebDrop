//! Optional encoders run in killable child processes; threads cannot provide hard timeouts.
use anyhow::{Context, Result, bail, ensure};
use std::{
    ffi::OsString,
    fs::{self, File},
    io::{Read, Write},
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
    let flag = args.next()?;
    let smart = flag == "--internal-smart-encode";
    if flag != "--internal-png-zopfli" && !smart {
        return None;
    }
    Some((|| {
        let input = PathBuf::from(args.next().context("Missing PNG worker input")?);
        let output = PathBuf::from(args.next().context("Missing PNG worker output")?);
        let kind = if smart {
            Some(args.next().context("Missing smart encoder")?)
        } else {
            None
        };
        let quality = if smart {
            args.next()
                .context("Missing smart quality")?
                .to_str()
                .context("Invalid smart quality")?
                .parse::<u8>()?
        } else {
            82
        };
        ensure!((1..=100).contains(&quality), "Invalid smart quality");
        let effort = args.next().context("Missing PNG worker effort")?;
        ensure!(args.next().is_none(), "Unexpected PNG worker arguments");
        ensure!(
            effort == "normal" || effort == "thorough",
            "Invalid PNG worker effort"
        );
        let mut data = Vec::new();
        crate::file_io::open_regular(&input)?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut data)?;
        ensure!(
            data.len() as u64 <= MAX_BYTES,
            "PNG worker input exceeds limit"
        );
        if let Some(kind) = kind {
            let encoded = crate::optimizer::smart_encode_worker(
                &data,
                kind.to_str().context("Invalid smart encoder")?,
                quality,
                effort == "thorough",
            )?;
            write_new(&output, &encoded)?;
            return Ok(());
        }
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
        write_new(&output, &encoded)?;
        Ok(())
    })())
}

// A malformed or manually invoked worker must never truncate an existing file,
// including a source file or a symlink. Parent jobs use fresh private directories.
fn write_new(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    let mut file = File::options().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    Ok(())
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
    run(
        data,
        "--internal-png-zopfli",
        &[OsString::from(if thorough { "thorough" } else { "normal" })],
        cancel,
        Duration::from_secs(if thorough { 30 } else { 10 }),
    )
}

pub(crate) fn smart(
    data: &[u8],
    kind: &str,
    quality: u8,
    thorough: bool,
    cancel: &AtomicBool,
) -> Result<Attempt> {
    run(
        data,
        "--internal-smart-encode",
        &[
            kind.into(),
            quality.to_string().into(),
            OsString::from(if thorough { "thorough" } else { "normal" }),
        ],
        cancel,
        Duration::from_secs(if thorough { 15 } else { 5 }),
    )
}

fn run(
    data: &[u8],
    flag: &str,
    arguments: &[OsString],
    cancel: &AtomicBool,
    timeout: Duration,
) -> Result<Attempt> {
    ensure!(!cancel.load(Ordering::Relaxed), "Anulowano");
    let program = EXECUTABLE
        .get()
        .context("PNG worker executable is not configured")?;
    let directory = tempfile::Builder::new()
        .prefix("webdrop-encoder-")
        .tempdir()?;
    let input = directory.path().join("input.png");
    let output = directory.path().join("output.png");
    fs::write(&input, data)?;
    let mut command = Command::new(program);
    command
        .args([
            OsString::from(flag),
            input.into_os_string(),
            output.clone().into_os_string(),
        ])
        .args(arguments)
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
    let status = wait(&mut child, cancel, timeout)?;
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
            ensure!(!encoded.is_empty(), "Empty encoder result");
            if flag == "--internal-png-zopfli" {
                crate::metadata::verify_chunks(&encoded, false)?;
            }
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
