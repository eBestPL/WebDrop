# WebDrop — Image Optimizer

[English](README.en.md) · [Polski](README.md)

**Mniejsze obrazy. Szybszy web. Wszystko lokalnie.**

Przeciągnij pliki lub foldery, wybierz ustawienia i zatwierdź konwersję — pojedynczo lub całą paczką.

## ⬇️ Pobierz WebDrop 0.9.2

**Wybierz swój system — bez instalowania Rusta, terminala i kompilowania.**

| System | Pobieranie | Co zrobić po pobraniu |
| --- | --- | --- |
| 🍎 **macOS 15+ · Apple Silicon** | **[Pobierz na Maca](dist/WebDrop-macOS-arm64.zip?raw=true)** | Rozpakuj ZIP, przeciągnij **WebDrop.app** do **Applications** i otwórz |
| 🪟 **Windows 10/11 · x64 · Wersja testowa** | **[Pobierz na Windows](dist/WebDrop-Windows-x64.zip?raw=true)** | Rozpakuj **cały ZIP** i otwórz **WebDrop.exe**; pliki DLL muszą pozostać obok aplikacji |
| 🐧 **Linux · x64 · Wersja testowa** | **[Pobierz na Linux](dist/WebDrop-Linux-x64.tar.gz?raw=true)** | Rozpakuj archiwum i uruchom plik **WebDrop** |

Wersje Windows i Linux są **eksperymentalne**: skompilowane na macOS, jeszcze bez testów uruchomienia na docelowych systemach. Linux wymaga **glibc 2.41+** (np. Debian 13), OpenGL, X11 lub Wayland, libxkbcommon i działającego portalu pulpitu do wyboru plików. Paczki są przenośnymi archiwami, nie instalatorami. Wersja macOS ma podpis lokalny, bez notaryzacji. Windows nie ma podpisu wydawcy, więc system może wyświetlić komunikat bezpieczeństwa.

**Wszystkie systemy:** PNG, JPG, WebP, AVIF, JPEG XL i trim SVG. **Tylko macOS:** import AI/PDF, TIFF i HEIC/HEIF. Nie dołączono binarek dla Maców Intel, Windows ARM ani Linux ARM.

[Sumy kontrolne SHA-256](dist/SHA256SUMS.txt) · [Licencja](LICENSE)

## Zobacz WebDrop w akcji

![WebDrop na macOS w ciemnym motywie: wyniki seryjnej konwersji PNG i JPG do WebP oraz zachowany SVG](docs/screenshots/batch-dark-pl.jpg)

**Cała paczka w jednym oknie.** Format, szerokość i kompresja po prawej; wyniki oraz przyciski ponowienia po lewej. Zrzut pokazuje rzeczywiste wyniki dla plików demonstracyjnych — oszczędność zależy od obrazu i ustawień.

[🍎 Pobierz na Maca](dist/WebDrop-macOS-arm64.zip?raw=true) · [🪟 Windows — wersja testowa](dist/WebDrop-Windows-x64.zip?raw=true) · [🐧 Linux — wersja testowa](dist/WebDrop-Linux-x64.tar.gz?raw=true)

Mała aplikacja w Rust do lokalnej optymalizacji obrazów i seryjnej konwersji do WebP, AVIF i JPEG XL. GUI: egui/eframe. Bez serwera, konta i wysyłania zdjęć.

Domyślnie zapisuje **WebP, maksymalnie 1500 px szerokości, jakość 82, do Downloads**. Dodaj obrazy, dostosuj opcje i kliknij **Rozpocznij konwersję**. Samo dodanie plików nie uruchamia przetwarzania.

Pliki są po lewej, opcje eksportu po prawej. Lista plików może się przewijać, prawa kolumna nie wymaga przewijania. **Ustawienia** z ikoną koła zębatego są w prawym górnym rogu. Presety **Na web** i **Logo** przyspieszają wybór opcji. Interfejs oferuje język polski i angielski oraz jasny, ciemny i systemowy wygląd.

