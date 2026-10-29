#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod open_documents;
mod platform;
#[cfg(not(target_os = "macos"))]
mod preferences;
mod ui;

use clap::Parser;
use image_optimizer::optimizer::{self, OutputFormat, PngMode, Settings, TrimMode, WebpMode};
use std::{path::PathBuf, sync::atomic::AtomicBool};
use ui::i18n::{english_error, english_message};

#[derive(Parser)]
#[command(
    name = "WebDrop Image Optimizer",
    version,
    about = "Desktop image optimizer. Run without --batch to open the GUI."
)]
struct Args {
    /// Process files without opening a window.
    #[arg(long)]
    batch: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Webp)]
    format: OutputFormat,
    /// Maximum width, without upscaling. 0 keeps original dimensions.
    #[arg(long, default_value_t = 1500)]
    width: u32,
    #[arg(long, default_value_t = 82, value_parser = clap::value_parser!(u8).range(1..=100))]
    quality: u8,
    #[arg(long, value_enum, default_value_t = PngMode::Lossless)]
    png_mode: PngMode,
    #[arg(long, value_enum, default_value_t = WebpMode::Lossy)]
    webp_mode: WebpMode,
    /// Lossless AVIF encoding after trimming/resizing and conversion to 8-bit sRGB.
    #[arg(long)]
    avif_lossless: bool,
    /// Lossless JPEG XL encoding after trimming/resizing and conversion to 8-bit sRGB.
    #[arg(long)]
    jxl_lossless: bool,
    #[arg(long)]
    thorough: bool,
    /// Remove margins before resizing. SVG always stays SVG; trim changes only its viewport.
    #[arg(long, value_enum, default_value_t = TrimMode::Off)]
    trim: TrimMode,
    #[arg(long, default_value_t = 8, value_parser = clap::value_parser!(u8).range(0..=64))]
    trim_tolerance: u8,
    /// AI/PDF rasterization resolution (transparent background).
    #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u32).range(72..=600))]
    vector_dpi: u32,
    /// Page/artboard for AI/PDF or page of a multipage TIFF; HEIC uses its primary image.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
    page: u32,
    /// Defaults to ~/Downloads.
    #[arg(long)]
    output: Option<PathBuf>,
    files: Vec<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    run().map_err(|error| anyhow::anyhow!(english_error(&error)))
}

fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.batch {
        anyhow::ensure!(!args.files.is_empty(), "Add at least one file to --batch");
        let output = args
            .output
            .map(Ok)
            .unwrap_or_else(optimizer::downloads_dir)?;
        let settings = Settings {
            format: args.format,
            max_width: (args.width > 0).then_some(args.width),
            quality: args.quality,
            png_mode: args.png_mode,
            webp_mode: args.webp_mode,
            avif_lossless: args.avif_lossless,
            jxl_lossless: args.jxl_lossless,
            thorough: args.thorough,
            trim: args.trim,
            trim_tolerance: args.trim_tolerance,
            vector_dpi: args.vector_dpi,
            page: args.page,
        };
        let mut failed = false;
        for path in args.files {
            match optimizer::optimize(&path, &output, &settings, &AtomicBool::new(false)) {
                Ok(result) => {
                    println!(
                        "{} → {} | {} × {} | {} → {} | {:.1}% saved | {}",
                        path.display(),
                        result.path.display(),
                        result.width,
                        result.height,
                        optimizer::format_bytes(result.input_bytes),
                        optimizer::format_bytes(result.output_bytes),
                        result.savings_percent(),
                        english_message(result.encoder)
                    );
                    for note in &result.notes {
                        println!("  {}", english_message(note));
                    }
                }
                Err(error) => {
                    eprintln!("{}: {}", path.display(), english_error(&error));
                    failed = true;
                }
            }
        }
        anyhow::ensure!(!failed, "Some files could not be processed");
        return Ok(());
    }
    open_documents::install();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("WebDrop")
            .with_inner_size([1000.0, 780.0])
            .with_min_inner_size([920.0, 720.0])
            .with_fullsize_content_view(true)
            .with_title_shown(false)
            .with_titlebar_shown(false)
            .with_drag_and_drop(true),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "WebDrop",
        options,
        Box::new(move |cc| Ok(Box::new(ui::OptimizerApp::new(cc, args.files)))),
    )
    .map_err(|err| anyhow::anyhow!("Cannot open the application window: {err}"))
}
