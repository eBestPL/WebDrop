use eframe::egui::{self, Color32, RichText, Vec2};
use image_optimizer::optimizer::{
    self, Outcome, OutputFormat, PngMode, Settings, TrimMode, WebpMode,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};

mod appearance;
mod destination;
use destination::{Destination, Mode as DestinationMode};
mod help;
pub(crate) mod i18n;
mod inputs;
use appearance::{Palette, segments};
use i18n::{Language, f, t};

struct Job {
    id: u64,
    input: PathBuf,
    output: PathBuf,
    settings: Settings,
    cancel: Arc<AtomicBool>,
}

enum Event {
    Started(u64),
    Finished(u64, Result<Outcome, String>),
}
enum Status {
    Ready,
    Waiting,
    Working,
    Done(Outcome),
    Failed(String),
    Cancelled,
}
impl Status {
    fn pending(&self) -> bool {
        matches!(self, Self::Waiting | Self::Working)
    }
}
struct Row {
    id: u64,
    input: PathBuf,
    settings: Settings,
    status: Status,
}

pub struct OptimizerApp {
    settings: Settings,
    rows: Vec<Row>,
    scans: Vec<inputs::Scan>,
    jobs: Sender<Job>,
    events: Receiver<Event>,
    cancel: Arc<AtomicBool>,
    downloads: Result<PathBuf, String>,
    destination: Destination,
    next_id: u64,
    batch_total: usize,
    notice: Option<String>,
    confirm_close: bool,
    close_when_idle: bool,
    advanced: bool,
    appearance: egui::ThemePreference,
    detail_row: Option<u64>,
    settings_tab: u8,
    language: Language,
    system_language: String,
}

