//! SVG is never passed to an optimizer or serializer. usvg is used only to measure
//! geometry; edits are spliced into the original UTF-8 source at root attribute ranges.
use anyhow::{Context, Result, ensure};
use roxmltree::{Document, Node};
use std::ops::Range;

const NS: &str = "http://www.w3.org/2000/svg";
const HELP: &str =
    "Wyłącz Trim, aby zapisać SVG bez zmian. Tekst można wcześniej zamienić na krzywe w edytorze.";

pub struct SvgResult {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub note: String,
}

pub fn process(data: &[u8], trim: bool) -> Result<SvgResult> {
    let source = std::str::from_utf8(data).context("SVG musi być zapisany jako UTF-8")?;
    let doc = Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            allow_dtd: true, // roxmltree never fetches external DTDs or resources.
            nodes_limit: 100_000,
            ..Default::default()
        },
    )
    .context("Nieprawidłowy dokument SVG")?;
    let root = doc.root_element();
    ensure!(
        root.has_tag_name((NS, "svg")),
        "Plik nie zawiera dokumentu SVG"
    );
    let view_box = root
        .attribute("viewBox")
        .and_then(|v| v.parse::<svgtypes::ViewBox>().ok());
    let width = dimension(root.attribute("width"))
        .or(view_box.map(|v| v.w))
        .unwrap_or(0.0);
    let height = dimension(root.attribute("height"))
        .or(view_box.map(|v| v.h))
        .unwrap_or(0.0);
    let unchanged = |note: &str| SvgResult {
        data: data.to_vec(),
        width: width.ceil() as u32,
        height: height.ceil() as u32,
        note: note.into(),
    };
    if !trim {
        return Ok(unchanged(
            "SVG: kopia identyczna bajt w bajt; bez konwersji, skalowania i optymalizacji.",
        ));
    }
    validate_trim(&doc)
        .with_context(|| format!("Nie można bezpiecznie przyciąć tego SVG. {HELP}"))?;
    let options = usvg::Options {
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(source, &options).context("Nie można wyznaczyć granic SVG")?;
    if tree.root().children().is_empty() {
        return Ok(unchanged(
            "SVG: brak widocznej geometrii; plik pozostawiony bez zmian.",
        ));
    }
    let canvas_w = tree.size().width() as f64;
    let canvas_h = tree.size().height() as f64;
    let vb = view_box.unwrap_or(svgtypes::ViewBox::new(0.0, 0.0, canvas_w, canvas_h));
    let aspect = root
        .attribute("preserveAspectRatio")
        .unwrap_or("xMidYMid meet")
        .parse::<svgtypes::AspectRatio>()
        .context("Nieprawidłowe preserveAspectRatio")?;
    let (sx, sy, tx, ty) = viewport_transform(vb, canvas_w, canvas_h, aspect);
    let bbox = tree.root().abs_layer_bounding_box();
    // Round outwards to whole original viewport pixels. This retains antialiasing
    // at the edge and the original pixel phase instead of cutting through a stroke.
    let left = (bbox.x() as f64).floor().max(0.0);
    let top = (bbox.y() as f64).floor().max(0.0);
    let right = (bbox.right() as f64).ceil().min(canvas_w);
    let bottom = (bbox.bottom() as f64).ceil().min(canvas_h);
    if right <= left
        || bottom <= top
        || (left == 0.0 && top == 0.0 && right == canvas_w && bottom == canvas_h)
    {
        return Ok(unchanged(
            "SVG: brak pustych marginesów do usunięcia; plik pozostawiony bez zmian.",
        ));
    }
    let new_w = right - left;
    let new_h = bottom - top;
    let new_view_box = format!(
        "{} {} {} {}",
        number((left - tx) / sx),
        number((top - ty) / sy),
        number(new_w / sx),
        number(new_h / sy)
    );
    let output = patch_viewport(source, root, [new_view_box, number(new_w), number(new_h)])?;
    Ok(SvgResult {
        data: output.into_bytes(), width: new_w.ceil() as u32, height: new_h.ceil() as u32,
        note: "SVG trim: zmieniono tylko viewBox, width i height głównego elementu. Zawartość, kolory i metadane zachowane; bez optymalizacji i usuwania tła.".into(),
    })
}

fn number(value: f64) -> String {
    if value == 0.0 {
        "0".into()
    } else {
        value.to_string()
    }
}

fn dimension(value: Option<&str>) -> Option<f64> {
    use svgtypes::LengthUnit::*;
    let length = value?.parse::<svgtypes::Length>().ok()?;
    let factor = match length.unit {
        None | Px => 1.0,
        In => 96.0,
        Cm => 96.0 / 2.54,
        Mm => 96.0 / 25.4,
        Pt => 96.0 / 72.0,
        Pc => 16.0,
        _ => return Option::None,
    };
    (length.number.is_finite() && length.number > 0.0).then_some(length.number * factor)
}

