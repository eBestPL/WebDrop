use image_optimizer::optimizer::{OutputFormat, Settings, TrimMode, optimize};
use std::{fs, sync::atomic::AtomicBool};

fn run(source: &str, settings: Settings) -> (String, image_optimizer::optimizer::Outcome) {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("Logo.SVG");
    fs::write(&input, source).unwrap();
    let result = optimize(
        &input,
        &dir.path().join("out"),
        &settings,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(input).unwrap(),
        source,
        "source is immutable"
    );
    assert_eq!(result.path.extension().unwrap(), "svg");
    (fs::read_to_string(&result.path).unwrap(), result)
}

fn trimmed() -> Settings {
    Settings {
        trim: TrimMode::Transparent,
        ..Default::default()
    }
}

fn render(source: &str) -> resvg::tiny_skia::Pixmap {
    let tree = usvg::Tree::from_str(source, &usvg::Options::default()).unwrap();
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    pixmap
}

fn assert_render_crop(before: &str, after: &str, x: u32, y: u32) {
    let before = render(before);
    let after = render(after);
    for row in 0..after.height() {
        for col in 0..after.width() {
            assert_eq!(
                before.pixel(col + x, row + y),
                after.pixel(col, row),
                "pixel {col},{row}"
            );
        }
    }
}

#[test]
fn svg_is_always_svg_and_byte_identical_without_trim_for_every_output_format() {
    let source = "\u{feff}<?xml version=\"1.0\"?>\r\n<!-- Preserve this -->\r\n<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 2400 800'><metadata>© Logo &amp; Brand</metadata><text x='30' y='50'>Original font</text><rect width='100%' height='100%' fill='#ABCDEF'/></svg>\r\n";
    for format in [
        OutputFormat::Webp,
        OutputFormat::Original,
        OutputFormat::Png,
        OutputFormat::Jpeg,
    ] {
        let (result, outcome) = run(
            source,
            Settings {
                format,
                max_width: Some(500),
                quality: 1,
                ..Default::default()
            },
        );
        assert_eq!(result.as_bytes(), source.as_bytes());
        assert_eq!((outcome.width, outcome.height), (2400, 800));
    }
}

#[test]
fn trims_only_root_attributes_keeps_all_content_and_strokes() {
    let source = "<?xml version='1.0'?>\n<!-- Original -->\n<svg xmlns='http://www.w3.org/2000/svg' width='200' height='120' viewBox='0 0 200 120' data-note='a > b'>\n <metadata>© My logo</metadata>\n <g transform='translate(20 10)'><path id='brand' d='M30 20H130V70H30Z' fill='#6633cc' stroke='#123456' stroke-width='4'/></g>\n</svg>\n";
    let (result, outcome) = run(source, trimmed());
    let expected = source
        .replace("width='200'", "width='104'")
        .replace("height='120'", "height='54'")
        .replace("viewBox='0 0 200 120'", "viewBox='48 28 104 54'");
    assert_eq!(result, expected);
    assert_eq!((outcome.width, outcome.height), (104, 54));
    assert_render_crop(source, &result, 48, 28);
}

#[test]
fn preserves_css_gradients_clipping_and_definitions() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="120" viewBox="0 0 200 120">
<style>/* Illustrator */ .st0 {fill:url(#brand);stroke:#234567;stroke-width:4}</style>
<defs><linearGradient id="brand" x1="0%" x2="100%"><stop offset="0%" stop-color="#9f12e3"/><stop offset="100%" stop-color="#45c586"/></linearGradient><clipPath id="cut"><rect x="40" y="20" width="110" height="80"/></clipPath></defs>
<g clip-path="url(#cut)"><rect class="st0" x="50" y="30" width="100" height="50"/></g></svg>"##;
    let (result, _) = run(source, trimmed());
    assert_eq!(
        source.split_once('>').unwrap().1,
        result.split_once('>').unwrap().1
    );
    assert_render_crop(source, &result, 48, 28);
}

#[test]
fn handles_viewbox_offsets_scale_alignment_and_nonuniform_scale() {
    for aspect in ["xMidYMid meet", "xMaxYMin meet", "none", "xMidYMid slice"] {
        let source = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="200" viewBox="10 20 100 100" preserveAspectRatio="{aspect}"><rect x="30" y="50" width="40" height="20" fill="red"/></svg>"#
        );
        let (result, _) = run(&source, trimmed());
        let doc = roxmltree::Document::parse(&result).unwrap();
        let vb: svgtypes::ViewBox = doc
            .root_element()
            .attribute("viewBox")
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!((vb.x, vb.y, vb.w, vb.h), (30.0, 50.0, 40.0, 20.0));
        let offset = match aspect {
            "xMaxYMin meet" => (240, 60),
            "none" => (80, 60),
            "xMidYMid slice" => (80, 20),
            _ => (140, 60),
        };
        assert_render_crop(&source, &result, offset.0, offset.1);
    }
}