impl OptimizerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, files: Vec<PathBuf>) -> Self {
        let language = Language::load();
        let system_language = i18n::system_language();
        language.activate(&system_language);
        let ctx = cc.egui_ctx.clone();
        crate::open_documents::attach(&ctx);
        appearance::install(&ctx);
        let (jobs, work) = mpsc::channel::<Job>();
        let (updates, events) = mpsc::channel();
        std::thread::Builder::new()
            .name("image-optimizer-worker".into())
            .spawn(move || {
                while let Ok(job) = work.recv() {
                    if updates.send(Event::Started(job.id)).is_err() {
                        break;
                    }
                    ctx.request_repaint();
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        optimizer::optimize(&job.input, &job.output, &job.settings, &job.cancel)
                    }))
                    .map_err(|_| "Koder nie mógł przetworzyć tego obrazu".to_string())
                    .and_then(|result| result.map_err(|err| format!("{err:#}")));
                    if updates.send(Event::Finished(job.id, result)).is_err() {
                        break;
                    }
                    ctx.request_repaint();
                }
            })
            .expect("Cannot create optimizer worker");
        let mut app = Self {
            settings: Settings::default(),
            rows: Vec::new(),
            scans: Vec::new(),
            jobs,
            events,
            cancel: Arc::new(AtomicBool::new(false)),
            downloads: optimizer::downloads_dir().map_err(|err| err.to_string()),
            destination: Destination::load(),
            next_id: 0,
            batch_total: 0,
            notice: None,
            confirm_close: false,
            close_when_idle: false,
            advanced: false,
            appearance: egui::ThemePreference::System,
            detail_row: None,
            settings_tab: 2,
            language,
            system_language,
        };
        app.import_paths(files, &cc.egui_ctx);
        app
    }

    fn import_paths(&mut self, paths: Vec<PathBuf>, ctx: &egui::Context) {
        if paths.is_empty() || self.close_when_idle {
            return;
        }
        self.notice = None;
        match inputs::Scan::start(paths, ctx.clone()) {
            Ok(scan) => self.scans.push(scan),
            Err(error) => {
                self.notice = Some(format!("Nie można rozpocząć wyszukiwania obrazów: {error}"));
            }
        }
    }

    fn collect_imports(&mut self) {
        let mut index = 0;
        while index < self.scans.len() {
            match self.scans[index].result.try_recv() {
                Ok(collected) => {
                    self.scans.remove(index);
                    if collected.unreadable > 0 {
                        self.notice = Some(format!(
                            "Nie można odczytać części plików lub folderów ({0}). Pierwszy błąd: {1}",
                            collected.unreadable,
                            collected.first_error.unwrap_or_default()
                        ));
                    } else if collected.files.is_empty() {
                        self.notice = Some(
                            "Nie znaleziono obsługiwanych obrazów w dodanych folderach.".into(),
                        );
                    }
                    self.enqueue(collected.files);
                }
                Err(mpsc::TryRecvError::Empty) => index += 1,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.scans.remove(index);
                    self.notice = Some(
                        "Wyszukiwanie obrazów zostało przerwane. Dodaj folder ponownie.".into(),
                    );
                }
            }
        }
    }

    fn enqueue(&mut self, paths: Vec<PathBuf>) {
        if self.close_when_idle || paths.is_empty() {
            return;
        }
        let mut staged: std::collections::HashSet<_> = self
            .rows
            .iter()
            .filter(|row| matches!(row.status, Status::Ready) || row.status.pending())
            .map(|row| row.input.clone())
            .collect();
        for input in paths {
            // A staged or submitted file should appear only once until its job ends.
            if !staged.insert(input.clone()) {
                continue;
            }
            let id = self.next_id;
            self.next_id += 1;
            let status = if !optimizer::supported(&input) {
                Status::Failed(
                    if cfg!(target_os = "macos") {
                        "Wybierz PNG, JPG, WebP, AVIF, JXL, AI/PDF, EPS, TIFF, HEIC lub SVG"
                    } else {
                        "Wybierz PNG, JPG, WebP, AVIF, JXL, AI/PDF, EPS lub SVG"
                    }
                    .into(),
                )
            } else {
                Status::Ready
            };
            self.rows.push(Row {
                id,
                input,
                settings: self.settings.clone(),
                status,
            });
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Started(id) => {
                if let Some(row) = self.rows.iter_mut().find(|r| r.id == id) {
                    row.status = Status::Working;
                }
            }
            Event::Finished(id, result) => {
                if let Some(row) = self.rows.iter_mut().find(|r| r.id == id) {
                    row.status = match result {
                        Ok(result) => Status::Done(result),
                        Err(error) if error == "Anulowano" => Status::Cancelled,
                        Err(error) => Status::Failed(error),
                    };
                }
            }
        }
    }

    fn can_prepare_again(row: &Row) -> bool {
        matches!(
            row.status,
            Status::Done(_) | Status::Failed(_) | Status::Cancelled
        ) && optimizer::supported(&row.input)
    }

    /// Stage original inputs only; the normal Start button remains the sole dispatcher.
    fn prepare_again(&mut self, id: Option<u64>) {
        if self.close_when_idle || self.pending() > 0 {
            return;
        }
        let mut staged: std::collections::HashSet<PathBuf> = self
            .rows
            .iter()
            .filter(|row| matches!(row.status, Status::Ready))
            .map(|row| row.input.clone())
            .collect();
        let mut count = 0;
        for row in &mut self.rows {
            if id.is_none_or(|id| id == row.id)
                && Self::can_prepare_again(row)
                && staged.insert(row.input.clone())
            {
                row.status = Status::Ready;
                count += 1;
            }
        }
        if count > 0 {
            self.detail_row = None;
            self.notice = Some("Pliki przygotowane ponownie. Zmień opcje i kliknij „Rozpocznij konwersję”. Nadpisane oryginały nie są przywracane.".into());
        }
    }

    fn ready(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| matches!(row.status, Status::Ready))
            .count()
    }

    /// Only the explicit start action may submit work. Settings are captured here,
    /// not on drop, so staged files follow every edit made before confirmation.
    fn start_conversion(&mut self) {
        if self.close_when_idle
            || !self.scans.is_empty()
            || self.pending() > 0
            || self.ready() == 0
            || !self.destination.configured()
        {
            return;
        }
        self.batch_total = self.ready();
        self.cancel = Arc::new(AtomicBool::new(false));
        self.notice = None;
        for row in &mut self.rows {
            if !matches!(row.status, Status::Ready) {
                continue;
            }
            row.settings = self.settings.clone();
            row.settings.overwrite_originals =
                self.destination.mode == DestinationMode::OverwriteOriginals;
            row.status = match self.destination.resolve(&row.input, &self.downloads) {
                Ok(output) => {
                    let job = Job {
                        id: row.id,
                        input: row.input.clone(),
                        output,
                        settings: row.settings.clone(),
                        cancel: self.cancel.clone(),
                    };
                    match self.jobs.send(job) {
                        Ok(()) => Status::Waiting,
                        Err(_) => Status::Failed(
                            "Wątek konwersji jest niedostępny. Uruchom aplikację ponownie.".into(),
                        ),
                    }
                }
                Err(error) => Status::Failed(error.clone()),
            };
        }
    }

    fn start_button(&mut self, ui: &mut egui::Ui) {
        let ready = self.ready();
        let pending = self.pending();
        if ready == 0 && pending == 0 {
            return;
        }
        let p = Palette::of(ui);
        let label = if ready > 0 {
            f("Rozpocznij konwersję · {0}", &[&ready])
        } else {
            t("Konwersja w toku…").to_owned()
        };
        let response = ui.add_enabled(
            ready > 0 && pending == 0 && self.scans.is_empty() && self.destination.configured(),
            egui::Button::new(
                RichText::new(label)
                    .size(14.0)
                    .variation("wght", 600.0)
                    .color(Color32::WHITE),
            )
            .fill(p.selected)
            .corner_radius(11)
            .min_size(Vec2::new(ui.available_width(), 40.0)),
        );
        if response.on_hover_text(t("Uruchamia oczekujące pliki z aktualnymi ustawieniami. Dodanie plików nie rozpoczyna konwersji.")).clicked() {
            self.start_conversion();
        }
        if ready > 0 && pending > 0 {
            ui.label(
                RichText::new(t("Nowe pliki uruchomisz po zakończeniu paczki."))
                    .size(12.0)
                    .color(p.muted),
            );
        }
    }

    fn pick_files(&mut self, ctx: &egui::Context) {
        if let Some(files) = rfd::FileDialog::new()
            .set_title(t("Dodaj obrazy"))
            .add_filter(t("Obrazy i dokumenty"), optimizer::INPUT_EXTENSIONS)
            .pick_files()
        {
            self.import_paths(files, ctx);
        }
    }

    fn remember_destination(&mut self) {
        if !self.destination.save() {
            self.notice =
                Some("Nie udało się zapamiętać miejsca zapisu. Bieżący wybór nadal działa.".into());
        }
    }

    fn choose_output_folder(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title(t("Wybierz folder wynikowy"));
        if let Some(path) = &self.destination.custom
            && path.is_dir()
        {
            dialog = dialog.set_directory(path);
        }
        if let Some(path) = dialog.pick_folder() {
            self.destination.custom = Some(path);
            self.destination.mode = DestinationMode::Custom;
            self.remember_destination();
        }
    }

    fn open_folder(&mut self, path: &std::path::Path) {
        let result = std::fs::create_dir_all(path).and_then(|()| crate::platform::open(path));
        if let Err(err) = result {
            self.notice = Some(format!("Nie można otworzyć folderu: {err}"));
        }
    }

    fn cancel_queue(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.notice = Some("Anulowanie kolejki…".into());
    }

    fn pending(&self) -> usize {
        self.rows.iter().filter(|row| row.status.pending()).count()
    }

    fn card(ui: &egui::Ui) -> egui::Frame {
        let p = Palette::of(ui);
        egui::Frame::new()
            .fill(p.surface)
            .corner_radius(16)
            .inner_margin(20)
            .stroke(egui::Stroke::new(1.0, p.border.gamma_multiply(0.7)))
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        let p = Palette::of(ui);
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), egui::Sense::hover());
        let drag = egui::Rect::from_min_max(
            rect.min + Vec2::new(80.0, 0.0),
            rect.max - Vec2::new(124.0, 0.0),
        );
        let response = ui.interact(
            drag,
            ui.scope_id().with("titlebar-drag"),
            egui::Sense::click_and_drag(),
        );
        if response.drag_started() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if response.double_clicked() {
            let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
        }
        ui.painter().text(
            rect.center() - Vec2::new(0.0, 7.0),
            egui::Align2::CENTER_CENTER,
            "WebDrop",
            egui::FontId::new(13.0, egui::FontFamily::Name("Semibold".into())),
            p.ink,
        );
        ui.painter().text(
            rect.center() + Vec2::new(0.0, 9.0),
            egui::Align2::CENTER_CENTER,
            "Image Optimizer",
            egui::FontId::proportional(10.0),
            p.muted,
        );
        let button = egui::Rect::from_center_size(
            egui::pos2(rect.right() - 56.0, rect.center().y),
            Vec2::new(112.0, 32.0),
        );
        let response = ui.put(button, egui::Button::new("").frame(false));
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, t("Ustawienia")));
        let color = if response.hovered() || response.has_focus() {
            p.accent
        } else {
            p.muted
        };
        appearance::gear_mark(
            ui.painter(),
            egui::Rect::from_center_size(
                egui::pos2(button.left() + 16.0, button.center().y),
                Vec2::splat(17.0),
            ),
            color,
        );
        ui.painter().text(
            egui::pos2(button.left() + 32.0, button.center().y),
            egui::Align2::LEFT_CENTER,
            t("Ustawienia"),
            egui::FontId::proportional(12.0),
            color,
        );
        if response
            .on_hover_text(t("Kompresja, import, język i informacje o aplikacji · ⌘,"))
            .clicked()
        {
            self.advanced = true;
        }
    }

    fn drop_zone(&mut self, ui: &mut egui::Ui, hovering: bool, height: f32, compact: bool) {
        let p = Palette::of(ui);
        egui::Frame::new()
            .fill(if hovering { p.tint } else { p.surface })
            .stroke(egui::Stroke::new(
                if hovering { 2.0 } else { 1.0 },
                if hovering { p.accent } else { p.border },
            ))
            .corner_radius(18)
            .inner_margin(if compact { 12 } else { 16 })
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                if compact {
                    ui.horizontal(|ui| {
                        let (r, _) =
                            ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                        appearance::image_mark(ui.painter(), r.shrink(2.0), p.accent);
                        ui.label(
                            RichText::new(if hovering {
                                t("Upuść obrazy lub foldery")
                            } else {
                                t("Obrazy lub foldery")
                            })
                            .size(13.0),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(t("+  Dodaj pliki")).color(p.accent),
                                    )
                                    .fill(p.tint),
                                )
                                .on_hover_text(t("Wybierz jeden lub wiele plików · ⌘O"))
                                .clicked()
                            {
                                self.pick_files(ui.ctx());
                            }
                        });
                    });
                } else {
                    let tall = height >= 240.0;
                    let medium = height >= 190.0;
                    let content_height = if tall {
                        200.0
                    } else if medium {
                        148.0
                    } else {
                        84.0
                    };
                    ui.set_min_height((height - 32.0).max(content_height));
                    ui.vertical_centered(|ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        ui.add_space(((height - 32.0 - content_height) / 2.0).max(0.0));
                        if tall {
                            appearance::upload_mark(ui);
                        }
                        ui.label(
                            RichText::new(if hovering {
                                t("Upuść obrazy lub foldery")
                            } else {
                                t("Przeciągnij obrazy lub foldery")
                            })
                            .size(20.0)
                            .variation("wght", 600.0)
                            .strong(),
                        );
                        if medium {
                            ui.label(
                                RichText::new(t(
                                    "Foldery obejmują podfoldery. Konwersję uruchomisz przyciskiem.",
                                ))
                                .size(12.0)
                                .color(p.muted),
                            );
                        }
                        ui.add_space(2.0);
                        if ui
                            .add_sized(
                                [190.0, 38.0],
                                egui::Button::new(
                                    RichText::new(t("+   Wybierz pliki"))
                                        .size(14.0)
                                        .variation("wght", 600.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(p.selected)
                                .stroke(egui::Stroke::NONE)
                                .corner_radius(11),
                            )
                            .on_hover_text(t("Jeden obraz lub wiele plików naraz · ⌘O"))
                            .clicked()
                        {
                            self.pick_files(ui.ctx());
                        }
                        if medium {
                            ui.label(
                                RichText::new(if cfg!(target_os = "macos") {
                                    "PNG · JPG · WebP · AVIF · JXL · AI/EPS · TIFF · HEIC · SVG"
                                } else {
                                    "PNG · JPG · WebP · AVIF · JXL · AI/PDF · EPS · SVG"
                                })
                                .size(12.0)
                                .color(p.muted),
                            );
                        }
                    });
                }
            });
    }

    fn logo_preset(&mut self) {
        self.settings.format = OutputFormat::Webp;
        self.settings.webp_mode = WebpMode::Lossless;
        self.settings.max_width = Some(1500);
        self.settings.trim = TrimMode::Background;
        self.settings.trim_tolerance = 8;
    }

    fn export_sidebar(&mut self, ui: &mut egui::Ui) -> (egui::Rect, egui::Rect) {
        let p = Palette::of(ui);
        ui.spacing_mut().item_spacing.y = 8.0;
        ui.label(
            RichText::new(t("Opcje eksportu"))
                .size(20.0)
                .variation("wght", 600.0),
        );
        egui::Panel::bottom("export-actions")
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(p.background)
                    .inner_margin(egui::Margin {
                        top: 14,
                        bottom: 0,
                        left: 0,
                        right: 0,
                    }),
            )
            .show(ui, |ui| {
                self.destination(ui);
                self.start_button(ui);
                if self.ready() == 0 && self.pending() == 0 {
                    ui.add_enabled(
                        false,
                        egui::Button::new(t("Rozpocznij konwersję"))
                            .min_size(Vec2::new(ui.available_width(), 40.0)),
                    );
                }
            });
        let available = ui.available_rect_before_wrap();
        let content = ui.scope(|ui| self.settings_ui(ui)).response.rect;
        (content, available)
    }

    fn workspace_ui(&mut self, ui: &mut egui::Ui, hovering: bool) {
        let p = Palette::of(ui);
        let bounds = ui.available_rect_before_wrap();
        let gap = 28.0;
        let sidebar_width = 380.0;
        let split = bounds.right() - sidebar_width;
        ui.painter().vline(
            split - gap / 2.0,
            bounds.y_range(),
            egui::Stroke::new(1.0, p.border),
        );
        let left = egui::Rect::from_min_max(bounds.min, egui::pos2(split - gap, bounds.bottom()));
        let right = egui::Rect::from_min_max(egui::pos2(split, bounds.top()), bounds.max);
        ui.scope_builder(
            egui::UiBuilder::new()
                .id_salt("files-column")
                .max_rect(left),
            |ui| {
                if !self.scans.is_empty() {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(100));
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new(t("Wyszukiwanie obrazów…")).size(12.0));
                        if ui.small_button(t("Anuluj")).clicked() {
                            self.scans.clear();
                        }
                    });
                    ui.add_space(6.0);
                }
                if let Some(notice) = self.notice.clone() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(i18n::message(&notice))
                                .size(11.0)
                                .color(p.muted),
                        );
                        if ui.small_button("×").clicked() {
                            self.notice = None;
                        }
                    });
                }
                if self.rows.is_empty() {
                    self.drop_zone(ui, hovering, ui.available_height() - 2.0, false);
                } else {
                    self.drop_zone(ui, hovering, 0.0, true);
                    ui.add_space(4.0);
                    self.queue_ui(ui);
                }
            },
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .id_salt("options-column")
                .max_rect(right),
            |ui| {
                self.export_sidebar(ui);
            },
        );
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let p = Palette::of(ui);
        let web = self.settings.format == OutputFormat::Webp
            && self.settings.webp_mode == WebpMode::Lossy
            && self.settings.max_width == Some(1500)
            && self.settings.quality == 82
            && self.settings.trim == TrimMode::Off;
        let logo = self.settings.format == OutputFormat::Webp
            && self.settings.webp_mode == WebpMode::Lossless
            && self.settings.max_width == Some(1500)
            && self.settings.trim == TrimMode::Background
            && self.settings.trim_tolerance == 8;
        ui.columns(2, |columns| {
            if appearance::preset(
                &mut columns[0],
                t("Na web"),
                t("WebP · 1500 px"),
                web,
                false,
            )
            .on_hover_text(t("Dla zdjęć i obrazów na stronę: WebP stratny, jakość 82, do 1500 px szerokości. Zostawia marginesy."))
            .clicked()
            {
                self.settings.format = OutputFormat::Webp;
                self.settings.webp_mode = WebpMode::Lossy;
                self.settings.max_width = Some(1500);
                self.settings.quality = 82;
                self.settings.trim = TrimMode::Off;
            }
            if appearance::preset(
                &mut columns[1],
                "Logo",
                t("Bezstratnie · trim"),
                logo,
                true,
            )
            .on_hover_text(
                t("Dla logo: WebP bezstratny, do 1500 px. Przycina jednolite i przezroczyste brzegi, potem skaluje. SVG pozostaje SVG."),
            )
            .clicked()
            {
                self.logo_preset();
            }
        });
        Self::card(ui).inner_margin(16).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 6.0;
            Self::export_row(ui, "Format", |ui| {
                egui::ComboBox::from_id_salt("output-format")
                    .selected_text(t(self.settings.format.label())).width(ui.available_width()).truncate().show_ui(ui, |ui| {
                        for format in [OutputFormat::Hybrid, OutputFormat::Webp, OutputFormat::Png, OutputFormat::Jpeg, OutputFormat::Original, OutputFormat::Avif, OutputFormat::Jxl] {
                            Self::explained_option(ui, &mut self.settings.format, format, t(format.label()), help::format_description(format));
                        }
                    });
            });
            Self::setting_hint(ui, help::format_description(self.settings.format));
            Self::export_row(ui, t("Szerokość"), |ui| {
                ui.horizontal(|ui| {
                    let selected = match self.settings.max_width {
                        None => t("Pełna").to_owned(),
                        Some(width @ (1500 | 1000 | 700 | 500)) => format!("{width} px"),
                        Some(_) => t("Własna szerokość").to_owned(),
                    };
                    egui::ComboBox::from_id_salt("resize-width")
                        .selected_text(selected).width((ui.available_width() - 100.0).max(90.0)).truncate()
                        .show_ui(ui, |ui| {
                            for width in [1500, 1000, 700, 500] {
                                ui.selectable_value(&mut self.settings.max_width, Some(width), format!("{width} px"));
                            }
                            ui.selectable_value(&mut self.settings.max_width, None, t("Pełna"));
                            let custom = self.settings.max_width.is_some_and(|w| ![1500, 1000, 700, 500].contains(&w));
                            if ui.selectable_label(custom, t("Własna szerokość")).clicked() && !custom {
                                self.settings.max_width = Some(1200);
                            }
                        });
                    if let Some(width) = &mut self.settings.max_width {
                        ui.add_sized([90.0, 24.0], egui::DragValue::new(width).range(1..=65_535).speed(10.0).suffix(" px"))
                            .on_hover_text(t("Wpisz dowolną szerokość od 1 do 65535 px. Wysokość jest proporcjonalna. Mniejsze obrazy nie są powiększane."));
                    }
                });
            });
            Self::export_row(ui, t("Marginesy"), |ui| {
                egui::ComboBox::from_id_salt("trim").selected_text(help::trim_label(self.settings.trim))
                    .width(ui.available_width()).truncate().show_ui(ui, |ui| {

                        ui.spacing_mut().item_spacing.y = 4.0;
                        for mode in [TrimMode::Off, TrimMode::Transparent, TrimMode::Background] {
                            Self::explained_option(ui, &mut self.settings.trim, mode, help::trim_label(mode), help::trim_description(mode));
                        }
                        ui.separator();
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| { ui.add(egui::Label::new(RichText::new(t("Przycinanie działa przed skalowaniem. SVG: oba tryby przycinają tylko obszar dokumentu.")).size(12.0).color(p.muted)).wrap()); });
                    });
            });
            Self::setting_hint(ui, help::trim_description(self.settings.trim));
            Self::export_row(ui, t("Kompresja"), |ui| {
                match self.settings.format {
                    OutputFormat::Webp | OutputFormat::Hybrid => {
                        ui.horizontal(|ui| {
                            let width = (ui.available_width() - 134.0).max(120.0);
                            egui::ComboBox::from_id_salt("main-webp-mode").selected_text(match self.settings.webp_mode {
                                WebpMode::Lossy => t("Stratna"), WebpMode::Lossless => t("Bezstratna"), WebpMode::NearLossless => t("Prawie bezstratna"),
                            }).width(width).truncate().show_ui(ui, |ui| {

                                for (mode, label) in [(WebpMode::Lossy, t("Stratna · zdjęcia")), (WebpMode::Lossless, t("Bezstratna · logo i grafiki")),
                                    (WebpMode::NearLossless, t("Prawie bezstratna"))] { Self::explained_option(ui, &mut self.settings.webp_mode, mode, label, help::webp_description(mode)); }
                            });
                            if self.settings.webp_mode == WebpMode::Lossless {
                                ui.label(RichText::new(t("Bez utraty pikseli")).size(12.0).color(p.green))
                                    .on_hover_text(t("Dotyczy kodowania po renderowaniu, trimie i skalowaniu. Suwak jakości nie ma wpływu na ten tryb."));
                            } else {
                                ui.label(RichText::new(t("Jakość")).size(12.0).color(p.muted));
                                ui.add(egui::DragValue::new(&mut self.settings.quality).range(1..=100).speed(1.0))
                                    .on_hover_text(t("Jakość 1–100. Kliknij, aby wpisać, lub przeciągnij wartość."));
                            }
                        });
                    }
                    OutputFormat::Avif | OutputFormat::Jxl => {
                        let lossless = if self.settings.format == OutputFormat::Avif { &mut self.settings.avif_lossless } else { &mut self.settings.jxl_lossless };
                        ui.horizontal(|ui| {
                            let width = (ui.available_width() - 134.0).max(120.0);
                            egui::ComboBox::from_id_salt("modern-mode").selected_text(if *lossless { t("Bezstratna") } else { t("Stratna") })
                                .width(width).truncate().show_ui(ui, |ui| {
                                    for mode in [false, true] {
                                        Self::explained_option(ui, lossless, mode, if mode { t("Bezstratna · logo i grafiki") } else { t("Stratna · zdjęcia") }, help::modern_description(mode));
                                    }
                                });
                            if *lossless {
                                ui.label(RichText::new(t("Bez utraty pikseli")).size(12.0).color(p.green))
                                    .on_hover_text(t("Dotyczy kodowania po renderowaniu, trimie i skalowaniu. Suwak jakości nie ma wpływu na ten tryb."));
                            } else {
                                ui.label(RichText::new(t("Jakość")).size(12.0).color(p.muted));
                                ui.add(egui::DragValue::new(&mut self.settings.quality).range(1..=100).speed(1.0));
                            }
                        });
                    }
                    OutputFormat::Png => {
                        egui::ComboBox::from_id_salt("main-png-mode").selected_text(t(self.settings.png_mode.label()))
                            .width(ui.available_width()).truncate().show_ui(ui, |ui| {

                                for mode in [PngMode::Auto, PngMode::Zopfli, PngMode::Lossless, PngMode::Palette] {
                                    Self::explained_option(ui, &mut self.settings.png_mode, mode, t(mode.label()), help::png_description(mode));
                                }
                            });
                    }
                    OutputFormat::Jpeg => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(t("Jakość")).color(p.muted));
                            ui.add(egui::Slider::new(&mut self.settings.quality, 1..=100));
                        });
                    }
                    OutputFormat::Original => {
                        ui.label(RichText::new(t("Ustawienia według formatu")).size(12.0).color(p.muted))
                            .on_hover_text(t("Szczegóły zmienisz w Ustawieniach w prawym górnym rogu."));
                    }
                }
            });
            Self::setting_hint(ui, help::compression_description(&self.settings));
        });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(RichText::new("↔").size(13.0).color(p.accent));
            ui.label(RichText::new(match self.settings.max_width { Some(width) => f("Do {0} px", &[&width]), None => t("Pełna szerokość").into() }).size(11.0).color(p.muted))
                .on_hover_text(t("1500 px to maksymalna szerokość, nie obowiązkowy rozmiar. Obraz 3000 × 2000 px stanie się 1500 × 1000 px. Obraz 800 × 600 px pozostanie bez zmian. Po przycięciu marginesów rozmiar liczony jest od przyciętego obrazu. SVG nie jest skalowany."));
            ui.add_space(8.0);
                ui.label(RichText::new(t("SVG: tylko marginesy")).size(11.0).color(p.muted))
                    .on_hover_text(t("SVG pozostaje SVG. Bez optymalizacji, konwersji i usuwania metadanych. Trim zmienia wyłącznie obszar dokumentu."));
        });
    }

    fn setting_hint(ui: &mut egui::Ui, text: &str) {
        let p = Palette::of(ui);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            if ui.available_width() >= 420.0 {
                ui.add_space(88.0);
            }
            ui.add(
                egui::Label::new(
                    RichText::new(text)
                        .size(12.0)
                        .line_height(Some(17.0))
                        .color(p.muted),
                )
                .wrap(),
            );
        });
    }

    fn explained_option<T: Copy + PartialEq>(
        ui: &mut egui::Ui,
        value: &mut T,
        item: T,
        title: &str,
        description: &str,
    ) {
        if ui
            .selectable_value(value, item, RichText::new(title).variation("wght", 500.0))
            .clicked()
        {
            ui.close();
        }
        // Popup rows are justified by egui. Descriptions must keep natural word spacing.
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin {
                    left: 12,
                    right: 12,
                    top: 0,
                    bottom: 6,
                })
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(description)
                                .size(12.0)
                                .line_height(Some(17.0))
                                .color(Palette::of(ui).muted),
                        )
                        .wrap(),
                    );
                });
        });
    }

    fn export_row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(72.0, 32.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_size(Vec2::new(72.0, 32.0));
                    ui.label(RichText::new(label).size(12.0).color(Palette::of(ui).muted));
                },
            );
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                add(ui);
            });
        });
    }

    fn setting_row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
        let p = Palette::of(ui);
        if ui.available_width() < 420.0 {
            ui.label(
                RichText::new(label)
                    .size(12.5)
                    .variation("wght", 600.0)
                    .color(p.ink),
            );
            add(ui);
            return;
        }
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(78.0, 34.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_size(Vec2::new(78.0, 34.0));
                    ui.label(RichText::new(label).size(12.5).color(p.muted));
                },
            );
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                add(ui);
            });
        });
    }

    fn compression_window(&mut self, ctx: &egui::Context) {
        if !self.advanced {
            return;
        }
        let modal = egui::Modal::new(egui::Id::unique("settings-window"))
            .frame(egui::Frame::window(&ctx.global_style()).inner_margin(24).corner_radius(18))
            .backdrop_color(Color32::from_black_alpha(115))
            .show(ctx, |ui| {
                ui.set_width((ctx.content_rect().width() - 96.0).clamp(340.0, 460.0));
                let p = Palette::of(ui);
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t("Ustawienia")).size(20.0).variation("wght", 600.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized([30.0, 30.0], egui::Button::new("×").frame(false)).on_hover_text(t("Zamknij")).clicked() { self.advanced = false; }
                    });
                });
                segments(ui, &mut self.settings_tab, &[(0, t("Kompresja")), (1, "Import"), (2, t("Ogólne")), (3, t("O aplikacji"))]);
                egui::ScrollArea::vertical().auto_shrink([false, true]).id_salt(("settings-content", self.settings_tab))
                    .max_height((ctx.content_rect().height() - 240.0).clamp(200.0, 430.0))
                    .show(ui, |ui| {
                    ui.set_min_height(240.0);
                    ui.add_space(8.0);
                    match self.settings_tab {
                        0 => {
                            ui.label(RichText::new(t("Kontrola nad jakością")).size(18.0).variation("wght", 600.0).strong());
                            Self::setting_row(ui, t("Jakość"), |ui| {
                                let lossless = match self.settings.format {
                                    OutputFormat::Webp | OutputFormat::Hybrid => self.settings.webp_mode == WebpMode::Lossless,
                                    OutputFormat::Avif => self.settings.avif_lossless,
                                    OutputFormat::Jxl => self.settings.jxl_lossless,
                                    _ => false,
                                };
                                ui.add_enabled(!lossless, egui::Slider::new(&mut self.settings.quality, 1..=100).suffix(" / 100"));
                            });
                            ui.label(RichText::new(t("Jakość 1–100: wyżej = więcej szczegółów i zwykle większy plik. 82 nie oznacza 82% rozmiaru ani jakości oryginału.")).size(12.0).color(p.muted));
                            Self::setting_row(ui, "WebP", |ui| {
                                egui::ComboBox::from_id_salt("webp-mode").selected_text(t(self.settings.webp_mode.label()))
                                    .width(ui.available_width()).truncate().show_ui(ui, |ui| {

                                        for mode in [WebpMode::Lossy, WebpMode::Lossless, WebpMode::NearLossless] { Self::explained_option(ui, &mut self.settings.webp_mode, mode, t(mode.label()), help::webp_description(mode)); }
                                    });
                            });
                            Self::setting_row(ui, "PNG", |ui| {
                                egui::ComboBox::from_id_salt("png-mode").selected_text(t(self.settings.png_mode.label()))
                                    .width(ui.available_width()).truncate().show_ui(ui, |ui| {

                                for mode in [PngMode::Auto, PngMode::Zopfli, PngMode::Lossless, PngMode::Palette] { Self::explained_option(ui, &mut self.settings.png_mode, mode, t(mode.label()), help::png_description(mode)); }
                                    });
                            });
                            for (label, lossless) in [("AVIF", &mut self.settings.avif_lossless), ("JPEG XL", &mut self.settings.jxl_lossless)] {
                                Self::setting_row(ui, label, |ui| {
                                    segments(ui, lossless, &[(false, t("Stratna")), (true, t("Bezstratna"))]);
                                });
                            }
                            ui.label(RichText::new(t("AVIF/JXL: jakość dotyczy trybu stratnego. Bezstratny zachowuje piksele gotowego obrazu w 8-bitowym sRGB.")).size(12.0).color(p.muted));
                            ui.checkbox(&mut self.settings.thorough, t("Dokładniejsza kompresja"));
                            ui.label(RichText::new(t("Poświęca więcej czasu na kompresję. Zachowuje wybrany tryb i jakość; nie gwarantuje mniejszego wyniku.")).size(12.0).color(p.muted));
                            egui::CollapsingHeader::new(t("Co oznacza bezstratnie?")).show(ui, |ui| {
                                ui.label(t("Bezstratny WebP zachowuje piksele gotowego obrazu po renderowaniu, przycięciu i skalowaniu. Jakość 1–100 nie wpływa na ten tryb. Dla prostych logo może dać mniejszy plik niż kompresja stratna."));
                            });
                        }
                        1 => {
                            ui.label(RichText::new(t("Import dokumentów i marginesy")).size(18.0).variation("wght", 600.0).strong());
                            #[cfg(target_os = "macos")]
                            {
                            Self::setting_row(ui, "AI / PDF / EPS", |ui| {
                                ui.horizontal(|ui| {
                                    egui::ComboBox::from_id_salt("dpi").selected_text(format!("{} DPI", self.settings.vector_dpi))
                                        .width(100.0).show_ui(ui, |ui| {
                                            for dpi in [72,144,300,600] { ui.selectable_value(&mut self.settings.vector_dpi,dpi,format!("{dpi} DPI")); }
                                        });
                                    ui.label(t("Strona"));
                                    ui.add(egui::DragValue::new(&mut self.settings.page).range(1..=9999))
                                        .on_hover_text(t("Strona AI/PDF lub wielostronicowego TIFF"));
                                });
                            });
                            ui.label(RichText::new(t("DPI określa szczegółowość renderowania AI/PDF/EPS do pikseli. Więcej DPI = więcej szczegółów, pamięci i czasu. 300 to domyślna wartość; szerokość wyniku ustawiasz osobno.")).size(12.0).color(p.muted));
                            ui.label(RichText::new(t("Strona: wybiera jedną stronę AI/PDF lub wielostronicowego TIFF. HEIC używa obrazu głównego.")).size(12.0).color(p.muted));
                            }
                            #[cfg(not(target_os = "macos"))]
                            {
                                Self::setting_row(ui, "EPS", |ui| {
                                    egui::ComboBox::from_id_salt("eps-dpi").selected_text(format!("{} DPI", self.settings.vector_dpi))
                                        .show_ui(ui, |ui| {
                                            for dpi in [72,144,300,600] { ui.selectable_value(&mut self.settings.vector_dpi,dpi,format!("{dpi} DPI")); }
                                        });
                                });
                                Self::setting_row(ui, t("Strona"), |ui| { ui.add(egui::DragValue::new(&mut self.settings.page).range(1..=9999)); });
                                ui.label(t("AI/PDF na Windows/Linux: tylko tryb hybrydowy, wymagany MuPDF. TIFF i HEIC: tylko macOS."));
                            }
                            ui.label(RichText::new(t("EPS: renderowanie na przezroczystym tle przez Ghostscript. Zawiera jedną stronę; preset Logo przycina marginesy. Na Windows/Linux zainstaluj Ghostscript.")).size(12.0).color(p.muted));
                            Self::setting_row(ui, t("Tolerancja"), |ui| {
                                ui.add(egui::Slider::new(&mut self.settings.trim_tolerance,0..=64));
                            });
                            ui.label(RichText::new(t("0 przycina tylko całkowicie przezroczyste piksele lub tło o identycznym kolorze. Wyższa wartość uznaje też prawie przezroczyste i podobne kolory za margines. Może przyciąć delikatny cień. Domyślnie: 8. Nie dotyczy SVG.")).size(12.0).color(p.muted));
                            egui::CollapsingHeader::new(t("Jak działa przycinanie marginesów (trim)?")).show(ui, |ui| {
                                for mode in [TrimMode::Off, TrimMode::Transparent, TrimMode::Background] {
                                    ui.label(RichText::new(help::trim_label(mode)).variation("wght", 600.0).strong());
                                    ui.label(help::trim_description(mode));
                                }
                                ui.label(t("Najpierw przycinamy, potem skalujemy. Jednolite tło rozpoznajemy po czterech narożnikach; jeśli się różnią, przycinamy tylko przezroczystość. Całkiem pusty obraz pozostaje bez zmian."));
                            });
                            ui.label(RichText::new(t("SVG jest chroniony")).variation("wght", 600.0).strong().color(p.accent));
                            ui.label(RichText::new(t("Zawsze pozostaje SVG. Trim zmienia wyłącznie obszar dokumentu. Bez optymalizacji, skalowania i usuwania metadanych.")).size(12.0).color(p.muted));
                            #[cfg(target_os = "macos")]
                            egui::CollapsingHeader::new(t("AI, TIFF i HEIC")).show(ui, |ui| {
                                ui.label(t("AI musi być zapisany ze zgodnością PDF. HEIC używa obrazu głównego. Numer strony dotyczy AI/PDF i wielostronicowego TIFF. Format „Oryginał” zapisuje te wejścia jako WebP."));
                            });
                        }
                        2 => self.general_settings(ui, ctx),
                        _ => self.about(ui),
                    }
                });
                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Reserve the action first; the explanation wraps within the remaining space.
                    if ui.add_sized([84.0, 34.0], egui::Button::new(RichText::new(t("Gotowe")).color(Color32::WHITE)).fill(p.selected)).clicked() { self.advanced=false; }
                    if self.settings_tab < 2 {
                        ui.add(egui::Label::new(RichText::new(t("Zmiany dotyczą plików czekających na start. Trwająca konwersja zachowuje swoje ustawienia.")).size(12.0).color(p.muted)).wrap());
                    }
                });
            });
        if modal.should_close() {
            self.advanced = false;
        }
    }

    fn general_settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let p = Palette::of(ui);
        ui.label(
            RichText::new(t("Język / Language"))
                .size(18.0)
                .variation("wght", 600.0)
                .strong(),
        );
        let previous = self.language;
        segments(
            ui,
            &mut self.language,
            &[
                (Language::System, t("Systemowy")),
                (Language::Polish, "Polski · PL"),
                (Language::English, "English · EN"),
            ],
        );
        if self.language != previous {
            self.system_language = i18n::system_language();
            if !self.language.save() {
                self.notice = Some(
                    "Nie udało się zapisać języka. Wybór działa do zamknięcia aplikacji.".into(),
                );
            }
            self.language.activate(&self.system_language);
            ctx.request_repaint();
        }
        if self.language == Language::System {
            ui.label(
                RichText::new(f(
                    "Język systemu: {0}. Dla języków innych niż polski używamy angielskiego.",
                    &[&self.system_language],
                ))
                .size(12.0)
                .color(p.muted),
            );
        }
        ui.label(
            RichText::new(t("Wybór języka jest zapamiętywany. Zmiana działa od razu."))
                .size(12.0)
                .color(p.muted),
        );
        ui.separator();
        ui.label(RichText::new(t("Wygląd")).variation("wght", 600.0).strong());
        segments(
            ui,
            &mut self.appearance,
            &[
                (egui::ThemePreference::System, t("Systemowy")),
                (egui::ThemePreference::Light, t("Jasny")),
                (egui::ThemePreference::Dark, t("Ciemny")),
            ],
        );
        ctx.set_theme(self.appearance);
        ui.label(
            RichText::new(t("Tryb systemowy dopasowuje wygląd do ustawień systemu."))
                .size(12.0)
                .color(p.muted),
        );
        egui::CollapsingHeader::new(t("Skróty klawiaturowe"))
            .id_salt("keyboard-shortcuts")
            .show(ui, |ui| {
                ui.label(t(
                    "⌘O   Dodaj pliki       ⌘,   Ustawienia       Esc   Zamknij panel",
                ));
            });
    }

    fn about(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.y = 10.0;
        let p = Palette::of(ui);
        ui.label(
            RichText::new("WebDrop")
                .size(24.0)
                .variation("wght", 600.0)
                .strong(),
        );
        ui.label(RichText::new("Image Optimizer").size(13.0).color(p.muted));
        ui.label(RichText::new(f("Wersja {0}", &[&env!("CARGO_PKG_VERSION")])).color(p.muted));
        ui.separator();
        ui.label(RichText::new(t("Autor")).size(12.0).color(p.muted));
        ui.label(
            RichText::new("Jędrzej Czarnecki")
                .size(16.0)
                .variation("wght", 600.0)
                .strong(),
        );
        ui.horizontal(|ui| {
            self.contact_link(ui, "info@ebest.pl", "mailto:info@ebest.pl");
            ui.label(RichText::new("·").color(p.muted));
            self.contact_link(ui, "ebest.pl  ↗", "https://ebest.pl");
        });
        ui.separator();
        ui.label(
            RichText::new(t("Bezpłatna aplikacja open source"))
                .variation("wght", 600.0)
                .strong(),
        );
        ui.label(RichText::new(t("Możesz używać aplikacji prywatnie i komercyjnie, modyfikować ją oraz udostępniać zgodnie z GNU GPL v3 lub nowszą. Przy rozpowszechnianiu obowiązują warunki tej licencji, w tym udostępnienie kodu źródłowego. Aplikacja jest dostarczana bez gwarancji.")).size(12.0).color(p.muted));
        ui.horizontal_wrapped(|ui| {
            if ui.button(t("Pełna licencja")).clicked() {
                self.open_license("LICENSE");
            }
            if ui.button(t("Licencje bibliotek")).clicked() {
                self.open_license("THIRD-PARTY-NOTICES.txt");
            }
        });
        ui.label(RichText::new(t("Obrazy są przetwarzane lokalnie. PNG, JPG, WebP, AVIF i JXL zawsze zapisują się bez EXIF, GPS i pozostałych metadanych. SVG zachowuje oryginalną zawartość.")).size(12.0).color(p.muted));
    }

    fn contact_link(&mut self, ui: &mut egui::Ui, label: &str, url: &str) {
        // Use LaunchServices, like the Finder actions; no browser dependency is needed.
        if ui.link(label).on_hover_text(url).clicked()
            && let Err(error) = crate::platform::open(url)
        {
            self.notice = Some(format!("Nie można otworzyć linku: {error}"));
        }
    }

    fn open_license(&mut self, name: &str) {
        let bundled = std::env::current_exe().ok().and_then(|path| {
            #[cfg(target_os = "macos")]
            {
                path.parent()?
                    .parent()
                    .map(|p| p.join("Resources").join(name))
            }
            #[cfg(not(target_os = "macos"))]
            {
                path.parent().map(|p| p.join(name))
            }
        });
        let path = bundled.filter(|p| p.is_file()).unwrap_or_else(|| {
            let root = std::env::current_dir().unwrap_or_default();
            if name == "LICENSE" {
                root.join(name)
            } else {
                root.join("artifacts").join(name)
            }
        });
        if let Err(error) = crate::platform::open(path) {
            self.notice = Some(format!("Nie można otworzyć informacji o licencji: {error}"));
        }
    }

    fn queue_ui(&mut self, ui: &mut egui::Ui) {
        let p = Palette::of(ui);
        let pending = self.pending();
        let done = self
            .rows
            .iter()
            .filter(|r| matches!(r.status, Status::Done(_)))
            .count();
        let failed = self
            .rows
            .iter()
            .filter(|r| matches!(r.status, Status::Failed(_)))
            .count();
        ui.horizontal(|ui| {
            let title = if pending > 0 {
                f("Przetwarzanie · {0}", &[&pending])
            } else if self.ready() > 0 {
                f("Czeka na start · {0}", &[&self.ready()])
            } else if failed > 0 {
                f("Gotowe: {0} · Błędy: {1}", &[&done, &failed])
            } else {
                f("Gotowe · {0}/{1}", &[&done, &self.rows.len()])
            };
            ui.label(
                RichText::new(title)
                    .size(13.0)
                    .variation("wght", 600.0)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if pending > 0 {
                    if ui.button(t("Anuluj")).clicked() {
                        self.cancel_queue();
                    }
                } else if ui
                    .add(
                        egui::Button::new(
                            RichText::new(t("Wyczyść listę")).size(12.0).color(p.muted),
                        )
                        .frame(false),
                    )
                    .on_hover_text(t(
                        "Usuwa tylko pozycje z listy. Zapisane pliki pozostają na dysku.",
                    ))
                    .clicked()
                {
                    self.rows.clear();
                    self.scans.clear();
                    self.detail_row = None;
                }
                if pending == 0 && self.rows.iter().any(Self::can_prepare_again)
                    && ui.button(t("Ponów wszystkie"))
                        .on_hover_text(t("Przygotuj te same pliki ponownie, zmień opcje i potwierdź start. Nadpisane oryginały nie są przywracane."))
                        .clicked()
                {
                    self.prepare_again(None);
                }
            });
        });
        if pending > 0 {
            ui.add(
                egui::ProgressBar::new(
                    self.batch_total.saturating_sub(pending) as f32
                        / self.batch_total.max(1) as f32,
                )
                .desired_height(4.0)
                .fill(p.accent)
                .animate(true),
            );
        }
        let mut reveal = None;
        let mut retry = None;
        egui::ScrollArea::vertical()
            .id_salt("file-queue")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 7.0;
                for row in &self.rows {
                    egui::Frame::new()
                        .fill(p.surface)
                        .corner_radius(13)
                        .inner_margin(12)
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let ext = row
                                    .input
                                    .extension()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_uppercase();
                                let color = if matches!(row.status, Status::Failed(_)) {
                                    p.red
                                } else {
                                    p.accent
                                };
                                let (r, _) = ui.allocate_exact_size(
                                    Vec2::new(40.0, 44.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(r, 10, p.tint);
                                ui.painter().text(
                                    r.center(),
                                    egui::Align2::CENTER_CENTER,
                                    ext.chars().take(4).collect::<String>(),
                                    egui::FontId::proportional(11.0),
                                    color,
                                );
                                ui.vertical(|ui| {
                                    ui.set_width((ui.available_width() - 155.0).max(80.0));
                                    ui.spacing_mut().item_spacing.y = 5.0;
                                    let name =
                                        row.input.file_name().unwrap_or_default().to_string_lossy();
                                    let response = ui.add(
                                        egui::Label::new(
                                            RichText::new(name.as_ref())
                                                .size(13.0)
                                                .variation("wght", 600.0)
                                                .strong(),
                                        )
                                        .truncate()
                                        .sense(egui::Sense::click()),
                                    );
                                    response.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::Role::Button,
                                            true,
                                            f("Szczegóły: {0}", &[&name]),
                                        )
                                    });
                                    if response
                                        .on_hover_text(format!(
                                            "{}\n{}",
                                            row.input.display(),
                                            t("Kliknij, aby zobaczyć szczegóły")
                                        ))
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        self.detail_row = Some(row.id);
                                    }
                                    match &row.status {
                                        Status::Ready => {
                                            let format = t(self.settings.format.output_label(&ext));
                                            ui.label(
                                                RichText::new(f(
                                                    "Czeka na start · {0}",
                                                    &[&format],
                                                ))
                                                .size(12.0)
                                                .color(p.muted),
                                            );
                                        }
                                        Status::Waiting => {
                                            ui.label(
                                                RichText::new(f(
                                                    "W kolejce · {0}",
                                                    &[&t(row.settings.format.output_label(&ext))],
                                                ))
                                                .size(12.0)
                                                .color(p.muted),
                                            );
                                        }
                                        Status::Working => {
                                            ui.horizontal(|ui| {
                                                ui.add(
                                                    egui::Spinner::new().size(12.0).color(p.accent),
                                                );
                                                ui.label(
                                                    RichText::new(t("Przetwarzanie…"))
                                                        .size(12.0)
                                                        .color(p.accent),
                                                );
                                            });
                                        }
                                        Status::Done(result) => {
                                            let output_ext = result
                                                .path
                                                .extension()
                                                .unwrap_or_default()
                                                .to_string_lossy()
                                                .to_uppercase();
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(format!(
                                                        "{} → {} · {} × {} · {}",
                                                        optimizer::format_bytes(result.input_bytes),
                                                        optimizer::format_bytes(
                                                            result.output_bytes
                                                        ),
                                                        result.width,
                                                        result.height,
                                                        output_ext
                                                    ))
                                                    .size(12.0)
                                                    .color(p.muted),
                                                )
                                                .truncate(),
                                            )
                                            .on_hover_text(format!(
                                                "{}\n{}\n{}",
                                                result.path.display(),
                                                t(result.encoder),
                                                result
                                                    .notes
                                                    .iter()
                                                    .map(|note| i18n::message(note))
                                                    .collect::<Vec<_>>()
                                                    .join("\n")
                                            ));
                                        }
                                        Status::Failed(error) => {
                                            if ui
                                                .add(
                                                    egui::Label::new(
                                                        RichText::new(i18n::message(error))
                                                            .size(12.0)
                                                            .color(p.red),
                                                    )
                                                    .truncate()
                                                    .sense(egui::Sense::click()),
                                                )
                                                .on_hover_text(t(
                                                    "Kliknij, aby przeczytać cały komunikat",
                                                ))
                                                .clicked()
                                            {
                                                self.detail_row = Some(row.id);
                                            }
                                        }
                                        Status::Cancelled => {
                                            ui.label(
                                                RichText::new(t("Anulowano"))
                                                    .size(12.0)
                                                    .color(p.muted),
                                            );
                                        }
                                    }
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if Self::can_prepare_again(row) {
                                            let response = ui.add_enabled(pending == 0,
                                                egui::Button::new(RichText::new(t("Ponów")).size(12.0))
                                                    .min_size(Vec2::new(52.0, 30.0)));
                                            response.widget_info(|| egui::WidgetInfo::labeled(
                                                egui::Role::Button, pending == 0,
                                                f("Ponów: {0}", &[&row.input.file_name().unwrap_or_default().to_string_lossy()])));
                                            if response.on_hover_text(t("Przygotuj te same pliki ponownie, zmień opcje i potwierdź start. Nadpisane oryginały nie są przywracane.")).clicked() {
                                                retry = Some(row.id);
                                            }
                                        }
                                        if let Status::Done(result) = &row.status {
                                            if ui
                                                .add_sized(
                                                    [30.0, 30.0],
                                                    egui::Button::new(
                                                        RichText::new("↗")
                                                            .size(18.0)
                                                            .color(p.accent),
                                                    )
                                                    .fill(p.tint),
                                                )
                                                .on_hover_text(t("Pokaż wynik w folderze"))
                                                .clicked()
                                            {
                                                reveal = Some(result.path.clone());
                                            }
                                            let savings = result.savings_percent();
                                            let label = if ext == "SVG" {
                                                "✓".into()
                                            } else if savings >= 99.5 {
                                                format!("−{:.1}%", savings.min(99.9))
                                            } else if savings >= 0.0 {
                                                format!("−{savings:.0}%")
                                            } else {
                                                format!("+{:.0}%", -savings)
                                            };
                                            ui.label(
                                                RichText::new(label)
                                                    .size(12.0)
                                                    .variation("wght", 600.0)
                                                    .strong()
                                                    .color(if savings >= 0.0 || ext == "SVG" {
                                                        p.green
                                                    } else {
                                                        p.muted
                                                    }),
                                            );
                                        } else if matches!(row.status, Status::Failed(_))
                                            && ui.button(t("Błąd ›")).clicked()
                                        {
                                            self.detail_row = Some(row.id);
                                        }
                                    },
                                );
                            });
                        });
                }
            });
        if let Some(path) = reveal {
            self.reveal_file(&path);
        }
        if let Some(id) = retry {
            self.prepare_again(Some(id));
        }
    }

    fn reveal_file(&mut self, path: &std::path::Path) {
        if let Err(err) = crate::platform::reveal(path) {
            self.notice = Some(err.to_string());
        }
    }

    fn details_window(&mut self, ctx: &egui::Context) {
        let Some(row) = self
            .detail_row
            .and_then(|id| self.rows.iter().find(|r| r.id == id))
        else {
            return;
        };
        let mut open = true;
        let mut reveal = None;
        let mut retry = None;
        egui::Window::new(t("Szczegóły pliku"))
            .id(egui::Id::unique("details-window"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(430.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                let p = Palette::of(ui);
                egui::ScrollArea::vertical()
                    .max_height(380.0)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(
                                row.input.file_name().unwrap_or_default().to_string_lossy(),
                            )
                            .size(16.0)
                            .variation("wght", 600.0)
                            .strong(),
                        );
                        ui.label(
                            RichText::new(row.input.display().to_string())
                                .size(12.0)
                                .color(p.muted),
                        );
                        ui.separator();
                        match &row.status {
                            Status::Done(result) => {
                                ui.label(
                                    RichText::new(format!(
                                        "{} → {}",
                                        optimizer::format_bytes(result.input_bytes),
                                        optimizer::format_bytes(result.output_bytes)
                                    ))
                                    .size(22.0)
                                    .variation("wght", 600.0)
                                    .strong(),
                                );
                                ui.label(format!(
                                    "{} × {} px · {}",
                                    result.width,
                                    result.height,
                                    t(result.encoder)
                                ));
                                for note in &result.notes {
                                    ui.label(
                                        RichText::new(i18n::message(note))
                                            .size(12.0)
                                            .color(p.muted),
                                    );
                                }
                                ui.separator();
                                ui.label(
                                    RichText::new(t("Zapisany plik"))
                                        .variation("wght", 600.0)
                                        .strong(),
                                );
                                ui.label(result.path.display().to_string());
                                if ui.button(t("Pokaż w folderze  ↗")).clicked() {
                                    reveal = Some(result.path.clone());
                                }
                            }
                            Status::Failed(error) => {
                                ui.label(RichText::new(i18n::message(error)).color(p.red));
                            }
                            Status::Ready => {
                                ui.label(t("Ustaw opcje, a następnie kliknij „Rozpocznij konwersję”. Plik nie został jeszcze przetworzony."));
                            }
                            Status::Cancelled => {
                                ui.label(t("Zadanie anulowano."));
                            }
                            _ => {
                                ui.label(t("Plik jest w kolejce przetwarzania."));
                            }
                        }
                        if Self::can_prepare_again(row) {
                            ui.separator();
                            if ui.add_enabled(self.pending() == 0, egui::Button::new(t("Zmień opcje i ponów"))).clicked() {
                                retry = Some(row.id);
                            }
                            ui.label(RichText::new(t("Przygotuj te same pliki ponownie, zmień opcje i potwierdź start. Nadpisane oryginały nie są przywracane.")).size(12.0).color(p.muted));
                        }
                    });
            });
        if !open {
            self.detail_row = None;
        }
        if let Some(id) = retry {
            self.prepare_again(Some(id));
        }
        if let Some(path) = reveal {
            self.reveal_file(&path);
        }
    }

    fn destination(&mut self, ui: &mut egui::Ui) {
        let p = Palette::of(ui);
        let mut mode = self.destination.mode;
        let label = |mode| match mode {
            DestinationMode::Downloads => "Downloads",
            DestinationMode::SameFolder => t("Folder źródłowy"),
            DestinationMode::Custom => t("Własny folder"),
            DestinationMode::OverwriteOriginals => t("Nadpisuj oryginały"),
        };
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(t("Zapisz w"))
                    .size(12.0)
                    .variation("wght", 600.0),
            )
            .on_hover_text(t(
                "Miejsce zapisu jest zapamiętywane. Zmiana dotyczy kolejnego startu.",
            ));
            egui::ComboBox::from_id_salt("save-destination")
                .selected_text(label(mode))
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for option in [
                        DestinationMode::Downloads,
                        DestinationMode::SameFolder,
                        DestinationMode::Custom,
                        DestinationMode::OverwriteOriginals,
                    ] {
                        if ui
                            .selectable_value(&mut mode, option, label(option))
                            .clicked()
                        {
                            ui.close();
                        }
                    }
                });
        });
        if mode != self.destination.mode {
            if mode == DestinationMode::Custom && self.destination.custom.is_none() {
                self.choose_output_folder(); // Cancelling leaves the previous choice intact.
            } else {
                self.destination.mode = mode;
                self.remember_destination();
            }
        }
        if self.destination.mode == DestinationMode::OverwriteOriginals {
            ui.add(egui::Label::new(RichText::new(t("Ten sam format: zastępuje oryginał. Inny format: nowy plik obok, oryginał zostaje."))
                .size(11.0).color(p.muted)).wrap());
        } else if self.destination.mode == DestinationMode::SameFolder {
            ui.label(
                RichText::new(t(
                    "Każdy wynik obok swojego oryginału. Oryginały nie są nadpisywane.",
                ))
                .size(11.0)
                .color(p.muted),
            );
        } else {
            let path = if self.destination.mode == DestinationMode::Custom {
                self.destination
                    .custom
                    .clone()
                    .ok_or_else(|| "Wybierz folder wynikowy.".into())
            } else {
                self.downloads.clone()
            };
            ui.horizontal(|ui| {
                if self.destination.mode == DestinationMode::Custom
                    && ui.small_button(t("Wybierz…")).clicked()
                {
                    self.choose_output_folder();
                }
                match path {
                    Ok(path) => {
                        if ui
                            .small_button("↗")
                            .on_hover_text(t("Otwórz folder wynikowy"))
                            .clicked()
                        {
                            self.open_folder(&path);
                        }
                        ui.add(
                            egui::Label::new(
                                RichText::new(path.display().to_string())
                                    .size(11.0)
                                    .color(p.muted),
                            )
                            .truncate(),
                        )
                        .on_hover_text(path.display().to_string());
                    }
                    Err(error) => {
                        ui.label(RichText::new(i18n::message(&error)).size(11.0).color(p.red));
                    }
                }
            });
        }
        let (before, after) = self.rows.iter().fold((0_u64, 0_u64), |(a, b), row| {
            if let Status::Done(result) = &row.status {
                (a + result.input_bytes, b + result.output_bytes)
            } else {
                (a, b)
            }
        });
        if before > after {
            ui.label(
                RichText::new(f(
                    "Oszczędzono {0}",
                    &[&optimizer::format_bytes(before - after)],
                ))
                .size(12.0)
                .color(p.green),
            );
        }
    }
}