fn viewport_transform(
    v: svgtypes::ViewBox,
    w: f64,
    h: f64,
    a: svgtypes::AspectRatio,
) -> (f64, f64, f64, f64) {
    use svgtypes::Align::*;
    let (mut sx, mut sy) = (w / v.w, h / v.h);
    if a.align != None {
        let s = if a.slice { sx.max(sy) } else { sx.min(sy) };
        sx = s;
        sy = s;
    }
    let dx = w - v.w * sx;
    let dy = h - v.h * sy;
    let ax = match a.align {
        XMidYMin | XMidYMid | XMidYMax => 0.5,
        XMaxYMin | XMaxYMid | XMaxYMax => 1.0,
        _ => 0.0,
    };
    let ay = match a.align {
        XMinYMid | XMidYMid | XMaxYMid => 0.5,
        XMinYMax | XMidYMax | XMaxYMax => 1.0,
        _ => 0.0,
    };
    (sx, sy, -v.x * sx + dx * ax, -v.y * sy + dy * ay)
}

fn patch_viewport(source: &str, root: Node<'_, '_>, values: [String; 3]) -> Result<String> {
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    let mut additions = String::new();
    for (name, value) in ["viewBox", "width", "height"].into_iter().zip(values) {
        if let Some(attr) = root.attribute_node(name) {
            let range = attr.range_value();
            // Guard against roxmltree's documented range limits and entity values.
            ensure!(
                source.get(range.clone()) == Some(attr.value()),
                "Nietypowy zapis atrybutu {name}; SVG nie został zmieniony"
            );
            edits.push((range, value));
        } else {
            additions.push_str(&format!(" {name}=\"{value}\""));
        }
    }
    // Insert immediately after the root's qualified name, never at a '>' inside a quoted attribute.
    if !additions.is_empty() {
        let start = root.range().start + 1;
        let end = start
            + source[start..]
                .find(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>')
                .context("Brak znacznika SVG")?;
        edits.push((end..end, additions));
    }
    edits.sort_by_key(|(r, _)| std::cmp::Reverse(r.start));
    let mut output = source.to_owned();
    for (range, value) in edits {
        output.replace_range(range, &value);
    }
    Ok(output)
}

/// Conservative static-vector profile. Refuse constructs whose bounds or geometry
/// could change with the viewport or depend on missing fonts/resources. Never
/// silently remove them, substitute fonts, or flatten them to a bitmap.
fn validate_trim(doc: &Document<'_>) -> Result<()> {
    let root = doc.root_element();
    ensure!(
        root.attribute("transform").is_none(),
        "Transformacja głównego elementu SVG"
    );
    ensure!(
        root.attribute("class").is_none(),
        "Klasa CSS głównego elementu SVG"
    );
    for node in doc.descendants() {
        ensure!(
            !node.is_pi() || node.pi().is_some_and(|p| p.target != "xml-stylesheet"),
            "Zewnętrzny arkusz CSS"
        );
        if !node.is_element()
            || node
                .ancestors()
                .skip(1)
                .any(|n| matches!(n.tag_name().name(), "metadata" | "title" | "desc"))
        {
            continue;
        }
        let tag = node.tag_name().name();
        ensure!(
            node.tag_name().namespace() == Some(NS),
            "Element spoza SVG: {tag}"
        );
        ensure!(
            matches!(
                tag,
                "svg"
                    | "g"
                    | "defs"
                    | "metadata"
                    | "title"
                    | "desc"
                    | "path"
                    | "rect"
                    | "circle"
                    | "ellipse"
                    | "line"
                    | "polyline"
                    | "polygon"
                    | "linearGradient"
                    | "radialGradient"
                    | "stop"
                    | "clipPath"
                    | "use"
                    | "style"
            ),
            "Element wymagający ręcznego trim: {tag}"
        );
        ensure!(tag != "svg" || node == root, "Zagnieżdżony obszar SVG");
        if tag == "style" {
            validate_stylesheet(node.text().unwrap_or(""))?;
        }
        for attr in node.attributes() {
            let name = attr.name();
            let value = attr.value().trim();
            ensure!(!name.starts_with("on"), "Interaktywne SVG");
            ensure!(
                !matches!(
                    name,
                    "vector-effect"
                        | "filter"
                        | "mask"
                        | "marker"
                        | "marker-start"
                        | "marker-mid"
                        | "marker-end"
                        | "requiredFeatures"
                        | "requiredExtensions"
                        | "systemLanguage"
                ),
                "Atrybut wymagający ręcznego trim: {name}"
            );
            if name == "overflow" {
                ensure!(value == "hidden", "Widoczna zawartość poza obszarem SVG");
            }
            if name == "href" {
                ensure!(value.starts_with('#'), "Zewnętrzne odwołanie SVG");
            }
            if name == "style" {
                validate_declarations(value)?;
            }
            if matches!(name, "fill" | "stroke") {
                svgtypes::Paint::from_str(value)
                    .context("Nieobsługiwany kolor lub zewnętrzna farba SVG")?;
            }
            if name == "clip-path" && value != "none" {
                svgtypes::FuncIRI::from_str(value).context("Nieobsługiwane przycięcie SVG")?;
            }
            if matches!(name, "transform" | "gradientTransform") {
                for transform in svgtypes::TransformListParser::from(value) {
                    transform.context("Nieprawidłowa transformacja SVG")?;
                }
            }
            if name == "d" {
                for segment in svgtypes::PathParser::from(value) {
                    segment.context("Nieprawidłowa ścieżka SVG")?;
                }
            }
            if matches!(
                name,
                "width"
                    | "height"
                    | "x"
                    | "y"
                    | "x1"
                    | "y1"
                    | "x2"
                    | "y2"
                    | "cx"
                    | "cy"
                    | "r"
                    | "rx"
                    | "ry"
                    | "fx"
                    | "fy"
                    | "fr"
                    | "stroke-width"
                    | "stroke-dasharray"
                    | "stroke-dashoffset"
            ) {
                let object_gradient = matches!(tag, "linearGradient" | "radialGradient")
                    && node
                        .attribute("gradientUnits")
                        .unwrap_or("objectBoundingBox")
                        == "objectBoundingBox"
                    && node.attribute("href").is_none()
                    && node
                        .attribute(("http://www.w3.org/1999/xlink", "href"))
                        .is_none();
                validate_length(value, object_gradient)?;
            }
        }
    }
    Ok(())
}

fn validate_length(value: &str, allow_percent: bool) -> Result<()> {
    // Absolute CSS units are stable. Font- and viewport-relative units are not.
    for item in value
        .split(|c: char| c.is_ascii_whitespace() || c == ',')
        .filter(|v| !v.is_empty() && *v != "none")
    {
        let length = item
            .parse::<svgtypes::Length>()
            .context("Nieobsługiwana długość SVG")?;
        ensure!(length.number.is_finite(), "Nieprawidłowa długość SVG");
        ensure!(
            matches!(
                length.unit,
                svgtypes::LengthUnit::None
                    | svgtypes::LengthUnit::Px
                    | svgtypes::LengthUnit::In
                    | svgtypes::LengthUnit::Cm
                    | svgtypes::LengthUnit::Mm
                    | svgtypes::LengthUnit::Pt
                    | svgtypes::LengthUnit::Pc
            ) || (allow_percent && length.unit == svgtypes::LengthUnit::Percent),
            "Wymiary względne mogłyby zmienić grafikę podczas trim"
        );
    }
    Ok(())
}

fn validate_stylesheet(mut css: &str) -> Result<()> {
    // Illustrator's simple .st0 { fill: ... } rules are supported. Responsive,
    // complex selectors and CSS affecting layout must be handled by an editor.
    while !css.trim().is_empty() {
        css = css.trim();
        if let Some(comment) = css.strip_prefix("/*") {
            css = &comment[comment.find("*/").context("Nieprawidłowy komentarz CSS")? + 2..];
            continue;
        }
        let (selectors, rest) = css.split_once('{').context("Nieobsługiwany CSS")?;
        for selector in selectors.split(',') {
            let class = selector
                .trim()
                .strip_prefix('.')
                .context("Trim obsługuje tylko proste selektory klas CSS")?;
            ensure!(
                !class.is_empty()
                    && class
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
                "Złożony selektor CSS"
            );
        }
        let (body, tail) = rest.split_once('}').context("Nieprawidłowy CSS")?;
        validate_declarations(body)?;
        css = tail;
    }
    Ok(())
}

fn validate_declarations(css: &str) -> Result<()> {
    for declaration in css.split(';').filter(|v| !v.trim().is_empty()) {
        let (name, value) = declaration
            .split_once(':')
            .context("Nieprawidłowy styl SVG")?;
        let name = name.trim();
        let value = value.trim();
        ensure!(
            matches!(
                name,
                "fill"
                    | "fill-rule"
                    | "fill-opacity"
                    | "stroke"
                    | "stroke-width"
                    | "stroke-linecap"
                    | "stroke-linejoin"
                    | "stroke-miterlimit"
                    | "stroke-dasharray"
                    | "stroke-dashoffset"
                    | "stroke-opacity"
                    | "opacity"
                    | "color"
                    | "stop-color"
                    | "stop-opacity"
                    | "clip-rule"
                    | "display"
                    | "visibility"
            ),
            "Styl wymagający ręcznego trim: {name}"
        );
        ensure!(
            !value.contains(['\\', '{', '}', '@', '!'])
                && !value.contains("var(")
                && !value.contains("env("),
            "Dynamiczny styl SVG"
        );
        if matches!(
            name,
            "stroke-width" | "stroke-dasharray" | "stroke-dashoffset"
        ) {
            validate_length(value, false)?;
        }
        if matches!(name, "fill" | "stroke") {
            svgtypes::Paint::from_str(value)
                .context("Nieobsługiwany kolor lub zewnętrzna farba CSS")?;
        }
    }
    Ok(())
}