#[test]
fn adds_missing_viewport_attributes_and_handles_prefixed_root() {
    for source in [
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 100'><rect x='40' y='20' width='100' height='50'/></svg>",
        "<s:svg xmlns:s='http://www.w3.org/2000/svg' width='200' height='100'><s:rect x='40' y='20' width='100' height='50'/></s:svg>",
    ] {
        let (result, outcome) = run(source, trimmed());
        assert_eq!((outcome.width, outcome.height), (100, 50));
        let doc = roxmltree::Document::parse(&result).unwrap();
        assert_eq!(
            doc.root_element().attribute("viewBox"),
            Some("40 20 100 50")
        );
        assert_eq!(
            source.split_once('>').unwrap().1,
            result.split_once('>').unwrap().1
        );
        assert_render_crop(source, &result, 40, 20);
    }
}

#[test]
fn background_trim_does_not_remove_white_shapes_or_change_colors() {
    let source = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><rect width='100' height='100' fill='white'/><circle cx='50' cy='50' r='20' fill='red'/></svg>";
    let (result, _) = run(
        source,
        Settings {
            trim: TrimMode::Background,
            ..Default::default()
        },
    );
    assert_eq!(source, result);
}

#[test]
fn empty_and_already_cropped_svgs_remain_byte_identical() {
    for body in [
        "",
        "<rect width='100' height='100'/>",
        "<path d='M200 200H220V220Z'/>",
    ] {
        let source =
            format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>{body}</svg>");
        assert_eq!(run(&source, trimmed()).0, source);
    }
}

#[test]
fn unsafe_trim_refuses_output_instead_of_damaging_svg() {
    let bodies = [
        "<text x='10' y='20'>Brand</text>",
        "<rect width='100%' height='50'/>",
        "<rect width='10em' height='50'/>",
        "<image href='missing.png' width='20' height='20'/>",
        "<script>alert(1)</script>",
        "<animate attributeName='x' from='1' to='20'/>",
        "<style>@media (min-width:50px){.a{fill:red}}</style>",
        "<style>svg[width='100'] path{stroke-width:20}</style>",
        "<g style='transform:translate(10px,20px)'/>",
        "<rect width='10' height='10' fill='url(other.svg#g)'/>",
        "<filter id='blur'/>",
        "<path d='M0 0L50 50' vector-effect='non-scaling-stroke'/>",
        "<svg viewBox='0 0 10 10'/>",
        "<use href='other.svg#brand'/>",
    ];
    for body in bodies {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("logo.svg");
        let output = dir.path().join("out");
        let source = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><rect x='20' y='20' width='10' height='10'/>{body}</svg>"
        );
        fs::write(&input, &source).unwrap();
        let error = optimize(&input, &output, &trimmed(), &AtomicBool::new(false)).unwrap_err();
        assert!(format!("{error:#}").contains("bezpiecznie"), "{error:#}");
        assert!(!output.exists());
        assert_eq!(fs::read_to_string(&input).unwrap(), source);
        assert_eq!(run(&source, Settings::default()).0, source);
    }
}

#[test]
fn svg_collision_keeps_existing_download_and_cancellation_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("logo.svg");
    let source = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><rect x='30' y='20' width='10' height='40'/></svg>";
    fs::write(&input, source).unwrap();
    let result = optimize(&input, dir.path(), &trimmed(), &AtomicBool::new(false)).unwrap();
    assert_eq!(result.path.file_name().unwrap(), "logo (1).svg");
    assert_eq!(fs::read_to_string(&input).unwrap(), source);
    assert!(optimize(&input, dir.path(), &trimmed(), &AtomicBool::new(true)).is_err());
    assert!(!dir.path().join("logo (2).svg").exists());
}