impl eframe::App for OptimizerApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.import_paths(crate::open_documents::take(), ctx);
        self.collect_imports();
        while let Ok(event) = self.events.try_recv() {
            self.handle_event(event);
        }
        if self.close_when_idle && self.pending() == 0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn ui(&mut self, root: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.language.activate(&self.system_language);
        let ctx = root.ctx().clone();
        frame.set_traffic_lights_position(&ctx, egui::Rangef::new(0.0, 48.0), 18.0);
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma)) {
            self.advanced = !self.advanced;
        }
        if !egui::Popup::is_any_open(&ctx)
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.advanced = false;
            self.detail_row = None;
        }
        let paths = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect::<Vec<_>>()
        });
        self.import_paths(paths, &ctx);
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.pick_files(&ctx);
        }
        if ctx.input(|i| i.viewport().close_requested()) && self.pending() > 0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if !self.close_when_idle {
                self.confirm_close = true;
            }
        }
        if self.confirm_close {
            egui::Window::new(t("Trwa optymalizacja"))
                .id(egui::Id::unique("close-window"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(&ctx, |ui| {
                    ui.label(t("Anulować kolejkę i zamknąć aplikację?"));
                    ui.horizontal(|ui| {
                        if ui.button(t("Pracuj dalej")).clicked() {
                            self.confirm_close = false;
                        }
                        if ui.button(t("Anuluj i zamknij")).clicked() {
                            self.cancel_queue();
                            self.close_when_idle = true;
                            self.confirm_close = false;
                        }
                    });
                });
        }
        let p = Palette::of(root);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(p.background)
                    .inner_margin(egui::Margin {
                        left: 24,
                        right: 24,
                        top: 0,
                        bottom: 14,
                    }),
            )
            .show(root, |ui| {
                self.header(ui);
                ui.add_space(12.0);
                let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
                self.workspace_ui(ui, hovering);
            });
        self.compression_window(&ctx);
        self.details_window(&ctx);
    }
}