Po zakończeniu użyj **Ponów** przy pliku lub **Ponów wszystkie**, zmień opcje i ponownie kliknij start. Oryginały i wcześniejsze wyniki pozostają na dysku; kolizje nazw otrzymują kolejny numer. Nowe pliki dodane podczas pracy czekają na następne zatwierdzenie.

## Miejsce zapisu

Pole **Zapisz w** nad przyciskiem startu oferuje:

- **Downloads** — domyślny folder pobierania.
- **Folder źródłowy / Same path** — każdy wynik obok swojego oryginału, również dla plików z różnych folderów w jednej paczce.
- **Własny folder / Custom path** — wybór folderu przez natywne okno systemowe; **Wybierz…** pozwala go zmienić. Anulowanie okna pozostawia poprzedni wybór.

Tryb i ostatnia własna ścieżka są zapamiętywane w preferencjach macOS (`OutputDestination`) po ponownym uruchomieniu, niezależnie od języka. Powrót do Custom path przywraca ostatnio wybrany własny folder. Cel jest ustalany osobno dla każdego pliku przy kliknięciu **Rozpocznij konwersję**; zmiana w trakcie pracy dotyczy kolejnego startu, także po **Ponów**. Nie ma automatycznego przełączania do Downloads, gdy wybrany folder jest niedostępny — aplikacja pokaże błąd. Oryginały i wcześniejsze wyniki nigdy nie są nadpisywane; kolizje nazw otrzymują numer, np. `logo (1).webp`. Opcja CLI `--output` pozostaje niezależna od preferencji GUI.

Na Windows i Linux język oraz miejsce zapisu są przechowywane w pliku `preferences.json` w katalogu konfiguracji użytkownika. Import dokumentów AI/PDF, TIFF i HEIC jest dostępny wyłącznie na macOS.

## Język i informacje o aplikacji

W **Ustawienia → Ogólne → Język / Language** można wybrać **Systemowy** (domyślnie), **Polski · PL** lub **English · EN**. Tryb systemowy odczytuje preferowany język systemu: polski wybiera PL, pozostałe języki wybierają EN. Wybór działa od razu i jest zapamiętywany między uruchomieniami w preferencjach aplikacji (`local.imageoptimizer.desktop`, klucz `InterfaceLanguage`). Powrót do „Systemowy” usuwa ręczne ustawienie. Kontrolki, opisy, podpowiedzi oraz komunikaty wyników i błędów mają obie wersje językowe. Natywne okna macOS, np. wybór plików, używają języka systemowego. Nazwy plików i ścieżki pozostają niezmienione.

