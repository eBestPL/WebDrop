//! Display localization for the GUI and English CLI diagnostics.
//! Workers retain the original diagnostics; they are translated only
//! when displayed, so changing language also updates already completed queue items.
use std::cell::Cell;
mod catalog;
use catalog::MESSAGES;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    System,
    Polish,
    English,
}

impl Language {
    fn from_stored(value: i32) -> Self {
        match value {
            1 => Self::Polish,
            2 => Self::English,
            _ => Self::System,
        }
    }

    pub fn load() -> Self {
        #[cfg(target_os = "macos")]
        // SAFETY: the bridge takes no pointers and returns a bounded preference code.
        unsafe {
            Self::from_stored(io_language_preference())
        }
        #[cfg(not(target_os = "macos"))]
        Self::from_stored(crate::preferences::load("language").unwrap_or(0))
    }

    pub fn save(self) -> bool {
        #[cfg(target_os = "macos")]
        // SAFETY: the bridge accepts these three codes and owns all Foundation values.
        unsafe {
            io_save_language_preference(match self {
                Self::System => 0,
                Self::Polish => 1,
                Self::English => 2,
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            crate::preferences::save(
                "language",
                &match self {
                    Self::System => 0,
                    Self::Polish => 1,
                    Self::English => 2,
                },
            )
            .is_ok()
        }
    }

    fn resolve(self, system: &str) -> Self {
        match self {
            Self::System
                if system
                    .split(['-', '_'])
                    .next()
                    .is_some_and(|v| v.eq_ignore_ascii_case("pl")) =>
            {
                Self::Polish
            }
            Self::System => Self::English,
            explicit => explicit,
        }
    }

    pub fn activate(self, system: &str) {
        CURRENT.set(self.resolve(system));
    }
}

// egui renders on one thread. No process-wide locale or worker state is mutated.
thread_local! { static CURRENT: Cell<Language> = const { Cell::new(Language::Polish) }; }

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn io_language_preference() -> i32;
    fn io_save_language_preference(language: i32) -> bool;
    fn io_system_language(buffer: *mut std::ffi::c_char, capacity: usize);
}

pub fn system_language() -> String {
    #[cfg(target_os = "macos")]
    {
        let mut buffer = [0 as std::ffi::c_char; 96];
        // SAFETY: the bridge receives a valid writable buffer, always NUL-terminates it.
        unsafe {
            io_system_language(buffer.as_mut_ptr(), buffer.len());
            std::ffi::CStr::from_ptr(buffer.as_ptr())
                .to_string_lossy()
                .into_owned()
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        crate::platform::system_language()
    }
}

pub fn t(source: &str) -> &str {
    if CURRENT.get() == Language::English {
        MESSAGES
            .iter()
            .find(|(pl, _)| *pl == source)
            .map_or(source, |(_, en)| *en)
    } else {
        source
    }
}

/// Numbered placeholders are substituted in one pass: values are never reinterpreted.
pub fn f(source: &str, values: &[&dyn std::fmt::Display]) -> String {
    render(
        t(source),
        &values.iter().map(ToString::to_string).collect::<Vec<_>>(),
    )
}

fn render(template: &str, values: &[String]) -> String {
    let mut output = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        output.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('}').map(|end| start + end) else {
            break;
        };
        if let Ok(index) = rest[start + 1..end].parse::<usize>() {
            if let Some(value) = values.get(index) {
                output.push_str(value);
            }
        } else {
            output.push_str(&rest[start..=end]);
        }
        rest = &rest[end + 1..];
    }
    output.push_str(rest);
    output
}

/// Match only known, complete application diagnostics, never filenames or paths.
/// Values captured from a diagnostic template are copied verbatim.
pub fn message(source: &str) -> String {
    if CURRENT.get() != Language::English {
        return source.to_owned();
    }
    english_message(source)
}

/// Translate application diagnostics without reading or changing GUI preferences.
pub fn english_message(source: &str) -> String {
    if let Some((_, en)) = MESSAGES.iter().find(|(pl, _)| *pl == source) {
        return (*en).into();
    }
    // anyhow context chains: translate a known context, then the remaining diagnostic.
    for &(pl, en) in MESSAGES.iter().filter(|(pl, _)| !pl.contains('{')) {
        if let Some(rest) = source.strip_prefix(pl).and_then(|s| s.strip_prefix(": ")) {
            return format!("{en}: {}", english_message(rest));
        }
    }
    for &(pl, en) in MESSAGES.iter().filter(|(pl, _)| pl.contains("{0}")) {
        if let Some(values) = capture(pl, source) {
            return render(en, &values);
        }
    }
    source.to_owned()
}

/// Preserve each error cause and translate only its application-owned message.
pub fn english_error(error: &anyhow::Error) -> String {
    error
        .chain()
        .map(|cause| english_message(&cause.to_string()))
        .collect::<Vec<_>>()
        .join(": ")
}

fn capture(template: &str, source: &str) -> Option<Vec<String>> {
    let mut pattern = template;
    let mut rest = source;
    let mut values = Vec::new();
    while let Some(start) = pattern.find('{') {
        rest = rest.strip_prefix(&pattern[..start])?;
        let end = pattern[start..].find('}')? + start;
        if pattern[start + 1..end].parse::<usize>().ok()? != values.len() {
            return None;
        }
        pattern = &pattern[end + 1..];
        let next_literal = pattern.split('{').next()?;
        let size = if next_literal.is_empty() {
            rest.len()
        } else {
            rest.find(next_literal)?
        };
        values.push(rest[..size].to_owned());
        rest = &rest[size..];
    }
    (rest == pattern).then_some(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_and_system_languages() {
        assert_eq!(Language::default(), Language::System);
        assert_eq!(Language::from_stored(99), Language::System);
        for tag in ["pl", "pl-PL", "PL_pl"] {
            assert_eq!(Language::System.resolve(tag), Language::Polish);
        }
        for tag in ["en-US", "de-DE", "es", "", "plausible"] {
            assert_eq!(Language::System.resolve(tag), Language::English);
        }
        assert_eq!(Language::Polish.resolve("en"), Language::Polish);
        assert_eq!(Language::English.resolve("pl"), Language::English);
    }
    #[test]
    fn catalog_has_unique_keys_and_matching_placeholders() {
        let mut keys = std::collections::HashSet::new();
        for &(pl, en) in MESSAGES {
            assert!(keys.insert(pl), "Duplicate translation: {pl}");
            assert!(!en.is_empty());
            for index in 0..10 {
                let token = format!("{{{index}}}");
                assert_eq!(
                    pl.matches(&token).count(),
                    en.matches(&token).count(),
                    "{pl}"
                );
            }
        }
    }
    #[test]
    fn switching_language_and_diagnostics_preserve_values() {
        Language::English.activate("pl");
        assert_eq!(t("Ustawienia"), "Settings");
        assert_eq!(
            message("Nie można odczytać obrazu: unexpected end"),
            "Cannot read the image: unexpected end"
        );
        assert_eq!(
            message("Strona 2 z 7 (wybrana w ustawieniach)"),
            "Page 2 of 7 (selected in settings)"
        );
        assert_eq!(
            message("Trim: 200 × 300 do 100 × 200 px (przed skalowaniem)"),
            "Trim: 200 × 300 to 100 × 200 px (before resizing)"
        );
        assert_eq!(
            f("Szczegóły: {0}", &[&"Ustawienia {1}.png"]),
            "Details: Ustawienia {1}.png"
        );
        assert_eq!(
            message("/Downloads/Ustawienia.png"),
            "/Downloads/Ustawienia.png"
        );
        Language::Polish.activate("en");
        assert_eq!(t("Ustawienia"), "Ustawienia");
        assert_eq!(
            english_message("Nie można odczytać obrazu"),
            "Cannot read the image"
        );
        assert_eq!(
            t("Ustawienia"),
            "Ustawienia",
            "CLI must not change the GUI language"
        );
    }
}