#[cfg(test)]
mod queue_tests {
    use super::*;

    fn app(output: PathBuf) -> (OptimizerApp, Receiver<Job>) {
        let (jobs, work) = mpsc::channel();
        let (_, events) = mpsc::channel();
        (
            OptimizerApp {
                settings: Settings::default(),
                rows: Vec::new(),
                scans: Vec::new(),
                jobs,
                events,
                cancel: Arc::new(AtomicBool::new(false)),
                downloads: Ok(output),
                destination: Destination::default(),
                next_id: 0,
                batch_total: 0,
                notice: None,
                confirm_close: false,
                close_when_idle: false,
                advanced: false,
                appearance: egui::ThemePreference::System,
                detail_row: None,
                settings_tab: 2,
                language: Language::English,
                system_language: "en".into(),
            },
            work,
        )
    }

    #[test]
    fn dropped_folders_stage_recursively_and_wait_for_start_with_current_settings() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("images");
        std::fs::create_dir_all(root.join("nested")).unwrap();
        let first = root.join("a.PNG");
        let second = root.join("nested/b.jpg");
        for file in [&first, &second, &root.join("notes.txt")] {
            std::fs::write(file, b"fixture").unwrap();
        }
        let (mut app, work) = app(dir.path().join("output"));
        app.enqueue(vec![first.clone()]);
        app.import_paths(
            vec![root.clone(), root.join("nested")],
            &egui::Context::default(),
        );
        app.start_conversion();
        assert!(work.try_recv().is_err()); // Wait for all folder scans, even with staged rows.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !app.scans.is_empty() {
            assert!(
                std::time::Instant::now() < deadline,
                "folder scan timed out"
            );
            app.collect_imports();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(app.ready(), 2);
        assert_eq!(app.rows.len(), 2);
        assert!(work.try_recv().is_err()); // Scanning never starts conversion.
        app.settings.max_width = Some(700);
        app.destination.mode = DestinationMode::SameFolder;
        app.start_conversion();
        for input in [first, second] {
            let job = work.try_recv().unwrap();
            assert_eq!(job.output, input.parent().unwrap());
            assert_eq!(job.input, input);
            assert_eq!(job.settings.max_width, Some(700));
        }
        assert!(work.try_recv().is_err());
    }

    #[test]
    fn cancelling_folder_import_does_not_add_late_results() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.png"), b"fixture").unwrap();
        let (mut app, work) = app(dir.path().join("output"));
        app.import_paths(vec![dir.path().to_owned()], &egui::Context::default());
        app.scans.clear();
        app.collect_imports();
        assert!(app.rows.is_empty());
        assert!(work.try_recv().is_err());
    }

    #[test]
    fn sidebar_fits_without_scrolling_at_minimum_window_height() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, _) = app(dir.path().join("downloads"));
        let ctx = egui::Context::default();
        appearance::install(&ctx);
        for language in [Language::Polish, Language::English] {
            language.activate("en");
            for format in [
                OutputFormat::Hybrid,
                OutputFormat::Webp,
                OutputFormat::Avif,
                OutputFormat::Jxl,
                OutputFormat::Png,
                OutputFormat::Jpeg,
                OutputFormat::Original,
            ] {
                for mode in [
                    DestinationMode::Downloads,
                    DestinationMode::SameFolder,
                    DestinationMode::Custom,
                    DestinationMode::OverwriteOriginals,
                ] {
                    for state in 0..3 {
                        let busy = state > 0;
                        let lossless = state == 2;
                        app.settings.format = format;
                        app.settings.trim = TrimMode::Background;
                        app.settings.webp_mode = if lossless {
                            WebpMode::Lossless
                        } else {
                            WebpMode::NearLossless
                        };
                        app.settings.avif_lossless = lossless;
                        app.settings.jxl_lossless = lossless;
                        app.settings.png_mode = if state == 0 {
                            PngMode::Auto
                        } else {
                            PngMode::Zopfli
                        };
                        app.settings.max_width = if state == 0 {
                            Some(1234)
                        } else if state == 1 {
                            Some(1500)
                        } else {
                            None
                        };
                        app.destination.mode = mode;
                        app.destination.custom =
                            Some(dir.path().join("Long custom output folder name 🖼"));
                        app.rows.clear();
                        if busy {
                            app.enqueue(vec!["one.png".into(), "two.png".into()]);
                            app.rows[0].status = Status::Working;
                        }
                        let mut geometry = (egui::Rect::NOTHING, egui::Rect::NOTHING);
                        for _ in 0..3 {
                            let mut rendered = ctx.run_ui(
                                egui::RawInput {
                                    screen_rect: Some(egui::Rect::from_min_size(
                                        egui::Pos2::ZERO,
                                        Vec2::new(380.0, 630.0),
                                    )),
                                    ..Default::default()
                                },
                                |ui| {
                                    geometry = app.export_sidebar(ui);
                                },
                            );
                            rendered.textures_delta.clear();
                        }
                        let (content, available) = geometry;
                        assert!(
                            content.bottom() <= available.bottom() + 1.0
                                && content.right() <= available.right() + 1.0,
                            "{language:?}, {format:?}, {mode:?}, busy={busy}: content={content:?}, available={available:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn overwrite_is_opt_in_and_captured_at_start() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, work) = app(dir.path().join("downloads"));
        assert!(!app.settings.overwrite_originals);
        app.enqueue(vec![dir.path().join("logo.png")]);
        app.destination.mode = DestinationMode::OverwriteOriginals;
        assert!(work.try_recv().is_err());
        app.start_conversion();
        let job = work.try_recv().unwrap();
        assert!(job.settings.overwrite_originals);
        assert_eq!(job.output, dir.path());
        app.destination.mode = DestinationMode::Downloads;
        assert!(job.settings.overwrite_originals);
        assert!(!app.settings.overwrite_originals);
    }

    #[test]
    fn destinations_are_captured_per_original_at_start_and_rerun_can_change_them() {
        let dir = tempfile::tempdir().unwrap();
        let inputs: Vec<_> = ["first", "second"]
            .iter()
            .map(|name| {
                let folder = dir.path().join(name);
                std::fs::create_dir(&folder).unwrap();
                let input = folder.join("logo.png");
                image::RgbaImage::from_pixel(40, 20, image::Rgba([80, 120, 200, 255]))
                    .save(&input)
                    .unwrap();
                input
            })
            .collect();
        let originals: Vec<_> = inputs.iter().map(|p| std::fs::read(p).unwrap()).collect();
        let (mut app, work) = app(dir.path().join("downloads"));
        app.enqueue(inputs.clone());
        app.settings.format = OutputFormat::Original;
        app.destination.mode = DestinationMode::SameFolder;
        app.start_conversion();
        let jobs: Vec<_> = work.try_iter().collect();
        assert_eq!(jobs.len(), 2);
        let custom = dir.path().join("Wyniki 🖼");
        app.destination = Destination {
            mode: DestinationMode::Custom,
            custom: Some(custom.clone()),
        };
        for (job, input) in jobs.into_iter().zip(&inputs) {
            assert_eq!(job.output, input.parent().unwrap());
            let result =
                optimizer::optimize(&job.input, &job.output, &job.settings, &job.cancel).unwrap();
            assert_eq!(result.path.file_name().unwrap(), "logo (1).png");
            app.handle_event(Event::Finished(job.id, Ok(result)));
        }
        assert!(!custom.exists());
        app.prepare_again(None);
        assert!(work.try_recv().is_err());
        app.settings.format = OutputFormat::Webp;
        app.start_conversion();
        let jobs: Vec<_> = work.try_iter().collect();
        assert_eq!(jobs.len(), 2);
        for job in jobs {
            assert_eq!(job.output, custom);
            let result =
                optimizer::optimize(&job.input, &job.output, &job.settings, &job.cancel).unwrap();
            app.handle_event(Event::Finished(job.id, Ok(result)));
        }
        assert!(custom.join("logo.webp").is_file());
        assert!(custom.join("logo (1).webp").is_file());
        for (input, bytes) in inputs.iter().zip(originals) {
            assert_eq!(std::fs::read(input).unwrap(), bytes);
        }
        app.prepare_again(None);
        app.destination.custom = None;
        app.start_conversion();
        assert_eq!(app.ready(), 2);
        assert!(work.try_recv().is_err());
    }

    #[test]
    fn rerun_uses_original_and_latest_settings_without_overwriting_previous_result() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("logo.png");
        image::RgbaImage::from_pixel(80, 40, image::Rgba([80, 120, 200, 255]))
            .save(&input)
            .unwrap();
        let original = std::fs::read(&input).unwrap();
        let (mut app, work) = app(dir.path().join("output"));
        app.enqueue(vec![input.clone()]);
        app.settings.max_width = Some(20);
        app.start_conversion();
        let first = work.try_recv().unwrap();
        let outcome =
            optimizer::optimize(&first.input, &first.output, &first.settings, &first.cancel)
                .unwrap();
        let previous_path = outcome.path.clone();
        let previous_bytes = std::fs::read(&previous_path).unwrap();
        app.handle_event(Event::Finished(first.id, Ok(outcome)));
        app.prepare_again(Some(first.id));
        assert_eq!(app.ready(), 1);
        assert!(work.try_recv().is_err());
        assert_eq!(
            std::fs::read_dir(dir.path().join("output"))
                .unwrap()
                .count(),
            1
        );
        app.settings.max_width = Some(60);
        app.settings.webp_mode = WebpMode::Lossless;
        app.start_conversion();
        let second = work.try_recv().unwrap();
        assert_eq!(second.input, input);
        assert_eq!(second.settings.webp_mode, WebpMode::Lossless);
        let outcome = optimizer::optimize(
            &second.input,
            &second.output,
            &second.settings,
            &second.cancel,
        )
        .unwrap();
        assert_eq!((outcome.width, outcome.height), (60, 30));
        assert_ne!(outcome.path, previous_path);
        assert_eq!(std::fs::read(previous_path).unwrap(), previous_bytes);
        assert_eq!(std::fs::read(input).unwrap(), original);
        app.handle_event(Event::Finished(second.id, Ok(outcome)));
        assert_eq!(app.rows.len(), 1);
        assert!(work.try_recv().is_err());
    }

    #[test]
    fn rerun_selection_batch_deduplication_and_active_job_guards() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, work) = app(dir.path().join("output"));
        app.enqueue(vec![
            "one.png".into(),
            "two.jpg".into(),
            "three.webp".into(),
            "notes.txt".into(),
        ]);
        app.rows[0].status = Status::Failed("test".into());
        app.rows[1].status = Status::Cancelled;
        app.prepare_again(Some(0));
        assert_eq!(app.ready(), 2); // Selected retry plus the already staged third file.
        assert!(matches!(app.rows[1].status, Status::Cancelled));
        assert!(work.try_recv().is_err());
        app.prepare_again(Some(0)); // Repeated clicks do not create duplicates.
        app.prepare_again(None);
        assert_eq!(app.ready(), 3);
        assert!(matches!(app.rows[3].status, Status::Failed(_))); // Unsupported files stay failed.
        app.start_conversion();
        let jobs: Vec<_> = work.try_iter().collect();
        assert_eq!(jobs.len(), 3);
        app.handle_event(Event::Finished(jobs[0].id, Err("test".into())));
        app.prepare_again(None); // Other jobs still run.
        assert_eq!(app.ready(), 0);
        for job in &jobs[1..] {
            app.handle_event(Event::Finished(job.id, Err("Anulowano".into())));
        }
        app.enqueue(vec!["one.png".into()]); // Same original has a new staged entry.
        app.prepare_again(None);
        assert_eq!(app.ready(), 3); // Only one job per original, including the duplicate.
        assert!(work.try_recv().is_err());
        app.start_conversion();
        let jobs: Vec<_> = work.try_iter().collect();
        assert_eq!(jobs.len(), 3);
        assert_eq!(
            jobs.iter()
                .filter(|job| job.input == std::path::Path::new("one.png"))
                .count(),
            1
        );
        for job in jobs {
            app.handle_event(Event::Finished(job.id, Err("Anulowano".into())));
        }
        app.close_when_idle = true;
        app.prepare_again(None);
        assert_eq!(app.ready(), 0);
    }

    #[test]
    fn adding_files_does_not_submit_work_and_start_uses_latest_settings() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("logo.png");
        image::RgbaImage::from_pixel(40, 20, image::Rgba([80, 120, 200, 128]))
            .save(&input)
            .unwrap();
        let output = dir.path().join("output");
        let (mut app, work) = app(output.clone());
        app.enqueue(vec![input.clone(), input]);
        assert_eq!(app.ready(), 1);
        assert_eq!(app.pending(), 0);
        assert!(matches!(work.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert!(!output.exists());
        app.settings.format = OutputFormat::Avif;
        app.settings.avif_lossless = true;
        app.settings.max_width = Some(20);
        app.settings.trim = TrimMode::Transparent;
        app.start_conversion();
        let job = work.try_recv().unwrap();
        assert_eq!(job.settings.format, OutputFormat::Avif);
        assert_eq!(job.settings.max_width, Some(20));
        assert!(job.settings.avif_lossless);
        assert_eq!(job.settings.trim, TrimMode::Transparent);
        app.settings.format = OutputFormat::Jxl;
        app.start_conversion(); // Repeated start must never submit a second copy.
        assert!(work.try_recv().is_err());
        let outcome =
            optimizer::optimize(&job.input, &job.output, &job.settings, &job.cancel).unwrap();
        assert_eq!(outcome.path.extension().unwrap(), "avif");
        assert_eq!((outcome.width, outcome.height), (20, 10));
        app.handle_event(Event::Finished(job.id, Ok(outcome)));
        assert_eq!(app.pending(), 0);
        assert_eq!(app.ready(), 0);
        app.start_conversion();
        assert!(work.try_recv().is_err());
        assert_eq!(std::fs::read_dir(output).unwrap().count(), 1);
    }

    #[test]
    fn files_added_during_conversion_wait_for_another_explicit_start() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, work) = app(dir.path().join("output"));
        app.enqueue(vec!["first.png".into()]);
        app.start_conversion();
        let first = work.try_recv().unwrap();
        app.handle_event(Event::Started(first.id));
        app.enqueue(vec!["second.png".into(), "first.png".into()]);
        app.settings.format = OutputFormat::Jxl;
        app.start_conversion();
        assert!(work.try_recv().is_err());
        assert_eq!(app.rows.len(), 2);
        assert_eq!(app.ready(), 1);
        assert_eq!(app.batch_total, 1);
        assert_eq!(first.settings.format, OutputFormat::Webp);
        app.handle_event(Event::Finished(first.id, Err("test failure".into())));
        assert!(work.try_recv().is_err()); // Finishing is not an implicit start.
        app.start_conversion();
        let second = work.try_recv().unwrap();
        assert_eq!(second.input, PathBuf::from("second.png"));
        assert_eq!(second.settings.format, OutputFormat::Jxl);
        assert!(work.try_recv().is_err());
    }

    #[test]
    fn cancellation_does_not_submit_or_cancel_unconfirmed_files() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, work) = app(dir.path().join("output"));
        app.enqueue(vec!["first.png".into(), "queued.png".into()]);
        app.start_conversion();
        let first = work.try_recv().unwrap();
        let queued = work.try_recv().unwrap();
        app.enqueue(vec!["next.png".into()]);
        app.cancel_queue();
        assert!(first.cancel.load(Ordering::Relaxed));
        assert!(queued.cancel.load(Ordering::Relaxed));
        assert_eq!(app.ready(), 1);
        app.start_conversion();
        assert!(work.try_recv().is_err());
        app.handle_event(Event::Finished(first.id, Err("Anulowano".into())));
        app.handle_event(Event::Finished(queued.id, Err("Anulowano".into())));
        assert!(work.try_recv().is_err());
        app.start_conversion();
        let next = work.try_recv().unwrap();
        assert!(!next.cancel.load(Ordering::Relaxed));
        assert_eq!(next.input, PathBuf::from("next.png"));
    }

    #[test]
    fn invalid_inputs_clear_and_submission_errors_never_start_work() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, work) = app(dir.path().join("output"));
        app.enqueue(vec!["notes.txt".into(), "image.png".into()]);
        assert_eq!(app.ready(), 1);
        assert!(matches!(app.rows[0].status, Status::Failed(_)));
        app.rows.clear();
        app.start_conversion();
        assert!(work.try_recv().is_err());
        app.enqueue(vec!["image.png".into()]);
        app.downloads = Err("unavailable output".into());
        app.start_conversion();
        assert!(
            matches!(&app.rows[0].status, Status::Failed(message) if message == "unavailable output")
        );
        assert!(work.try_recv().is_err());
        app.downloads = Ok(dir.path().join("output"));
        app.enqueue(vec!["another.png".into()]);
        drop(work);
        app.start_conversion();
        assert!(matches!(app.rows[1].status, Status::Failed(_)));
        assert_eq!(app.pending(), 0);
    }
}
