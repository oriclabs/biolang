//! save_png writes a picture of the plot, not just a file.
//!
//! The easy version of this test - call save_png, assert the file exists - is
//! passed by a rasteriser that renders a blank canvas, which is exactly what
//! happens when the SVG fails to parse and the error is swallowed, or when the
//! transform is wrong and the drawing lands outside the pixmap. So these tests
//! decode the PNG and look at the pixels: the plot's own colours have to be in
//! there, in roughly the place the SVG puts them.

use bl_core::value::Value;
use bl_runtime::plot::call_plot_builtin;
use resvg::tiny_skia::Pixmap;
use std::collections::HashMap;

fn options(pairs: Vec<(&str, Value)>) -> Value {
    let mut map = HashMap::new();
    for (key, value) in pairs {
        map.insert(key.to_string(), value);
    }
    Value::Record(map.into())
}

/// A bar chart of two very different values, so the figure has large blocks of
/// solid colour that are easy to find again in the raster.
fn a_plot() -> String {
    let values = Value::List(
        (0..40)
            .map(|i| Value::Float(if i % 2 == 0 { 10.0 } else { 90.0 }))
            .collect::<Vec<_>>()
            .into(),
    );
    match call_plot_builtin(
        "histogram",
        vec![
            values,
            options(vec![("title", Value::Str("PNG export".to_string()))]),
        ],
    ) {
        Ok(Value::Str(svg)) => svg,
        other => panic!("histogram returned {other:?}"),
    }
}

fn save(svg: &str, path: &std::path::Path, opts: Vec<(&str, Value)>) -> bl_core::error::Result<()> {
    let mut args = vec![
        Value::Str(svg.to_string()),
        Value::Str(path.to_string_lossy().to_string()),
    ];
    if !opts.is_empty() {
        args.push(options(opts));
    }
    call_plot_builtin("save_png", args).map(|_| ())
}

fn decode(path: &std::path::Path) -> Pixmap {
    let bytes = std::fs::read(path).expect("no PNG was written");
    assert_eq!(
        &bytes[..8],
        b"\x89PNG\r\n\x1a\n",
        "the file is not a PNG at all"
    );
    Pixmap::decode_png(&bytes).expect("the PNG could not be decoded")
}

/// Every distinct fully-opaque colour in the image, as #rrggbb.
fn colours(pixmap: &Pixmap) -> HashMap<String, usize> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for pixel in pixmap.pixels() {
        if pixel.alpha() == 0 {
            continue;
        }
        let key = format!(
            "#{:02x}{:02x}{:02x}",
            pixel.red(),
            pixel.green(),
            pixel.blue()
        );
        *seen.entry(key).or_insert(0) += 1;
    }
    seen
}

#[test]
fn the_figure_is_actually_drawn_not_just_a_blank_canvas() {
    // The test the whole file exists for. A rasteriser that parses nothing and
    // renders white would pass "the file was written"; it cannot pass this.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("figure.png");
    let svg = a_plot();
    save(&svg, &path, vec![]).expect("save_png failed");

    let pixmap = decode(&path);
    let counts = colours(&pixmap);
    let total: usize = counts.values().sum();
    let background = counts.values().cloned().max().unwrap_or(0);
    assert!(
        total - background > total / 100,
        "over 99% of the image is one colour - nothing was drawn"
    );

    // And the ink is the plot's own colour, so it is this figure that was
    // rasterised rather than some fallback.
    let bar = counts.get("#4e79a7").copied().unwrap_or(0);
    assert!(
        bar > 200,
        "the palette colour #4e79a7 covers only {bar} pixels - the bars are missing"
    );
}

#[test]
fn scale_multiplies_the_pixels_and_leaves_the_drawing_alone() {
    let dir = tempfile::tempdir().unwrap();
    let svg = a_plot();

    let small = dir.path().join("small.png");
    let large = dir.path().join("large.png");
    save(&svg, &small, vec![("scale", Value::Float(1.0))]).unwrap();
    save(&svg, &large, vec![("scale", Value::Float(3.0))]).unwrap();

    let small = decode(&small);
    let large = decode(&large);
    assert_eq!(large.width(), small.width() * 3, "width did not scale");
    assert_eq!(large.height(), small.height() * 3, "height did not scale");

    // Same figure, more pixels: the share of the image covered by the bars has
    // to stay put. If `scale` were resizing the drawing instead of the raster,
    // this fraction would move.
    let share = |pixmap: &Pixmap| {
        let counts = colours(pixmap);
        let total: usize = counts.values().sum();
        counts.get("#4e79a7").copied().unwrap_or(0) as f64 / total as f64
    };
    let (a, b) = (share(&small), share(&large));
    assert!(
        (a - b).abs() < 0.02,
        "bars cover {:.1}% at 1x but {:.1}% at 3x - scale is resizing the drawing",
        a * 100.0,
        b * 100.0
    );
}

#[test]
fn the_default_scale_is_a_sharp_one() {
    let dir = tempfile::tempdir().unwrap();
    let svg = a_plot();
    let default = dir.path().join("default.png");
    let two = dir.path().join("two.png");
    save(&svg, &default, vec![]).unwrap();
    save(&svg, &two, vec![("scale", Value::Float(2.0))]).unwrap();
    assert_eq!(decode(&default).width(), decode(&two).width());
}

#[test]
fn text_in_the_figure_survives_rasterising() {
    // resvg draws no text when the machine has no fonts, which is out of this
    // crate's hands - so the assertion runs only where a font exists.
    let mut fonts = resvg::usvg::fontdb::Database::new();
    fonts.load_system_fonts();
    if fonts.is_empty() {
        eprintln!("no system fonts: skipping the text assertion");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let with_title = dir.path().join("titled.png");
    let plain = dir.path().join("plain.png");
    save(&a_plot(), &with_title, vec![]).unwrap();

    // The same figure with an empty title: the only difference is the glyphs.
    let untitled = a_plot().replace("PNG export", "");
    save(&untitled, &plain, vec![]).unwrap();

    let ink = |path: &std::path::Path| {
        decode(path)
            .pixels()
            .iter()
            .filter(|p| p.alpha() > 0 && p.red() < 100 && p.green() < 100 && p.blue() < 100)
            .count()
    };
    assert!(
        ink(&with_title) > ink(&plain),
        "the title drew no pixels - text is being dropped"
    );
}

#[test]
fn a_failed_plot_is_reported_rather_than_written() {
    // save_png(some_plot_that_returned_nothing, ...) is the common mistake, and
    // an empty file on disk is the worst possible answer to it.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nil.png");
    let error = call_plot_builtin(
        "save_png",
        vec![Value::Nil, Value::Str(path.to_string_lossy().to_string())],
    )
    .unwrap_err();
    assert!(
        format!("{error}").contains("Nil"),
        "unhelpful error: {error}"
    );
    assert!(!path.exists(), "a file was written for a failed plot");
}

#[test]
fn nonsense_input_does_not_reach_the_rasteriser() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.png");
    let text = path.to_string_lossy().to_string();

    assert!(save("this is not an SVG", &path, vec![]).is_err());
    for scale in [0.0, -1.0, f64::NAN] {
        assert!(
            call_plot_builtin(
                "save_png",
                vec![
                    Value::Str(a_plot()),
                    Value::Str(text.clone()),
                    options(vec![("scale", Value::Float(scale))]),
                ],
            )
            .is_err(),
            "scale {scale} was accepted"
        );
    }
}
