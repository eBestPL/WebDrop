// Polish source text → English. Numbered placeholders must match on both sides.
pub const MESSAGES: &[(&str, &str)] = &[
    ("Hybrydowy · SVG / WebP", "Hybrid · SVG / WebP"),
    (
        "AI, EPS, PDF, SVG → SVG. Pozostałe → WebP. Rozmiar i jakość dotyczą tylko WebP; SVG: tylko trim.",
        "AI, EPS, PDF, SVG → SVG. Others → WebP. Size and quality apply only to WebP; SVG: trim only.",
    ),
    (
        "Tryb hybrydowy: eksport do SVG. Tekst zamieniony na krzywe; szerokość, jakość i DPI nie dotyczą SVG.",
        "Hybrid mode: SVG export. Text converted to paths; width, quality and DPI do not apply to SVG.",
    ),
    (
        "Konwersja do SVG wymaga MuPDF (mutool). Użyj pełnej paczki macOS lub zainstaluj MuPDF i dodaj mutool do PATH.",
        "SVG conversion requires MuPDF (mutool). Use the full macOS package or install MuPDF and add mutool to PATH.",
    ),
    (
        "Nie można uruchomić silnika konwersji SVG",
        "Cannot start the SVG conversion engine",
    ),
    (
        "Przekroczono limit czasu konwersji SVG (60 s)",
        "SVG conversion timed out (60 s)",
    ),
    (
        "Przekroczono limit danych konwersji SVG",
        "SVG conversion data limit exceeded",
    ),
    (
        "Nie można odczytać liczby stron PDF",
        "Cannot read the PDF page count",
    ),
    (
        "Wybrana strona nie istnieje w PDF",
        "The selected page does not exist in the PDF",
    ),
    (
        "Silnik konwersji nie utworzył SVG",
        "The conversion engine did not produce an SVG",
    ),
    ("Interaktywne SVG", "Interactive SVG"),
    (
        "Wybierz PNG, JPG, WebP, AVIF, JXL, AI/PDF, EPS lub SVG",
        "Choose PNG, JPG, WebP, AVIF, JXL, AI/PDF, EPS or SVG",
    ),
    (
        "Obsługiwane formaty: PNG, JPG, WebP, AVIF, JXL, AI/PDF, EPS, SVG (tylko trim)",
        "Supported formats: PNG, JPG, WebP, AVIF, JXL, AI/PDF, EPS, SVG (trim only)",
    ),
    (
        "AI/PDF na Windows/Linux: tylko tryb hybrydowy, wymagany MuPDF. TIFF i HEIC: tylko macOS.",
        "AI/PDF on Windows/Linux: Hybrid mode only, requires MuPDF. TIFF and HEIC: macOS only.",
    ),
    (
        "Nowe pliki uruchomisz po zakończeniu paczki.",
        "Start new files after this batch finishes.",
    ),
    (
        "Szczegóły zmienisz w Ustawieniach w prawym górnym rogu.",
        "Change the details in Settings in the top-right corner.",
    ),
    ("Zapisz w", "Save to"),
    ("Folder źródłowy", "Same path"),
    ("Własny folder", "Custom path"),
    ("Wybierz…", "Choose…"),
    ("Wybierz folder wynikowy", "Choose output folder"),
    ("Wybierz folder wynikowy.", "Choose an output folder."),
    ("Otwórz folder wynikowy", "Open output folder"),
    (
        "Nie można znaleźć folderu źródłowego.",
        "Cannot find the source folder.",
    ),
    ("Nie można otworzyć folderu: {0}", "Cannot open folder: {0}"),
    (
        "Nie udało się zapamiętać miejsca zapisu. Bieżący wybór nadal działa.",
        "Could not remember the output destination. Your current choice still works.",
    ),
    (
        "Każdy wynik obok swojego oryginału. Oryginały nie są nadpisywane.",
        "Each result is saved beside its original. Originals are never overwritten.",
    ),
    (
        "Miejsce zapisu jest zapamiętywane. Zmiana dotyczy kolejnego startu.",
        "Destination is remembered. Changes apply to the next start.",
    ),
    ("Opcje eksportu", "Export options"),
    (
        "Ustaw parametry przed rozpoczęciem konwersji",
        "Choose settings before starting conversion",
    ),
    ("Rozpocznij konwersję", "Start conversion"),
    (
        "Dodaj pliki do listy po lewej",
        "Add files to the list on the left",
    ),
    (
        "Wybierz „Ponów”, aby użyć nowych ustawień",
        "Choose “Re-run” to use new settings",
    ),
    ("Więcej ustawień  ›", "More settings  ›"),
    ("Pliki", "Files"),
    (
        "Dodaj obrazy lub przeciągnij je do okna",
        "Add images or drag them into the window",
    ),
    ("WebP · 1500 px", "WebP · 1500 px"),
    ("Bezstratnie · trim", "Lossless · trim"),
    ("Ponów", "Re-run"),
    ("Ponów: {0}", "Re-run: {0}"),
    ("Ponów wszystkie", "Re-run all"),
    ("Zmień opcje i ponów", "Change settings and re-run"),
    (
        "Przygotuj oryginały ponownie, zmień opcje i potwierdź przyciskiem startu. Poprzednie wyniki pozostaną na dysku.",
        "Stage the originals again, change settings, then confirm with the Start button. Previous results stay on disk.",
    ),
    (
        "Pliki przygotowane ponownie. Zmień opcje i kliknij „Rozpocznij konwersję”. Poprzednie wyniki pozostają na dysku.",
        "Files staged again. Change settings, then click “Start conversion”. Previous results stay on disk.",
    ),
    ("Rozpocznij konwersję · {0}", "Start conversion · {0}"),
    ("Konwersja w toku…", "Conversion in progress…"),
    ("Czeka na start · {0}", "Awaiting start · {0}"),
    (
        "Uruchamia oczekujące pliki z aktualnymi ustawieniami. Dodanie plików nie rozpoczyna konwersji.",
        "Starts waiting files with the current settings. Adding files does not start conversion.",
    ),
    (
        "Nowe pliki czekają na osobny start po zakończeniu tej konwersji.",
        "New files await a separate start after this conversion finishes.",
    ),
    (
        "Ustaw opcje, a następnie kliknij „Rozpocznij konwersję”. Plik nie został jeszcze przetworzony.",
        "Choose settings, then click “Start conversion”. This file has not been processed yet.",
    ),
    ("Ustawienia przed startem", "Settings before starting"),
    ("Zwiń ustawienia", "Collapse settings"),
    ("Zmień ustawienia", "Change settings"),
    ("Do {0} px", "Up to {0} px"),
    ("Pełna szerokość", "Full width"),
    ("Ustawienia według formatu", "Settings per format"),
    (
        "Prawie bezstratna · jakość {0}",
        "Near-lossless · quality {0}",
    ),
    ("Jakość {0}", "Quality {0}"),
    (
        "AVIF: małe zdjęcia do internetu, także z przezroczystością. Zapis może potrwać dłużej.",
        "AVIF: small web photos, including transparency. Encoding may take longer.",
    ),
    (
        "JPEG XL: zdjęcia i grafiki wysokiej jakości. Sprawdź obsługę na docelowej stronie.",
        "JPEG XL: high-quality photos and graphics. Check support on your target website.",
    ),
    (
        "AVIF/JXL: jakość dotyczy trybu stratnego. Bezstratny zachowuje piksele gotowego obrazu w 8-bitowym sRGB.",
        "AVIF/JXL: quality applies to lossy mode. Lossless preserves the finished image’s pixels in 8-bit sRGB.",
    ),
    (
        "Zachowuje piksele gotowego obrazu w 8-bitowym sRGB. Jakość 1–100 nie ma wpływu.",
        "Preserves the finished image’s pixels in 8-bit sRGB. Quality 1–100 has no effect.",
    ),
    (
        "Wyższa jakość zachowuje więcej szczegółów. Ta sama liczba daje różne wyniki w AVIF, JXL i WebP.",
        "Higher quality preserves more detail. The same number gives different results in AVIF, JXL and WebP.",
    ),
    (
        "AVIF/JXL: przekroczono limit rozmiaru lub pamięci",
        "AVIF/JXL: size or memory limit exceeded",
    ),
    ("AVIF/JXL: za mało pamięci", "AVIF/JXL: not enough memory"),
    (
        "Animowane AVIF/JXL nie są obsługiwane; animacja nie została zmieniona",
        "Animated AVIF/JXL is not supported; the animation was not changed",
    ),
    (
        "AVIF/JXL: nieobsługiwany profil kolorów lub HDR. Wyeksportuj obraz do SDR/sRGB.",
        "AVIF/JXL: unsupported color profile or HDR. Export the image to SDR/sRGB.",
    ),
    (
        "Nie udało się zakodować AVIF/JXL",
        "Could not encode AVIF/JXL",
    ),
    (
        "Nie można odczytać AVIF/JXL: uszkodzony lub nieobsługiwany obraz",
        "Cannot read AVIF/JXL: corrupt or unsupported image",
    ),
    ("Pusty wynik AVIF/JXL", "Empty AVIF/JXL output"),
    ("Nieprawidłowy wynik AVIF/JXL", "Invalid AVIF/JXL output"),
    (
        "AVIF/JXL: eksport jako 8-bitowy obraz SDR w sRGB",
        "AVIF/JXL: exported as an 8-bit SDR image in sRGB",
    ),
    ("Zamknij", "Close"),
    ("Nie można otworzyć linku: {0}", "Cannot open the link: {0}"),
    ("Ustawienia", "Settings"),
    ("Kompresja", "Compression"),
    ("Marginesy", "Margins"),
    ("Stratna", "Lossy"),
    ("Bezstratna", "Lossless"),
    ("Strona", "Page"),
    ("Tolerancja", "Tolerance"),
    ("Systemowy", "System"),
    ("Jasny", "Light"),
    ("Ciemny", "Dark"),
    ("Gotowe", "Done"),
    ("Anuluj", "Cancel"),
    ("Anulowano", "Cancelled"),
    ("Przetwarzanie…", "Processing…"),
    ("Przetwarzanie · {0}", "Processing · {0}"),
    ("Gotowe: {0} · Błędy: {1}", "Done: {0} · Errors: {1}"),
    ("Gotowe · {0}/{1}", "Done · {0}/{1}"),
    (
        "Koder nie mógł przetworzyć tego obrazu",
        "The encoder could not process this image",
    ),
    (
        "Wybierz PNG, JPG, WebP, AVIF, JXL, AI/PDF, EPS, TIFF, HEIC lub SVG",
        "Choose PNG, JPG, WebP, AVIF, JXL, AI/PDF, EPS, TIFF, HEIC or SVG",
    ),
    (
        "Wątek konwersji jest niedostępny. Uruchom aplikację ponownie.",
        "The conversion worker is unavailable. Restart the app.",
    ),
    ("Dodaj obrazy", "Add images"),
    ("Obrazy i dokumenty", "Images and documents"),
    (
        "Nie można otworzyć Downloads: {0}",
        "Cannot open Downloads: {0}",
    ),
    (
        "Anulowanie kolejki. Trwający etap kodowania musi się zakończyć.",
        "Cancelling the queue. The current encoding step needs to finish.",
    ),
    (
        "Kompresja, import, język i informacje o aplikacji · ⌘,",
        "Compression, import, language and app information · ⌘,",
    ),
    ("Obrazy gotowe na web", "Images ready for the web"),
    (
        "Dodaj obrazy, ustaw opcje i kliknij „Rozpocznij konwersję”.",
        "Add images, choose settings and click “Start conversion”.",
    ),
    (
        "Dodaj obrazy, a potem potwierdź konwersję przyciskiem.",
        "Add images, then confirm conversion with the button.",
    ),
    ("Upuść kolejne obrazy", "Drop more images"),
    ("Dodaj kolejne obrazy", "Add more images"),
    ("+  Dodaj pliki", "+  Add files"),
    (
        "Wybierz jeden lub wiele plików · ⌘O",
        "Choose one or more files · ⌘O",
    ),
    ("Upuść obrazy lub foldery", "Drop images or folders"),
    (
        "Przeciągnij obrazy lub foldery",
        "Drag images or folders here",
    ),
    ("Obrazy lub foldery", "Images or folders"),
    ("Wyszukiwanie obrazów…", "Finding images…"),
    (
        "Nie można rozpocząć wyszukiwania obrazów: {0}",
        "Cannot start finding images: {0}",
    ),
    (
        "Nie można odczytać części plików lub folderów ({0}). Pierwszy błąd: {1}",
        "Some files or folders could not be read ({0}). First error: {1}",
    ),
    (
        "Nie znaleziono obsługiwanych obrazów w dodanych folderach.",
        "No supported images were found in the added folders.",
    ),
    (
        "Wyszukiwanie obrazów zostało przerwane. Dodaj folder ponownie.",
        "Finding images was interrupted. Add the folder again.",
    ),
    (
        "Foldery obejmują podfoldery. Konwersję uruchomisz przyciskiem.",
        "Includes subfolders. Start conversion when you are ready.",
    ),
    ("+   Wybierz pliki", "+   Choose files"),
    (
        "Jeden obraz lub wiele plików naraz · ⌘O",
        "One image or multiple files at once · ⌘O",
    ),
    ("Na web", "For the web"),
    ("WebP · 1500 px · jakość 82", "WebP · 1500 px · quality 82"),
    (
        "Dla zdjęć i obrazów na stronę: WebP stratny, jakość 82, do 1500 px szerokości. Zostawia marginesy.",
        "For photos and web images: lossy WebP, quality 82, up to 1500 px wide. Keeps margins.",
    ),
    ("Bezstratnie · bez marginesów", "Lossless · trimmed margins"),
    (
        "Dla logo: WebP bezstratny, do 1500 px. Przycina jednolite i przezroczyste brzegi, potem skaluje. SVG pozostaje SVG.",
        "For logos: lossless WebP, up to 1500 px. Trims solid and transparent edges, then resizes. SVG stays SVG.",
    ),
    ("Oryginał", "Original"),
    ("Oryginalny", "Original"),
    ("Szerokość", "Width"),
    ("Pełna", "Full"),
    (
        "Przycinanie działa przed skalowaniem. SVG: oba tryby przycinają tylko obszar dokumentu.",
        "Trimming happens before resizing. SVG: both modes only trim the document canvas.",
    ),
    ("Prawie bezstratna", "Near-lossless"),
    ("Stratna · zdjęcia", "Lossy · photos"),
    (
        "Bezstratna · logo i grafiki",
        "Lossless · logos and graphics",
    ),
    ("Bez utraty pikseli", "Preserves pixels"),
    (
        "Dotyczy kodowania po renderowaniu, trimie i skalowaniu. Suwak jakości nie ma wpływu na ten tryb.",
        "Applies to encoding after rendering, trimming and resizing. The quality slider does not affect this mode.",
    ),
    ("Jakość", "Quality"),
    (
        "Jakość 1–100. Kliknij, aby wpisać, lub przeciągnij wartość.",
        "Quality 1–100. Click to type or drag to adjust.",
    ),
    (
        "Ustawienia dla każdego formatu  ›",
        "Settings for each format  ›",
    ),
    (
        "Do {0} px · wysokość proporcjonalnie",
        "Up to {0} px · proportional height",
    ),
    (
        "Pełna szerokość · bez skalowania",
        "Full width · no resizing",
    ),
    (
        "1500 px to maksymalna szerokość, nie obowiązkowy rozmiar. Obraz 3000 × 2000 px stanie się 1500 × 1000 px. Obraz 800 × 600 px pozostanie bez zmian. Po przycięciu marginesów rozmiar liczony jest od przyciętego obrazu. SVG nie jest skalowany.",
        "1500 px is the maximum width, not a fixed size. A 3000 × 2000 px image becomes 1500 × 1000 px. An 800 × 600 px image stays the same size. After trimming margins, resizing uses the trimmed dimensions. SVG is not resized.",
    ),
    ("SVG: tylko marginesy", "SVG: margins only"),
    (
        "SVG pozostaje SVG. Bez optymalizacji, konwersji i usuwania metadanych. Trim zmienia wyłącznie obszar dokumentu.",
        "SVG stays SVG. No optimization, conversion or metadata removal. Trimming only changes the document canvas.",
    ),
    ("Wygląd", "Appearance"),
    ("Kontrola nad jakością", "Quality controls"),
    (
        "Jakość 1–100: wyżej = więcej szczegółów i zwykle większy plik. 82 nie oznacza 82% rozmiaru ani jakości oryginału.",
        "Quality 1–100: higher means more detail and usually a larger file. 82 does not mean 82% of the original size or quality.",
    ),
    ("Dokładniejsza kompresja", "More thorough compression"),
    (
        "Poświęca więcej czasu na kompresję. Zachowuje wybrany tryb i jakość; nie gwarantuje mniejszego wyniku.",
        "Spends more time compressing. Keeps the selected mode and quality; a smaller file is not guaranteed.",
    ),
    ("Co oznacza bezstratnie?", "What does lossless mean?"),
    (
        "Bezstratny WebP zachowuje piksele gotowego obrazu po renderowaniu, przycięciu i skalowaniu. Jakość 1–100 nie wpływa na ten tryb. Dla prostych logo może dać mniejszy plik niż kompresja stratna.",
        "Lossless WebP preserves the finished image's pixels after rendering, trimming and resizing. Quality 1–100 does not affect this mode. For simple logos it can produce a smaller file than lossy compression.",
    ),
    (
        "Import dokumentów i marginesy",
        "Document import and margins",
    ),
    (
        "Strona AI/PDF lub wielostronicowego TIFF",
        "Page of an AI/PDF or multipage TIFF",
    ),
    (
        "DPI określa szczegółowość renderowania AI/PDF/EPS do pikseli. Więcej DPI = więcej szczegółów, pamięci i czasu. 300 to domyślna wartość; szerokość wyniku ustawiasz osobno.",
        "DPI controls the detail when rendering AI/PDF/EPS to pixels. Higher DPI means more detail, memory and time. The default is 300; output width is set separately.",
    ),
    (
        "Strona: wybiera jedną stronę AI/PDF lub wielostronicowego TIFF. HEIC używa obrazu głównego.",
        "Page: selects one page from an AI/PDF or multipage TIFF. HEIC uses its primary image.",
    ),
    (
        "0 przycina tylko całkowicie przezroczyste piksele lub tło o identycznym kolorze. Wyższa wartość uznaje też prawie przezroczyste i podobne kolory za margines. Może przyciąć delikatny cień. Domyślnie: 8. Nie dotyczy SVG.",
        "0 trims only fully transparent pixels or an exact background color. Higher values also treat nearly transparent pixels and similar colors as margins. This may trim a faint shadow. Default: 8. Does not apply to SVG.",
    ),
    (
        "Jak działa przycinanie marginesów (trim)?",
        "How does margin trimming work?",
    ),
    (
        "Najpierw przycinamy, potem skalujemy. Jednolite tło rozpoznajemy po czterech narożnikach; jeśli się różnią, przycinamy tylko przezroczystość. Całkiem pusty obraz pozostaje bez zmian.",
        "We trim first, then resize. A solid background is detected from all four corners; if they differ, only transparency is trimmed. An entirely blank image stays unchanged.",
    ),
    ("SVG jest chroniony", "SVG is protected"),
    (
        "Zawsze pozostaje SVG. Trim zmienia wyłącznie obszar dokumentu. Bez optymalizacji, skalowania i usuwania metadanych.",
        "Always stays SVG. Trimming only changes the document canvas. No optimization, resizing or metadata removal.",
    ),
    ("AI, TIFF i HEIC", "AI, TIFF and HEIC"),
    (
        "AI musi być zapisany ze zgodnością PDF. HEIC używa obrazu głównego. Numer strony dotyczy AI/PDF i wielostronicowego TIFF. Format „Oryginał” zapisuje te wejścia jako WebP.",
        "AI must be saved with PDF compatibility. HEIC uses its primary image. Page selection applies to AI/PDF and multipage TIFF. The “Original” format exports these inputs as WebP.",
    ),
    ("Dopasuj do swojego Maca", "Make it yours"),
    (
        "Tryb systemowy dopasowuje wygląd do ustawień systemu.",
        "System appearance follows your operating system settings.",
    ),
    ("Skróty klawiaturowe", "Keyboard shortcuts"),
    (
        "⌘O   Dodaj pliki       ⌘,   Ustawienia       Esc   Zamknij panel",
        "⌘O   Add files       ⌘,   Settings       Esc   Close panel",
    ),
    (
        "Obrazy są przetwarzane lokalnie. PNG, JPG, WebP, AVIF i JXL zawsze zapisują się bez EXIF, GPS i pozostałych metadanych. SVG zachowuje oryginalną zawartość.",
        "Images are processed locally. PNG, JPG, WebP, AVIF and JXL are always saved without EXIF, GPS or other metadata. SVG keeps its original contents.",
    ),
    (
        "Zmiany dotyczą plików czekających na start. Trwająca konwersja zachowuje swoje ustawienia.",
        "Changes apply to files awaiting start. Running conversions keep their settings.",
    ),
    ("Wyczyść listę", "Clear list"),
    (
        "Usuwa tylko pozycje z listy. Zapisane pliki pozostają na dysku.",
        "Only clears the list. Saved files stay on disk.",
    ),
    ("Szczegóły: {0}", "Details: {0}"),
    ("Kliknij, aby zobaczyć szczegóły", "Click to view details"),
    ("W kolejce · {0}", "Queued · {0}"),
    (
        "Kliknij, aby przeczytać cały komunikat",
        "Click to read the full message",
    ),
    ("Pokaż wynik w folderze", "Show result in folder"),
    ("Błąd ›", "Error ›"),
    ("Szczegóły pliku", "File details"),
    ("Zapisany plik", "Saved file"),
    ("Pokaż w folderze  ↗", "Show in folder  ↗"),
    ("Zadanie anulowano.", "Task cancelled."),
    (
        "Plik jest w kolejce przetwarzania.",
        "The file is queued for processing.",
    ),
    (
        "Wyniki zapisują się tutaj. Kliknij, aby otworzyć folder.",
        "Results are saved here. Click to open the folder.",
    ),
    ("Oszczędzono {0}", "Saved {0}"),
    ("EXIF i GPS usuwane", "EXIF and GPS removed"),
    (
        "PNG, JPG, WebP, AVIF i JXL zawsze bez metadanych. SVG zachowuje oryginalną zawartość.",
        "PNG, JPG, WebP, AVIF and JXL are always metadata-free. SVG keeps its original contents.",
    ),
    ("Trwa optymalizacja", "Optimization in progress"),
    (
        "Anulować kolejkę i zamknąć aplikację?",
        "Cancel the queue and close the app?",
    ),
    ("Pracuj dalej", "Keep working"),
    ("Anuluj i zamknij", "Cancel and close"),
    ("Nie przycinaj", "Do not trim"),
    ("Przytnij przezroczyste brzegi", "Trim transparent edges"),
    (
        "Przytnij jednolite tło i przezroczystość",
        "Trim solid and transparent edges",
    ),
    (
        "Zostawia wszystkie brzegi obrazu, także białe i przezroczyste marginesy.",
        "Keeps all image edges, including white and transparent margins.",
    ),
    (
        "Odcina puste, przezroczyste brzegi, np. wokół logo PNG. Białe tło zostaje.",
        "Removes empty transparent edges, e.g. around a PNG logo. White backgrounds stay.",
    ),
    (
        "Odcina brzegi w kolorze narożników oraz przezroczyste. Nie wycina tła ze środka logo.",
        "Removes edges matching the corner colors, plus transparency. Does not remove the background inside a logo.",
    ),
    (
        "WebP: do internetu, także z przezroczystym tłem.",
        "WebP: for the web, including transparent backgrounds.",
    ),
    (
        "PNG: grafiki i logo z przezroczystością. Tryb kompresji wybierz poniżej.",
        "PNG: graphics and logos with transparency. Choose a compression mode below.",
    ),
    (
        "JPG: do zdjęć. Przezroczyste miejsca zostaną wypełnione bielą.",
        "JPG: for photos. Transparent areas are filled with white.",
    ),
    (
        "Zachowuje PNG/JPG/WebP/AVIF/JXL. AI/PDF/EPS/TIFF/HEIC → WebP. SVG zawsze → SVG.",
        "Keeps PNG/JPG/WebP/AVIF/JXL. AI/PDF/EPS/TIFF/HEIC → WebP. SVG always → SVG.",
    ),
    (
        "Zmniejsza plik kosztem części szczegółów. Wyższa jakość zwykle daje większy plik.",
        "Reduces file size by discarding some detail. Higher quality usually means a larger file.",
    ),
    (
        "Zachowuje piksele po przycięciu i skalowaniu. Dobre do logo; jakość 1–100 nie ma tu wpływu.",
        "Preserves pixels after trimming and resizing. Good for logos; quality 1–100 has no effect here.",
    ),
    (
        "Lekko upraszcza piksele, aby zmniejszyć plik. Wyższa jakość zachowuje więcej szczegółów.",
        "Slightly simplifies pixels to reduce file size. Higher quality preserves more detail.",
    ),
    (
        "OxiPNG: zmniejsza plik bez zmiany pikseli gotowego obrazu.",
        "OxiPNG: reduces file size without changing the finished image's pixels.",
    ),
    (
        "Paleta: ogranicza liczbę kolorów. Może dać mniejszy plik, ale zmienia wygląd obrazu.",
        "Palette: reduces the number of colors. Can produce a smaller file but changes the image's appearance.",
    ),
    (
        "Zopfli: szuka lepszej kompresji bez zmiany pikseli. Działa wolniej; wynik nie zawsze jest mniejszy.",
        "Zopfli: seeks better compression without changing pixels. Slower; the result is not always smaller.",
    ),
    (
        "Niższa jakość daje zwykle mniejszy plik i mniej szczegółów. 82 to punkt wyjścia do zdjęć.",
        "Lower quality usually gives a smaller file with less detail. 82 is a starting point for photos.",
    ),
    (
        "Każdy format używa własnych ustawień kompresji. „Oryginał” nie oznacza kopii bez zmian.",
        "Each format uses its own compression settings. “Original” does not mean an unchanged copy.",
    ),
    ("OxiPNG · bezstratnie", "OxiPNG · lossless"),
    ("Paleta · mniejszy plik", "Palette · smaller file"),
    ("Zopfli · dokładniej, wolniej", "Zopfli · thorough, slower"),
    ("Stratny · zdjęcia", "Lossy · photos"),
    ("Bezstratny · grafiki", "Lossless · graphics"),
    ("Prawie bezstratny", "Near-lossless"),
    ("Ogólne", "General"),
    ("O aplikacji", "About"),
    ("Język / Language", "Language / Język"),
    ("Systemowy (domyślnie)", "System (default)"),
    (
        "Język systemu: {0}. Dla języków innych niż polski używamy angielskiego.",
        "System language: {0}. Languages other than Polish use English.",
    ),
    (
        "Wybór języka jest zapamiętywany. Zmiana działa od razu.",
        "Your language choice is saved and takes effect immediately.",
    ),
    (
        "Nie udało się zapisać języka. Wybór działa do zamknięcia aplikacji.",
        "Could not save the language. Your choice will apply until the app closes.",
    ),
    ("Wersja {0}", "Version {0}"),
    ("Autor", "Author"),
    (
        "Bezpłatna aplikacja open source",
        "Free and open-source app",
    ),
    (
        "Możesz używać aplikacji prywatnie i komercyjnie, modyfikować ją oraz udostępniać zgodnie z GNU GPL v3 lub nowszą. Przy rozpowszechnianiu obowiązują warunki tej licencji, w tym udostępnienie kodu źródłowego. Aplikacja jest dostarczana bez gwarancji.",
        "You may use the app personally and commercially, modify it and share it under GNU GPL v3 or later. Distribution is subject to the license terms, including providing source code. The app is provided without warranty.",
    ),
    ("Warunki licencji GPL  ↗", "GPL license terms  ↗"),
    ("Licencje bibliotek", "Library licenses"),
    ("Pełna licencja", "Full license"),
    (
        "Nie można otworzyć informacji o licencji: {0}",
        "Cannot open license information: {0}",
    ),
    (
        "Nie można znaleźć folderu użytkownika",
        "Cannot locate your home folder",
    ),
    (
        "Jakość musi być w zakresie 1–100",
        "Quality must be between 1 and 100",
    ),
    (
        "Szerokość musi być większa od zera",
        "Width must be greater than zero",
    ),
    (
        "Obsługiwane formaty: PNG, JPG, WebP, AVIF, JXL, AI/PDF, EPS, TIFF, HEIC/HEIF, SVG (tylko trim)",
        "Supported formats: PNG, JPG, WebP, AVIF, JXL, AI/PDF, EPS, TIFF, HEIC/HEIF, SVG (trim only)",
    ),
    (
        "DPI musi być w zakresie 72–600",
        "DPI must be between 72 and 600",
    ),
    (
        "Numer strony zaczyna się od 1",
        "Page numbering starts at 1",
    ),
    (
        "Tolerancja trim musi być w zakresie 0–64",
        "Trim tolerance must be between 0 and 64",
    ),
    ("Nie można otworzyć pliku", "Cannot open the file"),
    ("Przeciągnij pliki, nie foldery", "Drop files, not folders"),
    (
        "Plik przekracza limit 128 MB",
        "The file exceeds the 128 MB limit",
    ),
    ("SVG · bez optymalizacji", "SVG · no optimization"),
    (
        "Ten AI nie zawiera PDF (np. starszy PostScript). Zapisz go w Illustratorze z ‘Create PDF Compatible File’ lub wyeksportuj PDF.",
        "This AI contains no PDF data (e.g. older PostScript). Save it in Illustrator with ‘Create PDF Compatible File’ or export a PDF.",
    ),
    ("Rasteryzacja AI/PDF: {0} DPI", "AI/PDF rendering: {0} DPI"),
    (
        "Strona {0} z {1} (wybrana w ustawieniach)",
        "Page {0} of {1} (selected in settings)",
    ),
    (
        "Ten format wejściowy jest eksportowany do WebP",
        "This input format is exported as WebP",
    ),
    ("Nie rozpoznano obrazu", "Image format not recognized"),
    (
        "Obsługiwane formaty: PNG, JPG i WebP",
        "Supported formats: PNG, JPG and WebP",
    ),
    ("Nie można odczytać obrazu", "Cannot read the image"),
    (
        "Obraz przekracza limit 64 megapikseli",
        "The image exceeds the 64 megapixel limit",
    ),
    (
        "Nieprawidłowe dane orientacji EXIF",
        "Invalid EXIF orientation data",
    ),
    ("Nieprawidłowy profil kolorów", "Invalid color profile"),
    ("Nie można zdekodować obrazu", "Cannot decode the image"),
    (
        "Trim: {0} × {1} do {2} × {3} px (przed skalowaniem)",
        "Trim: {0} × {1} to {2} × {3} px (before resizing)",
    ),
    (
        "Trim: brak pustych marginesów do usunięcia",
        "Trim: no empty margins to remove",
    ),
    (
        "WebP obsługuje maksymalnie 16383 px na bok. Wybierz mniejszy rozmiar.",
        "WebP supports a maximum of 16383 px per side. Choose a smaller size.",
    ),
    (
        "Metadane usunięte: EXIF/GPS, XMP/IPTC, komentarze i profile. Orientacja i kolory zastosowane przed zapisem.",
        "Metadata removed: EXIF/GPS, XMP/IPTC, comments and profiles. Orientation and colors were applied before saving.",
    ),
    ("Błąd OxiPNG", "OxiPNG error"),
    ("Błąd kodera MozJPEG", "MozJPEG encoder error"),
    ("Nieprawidłowy profil ICC", "Invalid ICC profile"),
    (
        "Ten profil ICC nie jest RGB ani grayscale. Wyeksportuj obraz do sRGB przed konwersją.",
        "This ICC profile is neither RGB nor grayscale. Export the image to sRGB before converting.",
    ),
    (
        "Animowane PNG/WebP nie są obsługiwane; animacja nie została zmieniona",
        "Animated PNG/WebP is not supported; the animation was not changed",
    ),
    ("Nieprawidłowy rozmiar obrazu", "Invalid image dimensions"),
    (
        "Nie można utworzyć folderu wynikowego",
        "Cannot create the output folder",
    ),
    (
        "Brak dostępu do folderu wynikowego",
        "Cannot access the output folder",
    ),
    ("Nie można zapisać obrazu", "Cannot save the image"),
    ("Plik nie ma nazwy", "The file has no name"),
    (
        "Nie można zapisać pliku wynikowego",
        "Cannot save the output file",
    ),
    (
        "Zbyt wiele plików z tą samą nazwą",
        "Too many files with the same name",
    ),
    (
        "Obraz przekracza limit 64 megapikseli. Dla AI/PDF zmniejsz DPI.",
        "The image exceeds the 64 megapixel limit. For AI/PDF, reduce DPI.",
    ),
    (
        "Za mało pamięci do odczytu obrazu",
        "Not enough memory to read the image",
    ),
    (
        "PDF/AI jest zabezpieczony hasłem. Zapisz odblokowaną kopię.",
        "The PDF/AI is password-protected. Save an unlocked copy.",
    ),
    (
        "Brak strony {0}. Liczba stron/obrazów: {1}. HEIC udostępnia obraz główny jako stronę 1.",
        "Page {0} does not exist. Number of pages/images: {1}. HEIC provides its primary image as page 1.",
    ),
    (
        "AI zapisano bez zawartości PDF. W Illustratorze włącz ‘Create PDF Compatible File’ lub wyeksportuj PDF.",
        "The AI was saved without PDF content. Enable ‘Create PDF Compatible File’ in Illustrator or export a PDF.",
    ),
    (
        "macOS nie może odczytać obrazu. AI musi zawierać zgodną reprezentację PDF.",
        "macOS cannot read the image. AI must contain a compatible PDF representation.",
    ),
    (
        "Nieprawidłowy wynik dekodera macOS",
        "Invalid macOS decoder output",
    ),
    ("Nieprawidłowy bufor macOS", "Invalid macOS buffer"),
    (
        "AI/PDF, TIFF i HEIC wymagają macOS",
        "AI/PDF, TIFF and HEIC require macOS",
    ),
    (
        "Wyłącz Trim, aby zapisać SVG bez zmian. Tekst można wcześniej zamienić na krzywe w edytorze.",
        "Disable Trim to save the SVG unchanged. You can first convert text to outlines in an editor.",
    ),
    (
        "SVG musi być zapisany jako UTF-8",
        "SVG must be saved as UTF-8",
    ),
    ("Nieprawidłowy dokument SVG", "Invalid SVG document"),
    (
        "Plik nie zawiera dokumentu SVG",
        "The file does not contain an SVG document",
    ),
    (
        "SVG: kopia identyczna bajt w bajt; bez konwersji, skalowania i optymalizacji.",
        "SVG: byte-for-byte identical copy; no conversion, resizing or optimization.",
    ),
    (
        "Nie można bezpiecznie przyciąć tego SVG. Wyłącz Trim, aby zapisać SVG bez zmian. Tekst można wcześniej zamienić na krzywe w edytorze.",
        "This SVG cannot be safely trimmed. Disable Trim to save the SVG unchanged. You can first convert text to outlines in an editor.",
    ),
    (
        "Nie można wyznaczyć granic SVG",
        "Cannot determine SVG bounds",
    ),
    (
        "SVG: brak widocznej geometrii; plik pozostawiony bez zmian.",
        "SVG: no visible geometry; the file was left unchanged.",
    ),
    (
        "Nieprawidłowe preserveAspectRatio",
        "Invalid preserveAspectRatio",
    ),
    (
        "SVG: brak pustych marginesów do usunięcia; plik pozostawiony bez zmian.",
        "SVG: no empty margins to remove; the file was left unchanged.",
    ),
    (
        "SVG trim: zmieniono tylko viewBox, width i height głównego elementu. Zawartość, kolory i metadane zachowane; bez optymalizacji i usuwania tła.",
        "SVG trim: only the root element's viewBox, width and height were changed. Contents, colors and metadata preserved; no optimization or background removal.",
    ),
    (
        "Nietypowy zapis atrybutu {0}; SVG nie został zmieniony",
        "Unusual attribute syntax for {0}; SVG was not changed",
    ),
    ("Brak znacznika SVG", "Missing SVG tag"),
    (
        "Transformacja głównego elementu SVG",
        "Transform on the SVG root element",
    ),
    (
        "Klasa CSS głównego elementu SVG",
        "CSS class on the SVG root element",
    ),
    ("Zewnętrzny arkusz CSS", "External CSS stylesheet"),
    ("Element spoza SVG: {0}", "Non-SVG element: {0}"),
    (
        "Element wymagający ręcznego trim: {0}",
        "Element requires manual trimming: {0}",
    ),
    ("Zagnieżdżony obszar SVG", "Nested SVG viewport"),
    (
        "Atrybut wymagający ręcznego trim: {0}",
        "Attribute requires manual trimming: {0}",
    ),
    (
        "Widoczna zawartość poza obszarem SVG",
        "Visible content outside the SVG viewport",
    ),
    ("Zewnętrzne odwołanie SVG", "External SVG reference"),
    (
        "Nieobsługiwany kolor lub zewnętrzna farba SVG",
        "Unsupported color or external SVG paint",
    ),
    ("Nieobsługiwane przycięcie SVG", "Unsupported SVG clipping"),
    ("Nieprawidłowa transformacja SVG", "Invalid SVG transform"),
    ("Nieprawidłowa ścieżka SVG", "Invalid SVG path"),
    ("Nieobsługiwana długość SVG", "Unsupported SVG length"),
    ("Nieprawidłowa długość SVG", "Invalid SVG length"),
    (
        "Wymiary względne mogłyby zmienić grafikę podczas trim",
        "Relative dimensions could change the artwork when trimming",
    ),
    ("Nieprawidłowy komentarz CSS", "Invalid CSS comment"),
    ("Nieobsługiwany CSS", "Unsupported CSS"),
    (
        "Trim obsługuje tylko proste selektory klas CSS",
        "Trimming supports only simple CSS class selectors",
    ),
    ("Złożony selektor CSS", "Complex CSS selector"),
    ("Nieprawidłowy CSS", "Invalid CSS"),
    ("Nieprawidłowy styl SVG", "Invalid SVG style"),
    (
        "Styl wymagający ręcznego trim: {0}",
        "Style requires manual trimming: {0}",
    ),
    ("Dynamiczny styl SVG", "Dynamic SVG style"),
    (
        "Nieobsługiwany kolor lub zewnętrzna farba CSS",
        "Unsupported color or external CSS paint",
    ),
    (
        "Ten PNG używa nieobsługiwanego profilu cICP/HDR. Wyeksportuj go do sRGB przed usunięciem metadanych.",
        "This PNG uses an unsupported cICP/HDR profile. Export it to sRGB before removing metadata.",
    ),
    ("Nieprawidłowa gamma PNG", "Invalid PNG gamma"),
    (
        "Nie można przeliczyć kolorów PNG przed usunięciem metadanych",
        "Cannot convert PNG colors before removing metadata",
    ),
    ("Niepełny wynik kodera", "Incomplete encoder output"),
    (
        "Niepełny blok wyniku kodera",
        "Incomplete encoder output block",
    ),
    (
        "Koder pozostawił metadane; wynik nie został zapisany",
        "The encoder left metadata in the result; the file was not saved",
    ),
    ("Nieprawidłowy rozmiar bloku", "Invalid block size"),
    ("Nieprawidłowy wynik JPEG", "Invalid JPEG output"),
    ("Nieprawidłowy znacznik JPEG", "Invalid JPEG marker"),
    ("Urwany znacznik JPEG", "Truncated JPEG marker"),
    ("Urwany blok JPEG", "Truncated JPEG block"),
    (
        "Nieprawidłowy rozmiar bloku JPEG",
        "Invalid JPEG block size",
    ),
    ("Brak końca obrazu JPEG", "Missing JPEG end-of-image marker"),
    ("Błąd konfiguracji WebP", "WebP configuration error"),
    ("Nieprawidłowy rozmiar WebP", "Invalid WebP dimensions"),
    ("Nieprawidłowe parametry WebP", "Invalid WebP parameters"),
    ("Błąd inicjalizacji WebP", "WebP initialization error"),
    ("Za mało pamięci dla WebP", "Not enough memory for WebP"),
    ("Błąd kodera WebP: {0}", "WebP encoder error: {0}"),
    ("Pusty wynik WebP", "Empty WebP output"),
    (
        "Nieprawidłowy nagłówek binarnego EPS",
        "Invalid binary EPS header",
    ),
    (
        "Plik nie zawiera poprawnego EPS/PostScript",
        "The file does not contain valid EPS/PostScript",
    ),
    (
        "EPS ma nieprawidłowy obszar BoundingBox",
        "EPS has an invalid BoundingBox",
    ),
    (
        "EPS nie określa obszaru BoundingBox",
        "EPS does not define a BoundingBox",
    ),
    (
        "EPS przekracza limit 64 megapikseli lub 65535 px na bok. Zmniejsz DPI.",
        "EPS exceeds 64 megapixels or 65535 pixels per side. Reduce DPI.",
    ),
    (
        "EPS wymaga Ghostscript. Zainstaluj aktualny Ghostscript lub użyj pełnej paczki WebDrop dla macOS.",
        "EPS requires Ghostscript. Install the current Ghostscript release or use the full WebDrop package for macOS.",
    ),
    (
        "Nie można uruchomić Ghostscript dla EPS",
        "Cannot start Ghostscript for EPS",
    ),
    (
        "Przekroczono limit czasu odczytu EPS (60 s)",
        "EPS rendering timed out (60 s)",
    ),
    (
        "Przekroczono limit danych renderowania EPS",
        "EPS rendering exceeded the output data limit",
    ),
    ("Nie można odczytać EPS: {0}", "Cannot read EPS: {0}"),
    (
        "Ghostscript nie utworzył obrazu EPS",
        "Ghostscript did not produce an EPS image",
    ),
    (
        "EPS ma jedną stronę. Ustaw numer strony na 1.",
        "EPS has one page. Set the page number to 1.",
    ),
    (
        "Nieprawidłowy obraz z EPS",
        "Invalid image rendered from EPS",
    ),
    (
        "Rasteryzacja EPS: {0} DPI (Ghostscript)",
        "EPS rendering: {0} DPI (Ghostscript)",
    ),
    (
        "EPS: renderowanie na przezroczystym tle przez Ghostscript. Zawiera jedną stronę; preset Logo przycina marginesy. Na Windows/Linux zainstaluj Ghostscript.",
        "EPS: rendered on a transparent background using Ghostscript. Contains one page; the Logo preset trims margins. On Windows/Linux, install Ghostscript.",
    ),
];
