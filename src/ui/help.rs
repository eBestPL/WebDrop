//! Plain-language descriptions shared by the main controls and settings.
use super::i18n::t;
use image_optimizer::optimizer::{OutputFormat, PngMode, Settings, TrimMode, WebpMode};

pub fn trim_label(mode: TrimMode) -> &'static str {
    match mode {
        TrimMode::Off => t("Nie przycinaj"),
        TrimMode::Transparent => t("Przytnij przezroczyste brzegi"),
        TrimMode::Background => t("Przytnij jednolite tło i przezroczystość"),
    }
}

pub fn trim_description(mode: TrimMode) -> &'static str {
    match mode {
        TrimMode::Off => {
            t("Zostawia wszystkie brzegi obrazu, także białe i przezroczyste marginesy.")
        }
        TrimMode::Transparent => {
            t("Odcina puste, przezroczyste brzegi, np. wokół logo PNG. Białe tło zostaje.")
        }
        TrimMode::Background => t(
            "Odcina brzegi w kolorze narożników oraz przezroczyste. Nie wycina tła ze środka logo.",
        ),
    }
}

pub fn format_description(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Hybrid => t(
            "AI, EPS, PDF, SVG → SVG. Pozostałe → WebP. Rozmiar i jakość dotyczą tylko WebP; SVG: tylko trim.",
        ),
        OutputFormat::Webp => t("WebP: do internetu, także z przezroczystym tłem."),
        OutputFormat::Avif => t(
            "AVIF: małe zdjęcia do internetu, także z przezroczystością. Zapis może potrwać dłużej.",
        ),
        OutputFormat::Jxl => {
            t("JPEG XL: zdjęcia i grafiki wysokiej jakości. Sprawdź obsługę na docelowej stronie.")
        }
        OutputFormat::Png => {
            t("PNG: grafiki i logo z przezroczystością. Tryb kompresji wybierz poniżej.")
        }
        OutputFormat::Jpeg => t("JPG: do zdjęć. Przezroczyste miejsca zostaną wypełnione bielą."),
        OutputFormat::Original => {
            t("Zachowuje PNG/JPG/WebP/AVIF/JXL. AI/PDF/EPS/TIFF/HEIC → WebP. SVG zawsze → SVG.")
        }
    }
}

pub fn webp_description(mode: WebpMode) -> &'static str {
    match mode {
        WebpMode::Lossy => {
            t("Zmniejsza plik kosztem części szczegółów. Wyższa jakość zwykle daje większy plik.")
        }
        WebpMode::Lossless => t(
            "Zachowuje piksele po przycięciu i skalowaniu. Dobre do logo; jakość 1–100 nie ma tu wpływu.",
        ),
        WebpMode::NearLossless => t(
            "Lekko upraszcza piksele, aby zmniejszyć plik. Wyższa jakość zachowuje więcej szczegółów.",
        ),
    }
}

pub fn png_description(mode: PngMode) -> &'static str {
    match mode {
        PngMode::Lossless => t("OxiPNG: zmniejsza plik bez zmiany pikseli gotowego obrazu."),
        PngMode::Palette => t(
            "Paleta: ogranicza liczbę kolorów. Może dać mniejszy plik, ale zmienia wygląd obrazu.",
        ),
        PngMode::Zopfli => t(
            "Zopfli: szuka lepszej kompresji bez zmiany pikseli. Działa wolniej; wynik nie zawsze jest mniejszy.",
        ),
    }
}

pub fn compression_description(settings: &Settings) -> &'static str {
    match settings.format {
        OutputFormat::Webp | OutputFormat::Hybrid => webp_description(settings.webp_mode),
        OutputFormat::Avif => modern_description(settings.avif_lossless),
        OutputFormat::Jxl => modern_description(settings.jxl_lossless),
        OutputFormat::Png => png_description(settings.png_mode),
        OutputFormat::Jpeg => t(
            "Niższa jakość daje zwykle mniejszy plik i mniej szczegółów. 82 to punkt wyjścia do zdjęć.",
        ),
        OutputFormat::Original => t(
            "Każdy format używa własnych ustawień kompresji. „Oryginał” nie oznacza kopii bez zmian.",
        ),
    }
}

pub fn modern_description(lossless: bool) -> &'static str {
    if lossless {
        t("Zachowuje piksele gotowego obrazu w 8-bitowym sRGB. Jakość 1–100 nie ma wpływu.")
    } else {
        t(
            "Wyższa jakość zachowuje więcej szczegółów. Ta sama liczba daje różne wyniki w AVIF, JXL i WebP.",
        )
    }
}