**Ustawienia → O aplikacji** pokazuje wersję z metadanych Cargo, autora **Jędrzeja Czarneckiego**, kontakt [info@ebest.pl](mailto:info@ebest.pl) i stronę [ebest.pl](https://ebest.pl). Aplikacja jest bezpłatna i open source na licencji **GNU GPL v3 lub nowszej**. Można korzystać z niej prywatnie i komercyjnie, modyfikować oraz rozpowszechniać zgodnie z warunkami GPL, w tym obowiązkiem udostępnienia kodu źródłowego przy rozpowszechnianiu. Bez gwarancji. Pełna licencja i licencje bibliotek są dostępne z tego panelu i dołączone do paczki aplikacji.

## Uruchomienie

Pobierz paczkę dla swojego systemu z tabeli na górze, rozpakuj ją i otwórz aplikację.

1. Przeciągnij pliki lub całe foldery do okna, użyj **Wybierz pliki…** / **⌘O** albo wybierz w Finderze **Otwórz w aplikacji → WebDrop**.
2. Dodane pliki czekają. Ustaw format, rozmiar i trim, następnie kliknij **Rozpocznij konwersję**. Domyślnie: **WebP, maks. 1500 px szerokości, jakość 82**.
3. Wyniki domyślnie trafiają do **Downloads**. W polu **Zapisz w** możesz wybrać **Folder źródłowy** (Same path) lub **Własny folder** (Custom path). Przycisk **↗** przy wyniku pokazuje plik w Finderze. Kliknięcie nazwy pliku otwiera szczegóły, w tym pełny komunikat błędu, użyty silnik, informacje o konwersji i ścieżkę wyniku.

**Całe foldery:** aplikacja zbiera obsługiwane obrazy także ze wszystkich podfolderów. Wyszukiwanie odbywa się w tle i można je anulować. Pomijane są pliki i foldery o nazwach zaczynających się od kropki, dowiązania znalezione wewnątrz folderów oraz nieobsługiwane formaty. Nakładające się foldery nie dodają tego samego pliku ponownie do oczekującej kolejki. Brak dostępu do części zawartości jest zgłaszany, a pozostałe obrazy nadal trafiają do listy. Konwersję zatwierdzasz przyciskiem po zakończeniu wyszukiwania. Przy zapisie do Downloads lub własnego folderu wyniki trafiają bezpośrednio do niego; struktura podfolderów nie jest odtwarzana. Same path zapisuje każdy wynik obok oryginału.

Format wybierasz z listy z opisami, a szerokość segmentami pod presetami. **Na web** przywraca WebP stratny, jakość 82, szerokość do 1500 px i wyłączony trim. **Logo** wybiera WebP bezstratny, szerokość do 1500 px i trim tła oraz przezroczystości. Aktywny preset jest wyróżniony; po zmianie jego głównych ustawień wyróżnienie znika. Wiersz **Kompresja** pokazuje tryb WebP/AVIF/JXL i edytowalną jakość, wybór silnika PNG lub jakość JPG. „Pełna” oznacza brak zmiany wymiarów. Duża strefa przeciągania zmniejsza się po dodaniu plików, zostawiając miejsce na kolejkę, pasek postępu i wyniki. **⌘,** otwiera ustawienia kompresji, importu, języka i wyglądu; **⌘O** dodaje pliki. Okno można przeciągać za środek paska tytułowego. Motyw i ustawienia eksportu wracają do wartości domyślnych po ponownym uruchomieniu.

Od wersji 0.4.2 aplikacja obsługuje systemowe „Otwórz w aplikacji” dla PNG, JPG, WebP, AVIF, JXL, TIFF, HEIC/HEIF, AI, PDF i SVG, także dla wielu plików naraz i podczas działania aplikacji. Pliki trafiają do kolejki i czekają na przycisk **Rozpocznij konwersję**, także przy otwarciu z Findera i podczas trwania innej konwersji. Ustawienia są pobierane w momencie kliknięcia startu (przy nowym uruchomieniu: WebP/1500 px). Rejestruje się jako dodatkowa aplikacja, bez zmiany domyślnego programu do otwierania obrazów.

Od wersji 0.5.1 opcja **Marginesy** oznacza przycinanie (trim). Pod formatem, marginesami i kompresją widać opis aktualnego wyboru. Listy rozwijane objaśniają poszczególne tryby. **Nie przycinaj** zachowuje wszystkie brzegi; **Przytnij przezroczyste brzegi** zostawia białe tło; **Przytnij jednolite tło i przezroczystość** rozpoznaje kolor tła po narożnikach i przycina zewnętrzne marginesy, bez usuwania tła ze środka logo. Ustawienia **Import** opisują DPI, numer strony i tolerancję oraz wpływ tolerancji na delikatne cienie. Reguły dla SVG pozostają osobne: wyłącznie zmiana obszaru dokumentu.

### Logo AI → przycięty WebP

![WebDrop na macOS w jasnym motywie: logo AI czekające na start, preset Logo, bezstratny WebP i przycinanie marginesów](docs/screenshots/logo-light-pl.jpg)

*Jasny motyw i preset Logo: ustaw trim oraz bezstratny WebP przed kliknięciem startu. Import AI/PDF jest dostępny na macOS.*

Dodaj plik `.ai`, wybierz **Logo**, a następnie kliknij **Rozpocznij konwersję**. Ten preset ustawia bezstratny WebP, trim jednolitego tła i przezroczystych marginesów oraz maks. 1500 px. Wynik trafia do wybranego folderu (domyślnie Downloads) pod nazwą logo z rozszerzeniem `.webp`.

- **Trim: przezroczyste marginesy** przycina do widocznych pikseli (ignoruje ukryte RGB pod alfą).
- **Trim: jednolite tło + alfa** dodatkowo przycina marginesy w kolorze narożników, jeśli wszystkie cztery narożniki są zgodne. Nadaje się np. do logo na białym tle. Gdy narożniki są różne, przycinane są tylko przezroczyste marginesy.
- Trim działa **przed zmianą rozmiaru**, zachowuje proporcje i nie usuwa tła wewnątrz logo. Całkowicie pusty/jednolity obraz pozostaje w swoim rozmiarze. Tolerancja 0–64 w ustawieniach kompresji kontroluje ignorowanie drobnych różnic i prawie przezroczystych pikseli (domyślnie 8).
- Trim jest domyślnie wyłączony dla zwykłych zdjęć. Preset logo należy wybrać **przed kliknięciem Rozpocznij konwersję**; nie zmienia już rozpoczętych zadań.

**Dodatkowe wejścia na macOS:** AI, PDF, TIF/TIFF, HEIC/HEIF. Import używa wbudowanych w macOS Image I/O i Core Graphics, bez dodatkowej instalacji. AI/PDF renderowane są na przezroczystym tle w sRGB, domyślnie 300 DPI. Ustawienia pozwalają wybrać 72/144/300/600 DPI i numer strony/artboardu AI/PDF lub strony wielostronicowego TIFF; domyślnie strona 1. HEIC używa obrazu głównego (pomija pomocnicze obrazy głębi). Wskazówka po najechaniu na wynik pokazuje stronę, DPI oraz rozmiary przed i po trim.

**AI musi być zapisany z [Create PDF Compatible File](https://helpx.adobe.com/illustrator/using/saving-artwork.html).** Starsze AI oparte na PostScript i pliki bez zawartości PDF wymagają ponownego zapisu w Illustratorze lub eksportu do PDF. Aplikacja wykrywa standardową stronę ostrzeżenia Illustratora zamiast konwertować ją do obrazu. Importowane są widoczne dane osadzone w pliku, bez edycji warstw Illustratora. Dokumenty z hasłem wymagają odblokowanej kopii. Dla bardzo dużych stron zmniejsz DPI, aby zmieścić się w limicie 64 megapikseli.

`zdjęcie.png` → `~/Downloads/zdjęcie.webp`. Oryginały nie są nadpisywane. Gdy wynik już istnieje, powstaje `zdjęcie (1).webp`, następnie `(2)` itd. Dotyczy to także plików z różnych katalogów o tej samej nazwie. Zapis jest atomowy i odporny na kolizje między instancjami aplikacji.

### SVG — wyłącznie trim, bez optymalizacji

Przeciągnij SVG tak jak pozostałe pliki. **SVG zawsze zapisuje się jako SVG**, również przy wybranym WebP, presecie „Logo” i szerokości 1500 px. Format, szerokość, jakość i algorytmy kompresji nie wpływają na SVG. Można mieszać SVG ze zdjęciami w jednej kolejce.

- **Trim wyłączony:** kopia identyczna bajt w bajt, bez usuwania metadanych i zmiany kodu.
- **Dowolny włączony tryb trim:** przycięcie pustego obszaru przez zmianę wyłącznie `viewBox`, `width` i `height` głównego elementu SVG. Ścieżki, grupy, kolory, obrysy, definicje, komentarze i metadane pozostają w oryginalnym zapisie. Wymiary płótna zachowują dotychczasową skalę grafiki. Nie ma rasteryzacji ani usuwania białych prostokątów/tła. Tolerancja trim nie dotyczy SVG.
- Granice obejmują obrysy i transformacje. Przycięcie jest konserwatywne: zaokrągla obszar na zewnątrz, a clipPath może pozostawić dodatkowy margines. Pusty lub już przycięty dokument pozostaje bez zmian.
- Automatyczny trim obsługuje statyczne ścieżki i figury, grupy, lokalne odwołania, gradienty i proste klasy CSS (np. `.st0` z Illustratora). Tekst zależny od fontów, animacje, filtry, maski, osadzone/zewnętrzne obrazy, zagnieżdżone obszary SVG, responsywne CSS i względne wymiary geometrii są odrzucane z czytelnym komunikatem **przed zapisem wyniku**. Takie SVG można skopiować z wyłączonym trim; tekst można wcześniej zamienić na krzywe w edytorze.

Wynik: `~/Downloads/logo.svg`; istniejące pliki i źródła są chronione tak samo jak przy innych formatach. Wymagany poprawny SVG w UTF-8, do 128 MB / 100 tys. węzłów XML. Parser nie pobiera zewnętrznych zasobów. Biblioteka usvg służy jedynie do pomiaru — jej przekształcony dokument nigdy nie jest zapisywany.

## Ustawienia

- Format: **WebP**, **AVIF**, **JPEG XL**, **Oryginalny**, **PNG**, **JPG**. Oryginalny zachowuje PNG/JPG/WebP/AVIF/JXL; AI/PDF/TIFF/HEIC eksportuje jako WebP, z informacją przy wyniku. SVG zawsze pozostaje SVG niezależnie od tej opcji.
- Szerokość: **1500, 1000, 700, 500 px** lub oryginalna. Wysokość proporcjonalna; bez powiększania małych obrazów.
- WebP: stratny, bezstratny lub prawie bezstratny. Jakość w trybie prawie bezstratnym określa zachowanie szczegółów; tryb bezstratny ignoruje suwak jakości.
- PNG: bezstratny OxiPNG, stratna paleta libimagequant + OxiPNG lub bezstratny OxiPNG + Zopfli.
- Dokładniejsza kompresja: wyższy nakład obliczeń, szczególnie dla WebP/PNG. Zopfli może trwać zauważalnie dłużej.

Ustawienia są przypisywane do pliku po kliknięciu przycisku startu. Nie zmieniają już trwającego zadania. Po ponownym uruchomieniu wracają domyślne WebP/1500 px. Anulowanie jest sprawdzane między etapami dekodowania, skalowania i kodowania; aktualne wywołanie kodera kończy się przed anulowaniem. Zakończone pliki pozostają na dysku.

## Silniki

Aktualne wersje zależności są zapisane w `Cargo.lock`; wybór sprawdzony 28.10.2026.

| Format / etap | Implementacja |
| --- | --- |
| PNG bezstratny | [OxiPNG](https://github.com/oxipng/oxipng): redukcja palety, głębi, koloru i dobór filtrów, libdeflate |
| PNG dokładny | OxiPNG + [Zopfli](https://github.com/google/zopfli) DEFLATE |
| PNG z paletą | [libimagequant](https://pngquant.org/lib/): kwantyzacja RGBA, dithering, następnie OxiPNG |
| JPEG | [MozJPEG](https://github.com/mozilla/mozjpeg): progresywny JPEG, optymalizacja skanów, kodowania i kwantyzacji |
| WebP | [libwebp 1.6](https://developers.google.com/speed/webp/docs/api), przez `libwebp-sys 0.14.4`: lossy/lossless/near-lossless, sharp YUV, pełna jakość alfa |
| Skalowanie | Lanczos3 w liniowym świetle, premultiplikowana alfa, bez ciemnych obwódek |
| Kolory | Little CMS: osadzony profil ICC RGB/grayscale → sRGB przed ponownym kodowaniem |
| SVG trim | [usvg 0.49](https://docs.rs/usvg/0.49.0/usvg/) do pomiaru granic; roxmltree do wskazania zakresów atrybutów; zapis bez serializacji/optymalizacji SVG |

Żaden algorytm nie daje najmniejszego pliku dla każdego obrazu. Konwersja i kompresja stratna mogą czasem utworzyć większy plik; interfejs pokazuje rzeczywisty wynik, także wzrost rozmiaru.

**Od wersji 0.4.1 każdy wynik PNG/JPG/WebP/AVIF/JXL jest zapisywany bez metadanych**: EXIF/GPS, XMP/IPTC, komentarzy, autora, dat, miniaturek, rozdzielczości/DPI i osadzonych profili ICC. Dotyczy to wszystkich ustawień, także „Oryginalny”, braku skalowania, bezstratnego PNG oraz wejść AI/PDF/TIFF/HEIC. Nie ma przełącznika pozwalającego zachować metadane. Oryginalne pliki pozostają nietknięte. Reguła dotyczy zawartości obrazu, nie systemowych dat utworzenia nowego pliku.

Przed usunięciem metadanych stosowana jest orientacja EXIF oraz konwersja kolorów do sRGB. PNG z ICC, gAMA/cHRM lub cICP Display P3 jest przeliczany przed usunięciem tych bloków. PNG z nieobsługiwanym cICP/HDR jest odrzucany z komunikatem zamiast utraty informacji o kolorach. Bez skalowania i trim bezstratny PNG zachowuje 16-bitową precyzję również przy konwersji ICC. OxiPNG używa `StripChunks::All`. Pozostają tylko bloki kodujące obraz, paletę i przezroczystość. Wyniki PNG/WebP są dodatkowo sprawdzane przed zapisem; JPG nie zawiera nawet opcjonalnych segmentów APP/COM, w tym JFIF. Przezroczystość pozostaje w PNG/WebP/AVIF/JXL; JPEG używa białego tła. Bezstratność kodowania nie oznacza bezstratności zmiany rozmiaru ani konwersji przestrzeni barw.

**Wyjątek: SVG**, zgodnie z zasadą zachowania jego oryginalnej zawartości. SVG zachowuje także metadane — jedyną dozwoloną zmianą pozostaje trim obszaru dokumentu.

Animowane PNG/WebP/AVIF/JXL są odrzucane z komunikatem zamiast utraty animacji. Obsługiwane wejście: statyczne PNG/JPG/WebP/AVIF/JXL, AI/PDF, TIFF i HEIC/HEIF, maks. 128 MB i 64 megapiksele (dla dokumentów: na renderowaną stronę). Profile CMYK/niestandardowe w dotychczasowym dekoderze JPG/PNG są odrzucane, jeśli nie można bezpiecznie wykonać konwersji; import macOS wykonuje konwersję kolorów do sRGB przez Core Graphics. WebP ma limit 16383 px na bok. Obrazy bez informacji o przestrzeni kolorów traktowane są jako sRGB. Skalowanie i trim przetwarzają obrazy w 8 bitach na kanał. HEIC HDR jest eksportowany jako 8-bitowy obraz SDR; dodatkowe dane HDR/głębi nie są zachowywane.

## Budowanie i sprawdzanie

Wymagane do budowania: macOS, Xcode Command Line Tools, Rust 1.95+ (zalecany stable), biblioteki `libavif` i `jpeg-xl` z Homebrew (`brew install libavif jpeg-xl`). Gotowa aplikacja zawiera biblioteki i nie wymaga Homebrew. Obecny pakiet wymaga macOS 15+ ze względu na wersje bibliotek; skrypt ustawia minimalny system zgodnie z dołączonymi bibliotekami.

```sh
./scripts/cargo.sh test --locked
./scripts/cargo.sh run --release
./scripts/bundle.sh
open "dist/WebDrop.app"
```

Skrypt `cargo.sh` używa lokalnego toolchaina w `.tools`, jeśli istnieje, w przeciwnym razie systemowego Cargo. Bundlowanie tworzy ikonę i lokalny podpis ad-hoc. Publiczna dystrybucja wymaga osobnego podpisu Developer ID i notaryzacji.

Testy obejmują formaty i silniki, bezstratny round-trip RGBA dla AVIF/JXL (również ukryte RGB i częściowa alfa), jakość stratną, import AVIF/JXL, orientację, usuwanie metadanych i odrzucanie animacji/HDR, batch PNG/JPG → WebP, orientację EXIF, kolory ICC, przezroczystość, brak ciemnych obwódek, PNG 16-bit, skalowanie, uszkodzone pliki, animacje, anulowanie i równoczesne kolizje nazw.

Testy importu obejmują także rzeczywiste syntetyczne pliki HEIC/TIFF, obrót HEIC, wielostronicowy TIFF, rasteryzację zgodnego AI/PDF, wybór strony, DPI, obrót PDF, wykrywanie strony ostrzeżenia Illustratora oraz trim przed skalowaniem. Dekoder HEIC wymaga dostępu do systemowych usług multimedialnych macOS — uruchamiaj te testy poza restrykcyjnym sandboxem. Pliki testowe w `tests/fixtures` nie zawierają danych użytkownika; generator znajduje się w `scripts/make-test-fixtures.swift`.

Testy SVG sprawdzają identyczność bajtów przy wyłączonym trim, zmianę wyłącznie atrybutów obszaru dokumentu, kolizje nazw, anulowanie oraz odmowę niebezpiecznego trim. Porównują też piksel po pikselu render oryginału i przyciętego SVG: obrysy, transformacje, gradienty, CSS, clipPath, przesunięty viewBox i różne preserveAspectRatio. Renderer resvg jest zależnością wyłącznie testową.

Testy metadanych używają syntetycznych PNG/JPG/WebP/AVIF/JXL z EXIF/GPS, XMP, IPTC, ICC, DPI i komentarzami. Sprawdzają brak bloków metadanych we wszystkich koderach, także przy bezpośrednim PNG i zapisie „Oryginalny”, oraz poprawną orientację, alfę, konwersję gamma/Display P3 i 16-bitowy RGB/grayscale. Nieobsługiwany cICP/HDR nie tworzy wyniku.

### AVIF i JPEG XL

Eksport i import statycznych `.avif` i `.jxl`, także batch i przez „Otwórz w aplikacji”. Oba formaty obsługują trim przed skalowaniem, przezroczystość i zapis do wybranego folderu (domyślnie Downloads) bez nadpisywania oryginałów. „Oryginalny” zachowuje format AVIF/JXL. SVG nadal pozostaje SVG.

- **AVIF**: libavif + AOM, pełna rozdzielczość koloru 4:4:4, alfa bezstratna. Tryb bezstratny używa odwracalnego przekształcenia RGB i jakości 100.
- **JPEG XL**: libjxl, tryb stratny z jakością przeliczaną przez `JxlEncoderDistanceFromQuality`, albo prawdziwy tryb bezstratny. Alfa jest bezstratna, a niewidoczne kolory są zachowywane.
- **Jakość 1–100** dotyczy trybu stratnego i ma inną skalę w każdym kodeku. „Dokładniejsza kompresja” zwiększa nakład obliczeń. Bezstratność dotyczy pikseli po konwersji do 8-bitowego sRGB, przycięciu i skalowaniu, nie oryginału HDR/16-bit.
- **Metadane**: kodery dostają wyłącznie piksele, bez EXIF/GPS, XMP/IPTC, dat, komentarzy i źródłowych profili ICC. AVIF zachowuje strukturalny opis kolorów sRGB (CICP/nclx), a JXL własne parametry koloru konieczne do poprawnego dekodowania. To nie są kopiowane dane źródłowe. JXL zapisywany jest jako czysty strumień bez kontenera z metadanymi.
- **Import**: stosuje obrót/odbicie i kolory przed usunięciem metadanych; JXL używa CMS libjxl, AVIF obsługuje ICC oraz sRGB/BT.709/Display P3 w SDR. Wyjście jest 8-bitowe. Nieobsługiwane profile, HDR i animacje są odrzucane z komunikatem, zamiast cichej utraty klatek lub obcięcia HDR.

Skrypt `bundle-codecs.py` kopiuje zależności kodeków do `Contents/Frameworks`, przepisuje ścieżki bibliotek na lokalne, podpisuje je i dodaje licencje do informacji o bibliotekach. `WEBDROP_BREW_PREFIX` pozwala wskazać inną instalację Homebrew podczas budowania i pakowania.

```sh
./scripts/cargo.sh run --release -- --batch --format avif --quality 75 foto.jpg
./scripts/cargo.sh run --release -- --batch --format jxl --quality 85 foto.jpg
./scripts/cargo.sh run --release -- --batch --format avif --avif-lossless --trim transparent logo.png
./scripts/cargo.sh run --release -- --batch --format jxl --jxl-lossless --trim background logo.ai
./scripts/cargo.sh run --release -- --batch photo.avif photo.jxl
```

Syntetyczne próbki orientacji, animacji i HDR w `tests/fixtures/modern-*` generuje `scripts/make-modern-fixtures.c`, linkowany do libavif i libjxl. Nie zawierają danych użytkownika.

### Tryb terminalowy

Komunikaty CLI, podsumowania i uwagi są zawsze po angielsku, niezależnie od języka systemu i GUI. Nazwy plików i ścieżki pozostają bez zmian.

Ten sam silnik jest dostępny bez GUI:

```sh
./scripts/cargo.sh run --release -- --batch foto.png foto.jpg
./scripts/cargo.sh run --release -- --batch --width 700 --quality 85 foto.jpg
./scripts/cargo.sh run --release -- --batch --format png --png-mode zopfli --width 0 grafika.png
./scripts/cargo.sh run --release -- --batch --webp-mode lossless --output /tmp/wyniki grafika.png
./scripts/cargo.sh run --release -- --batch --trim background --webp-mode lossless logo.ai
./scripts/cargo.sh run --release -- --batch --trim transparent --vector-dpi 300 --page 2 logo.pdf
./scripts/cargo.sh run --release -- --batch photo.heic scan.tiff
./scripts/cargo.sh run --release -- --batch --trim transparent logo.svg
```

Nieudany plik nie przerywa pozostałych zadań batch, ale powoduje niezerowy kod wyjścia. `--help` pokazuje wszystkie opcje.

Licencja projektu: GPL-3.0-or-later; zależności zachowują własne licencje (w tym libimagequant GPL-3.0-or-later).

## Budowanie Windows i Linux

Instrukcje kompilacji obu wersji na Apple Silicon znajdują się w [angielskim README](README.en.md#windows-and-linux-cross-builds). Skrypty pobierają zależności, sprawdzają ich SHA-256 i dołączają biblioteki oraz licencje do paczek. Wersje Windows/Linux wymagają jeszcze testów działania na docelowych systemach przed uznaniem za stabilne.

Archiwa w `dist` i plik `SHA256SUMS.txt` są przeznaczone do dodania do repozytorium. Wyjątki w `.gitignore` pozwalają je opublikować; rozpakowana aplikacja i pliki robocze pozostają ignorowane. Linki pobierania zadziałają na GitHubie po przesłaniu repozytorium razem z tymi plikami. Zachowujemy tylko bieżące paczki.
