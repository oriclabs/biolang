//! A rasterised scatter has to be the same picture as a vector one.
//!
//! Above a few thousand cells umap_plot stops emitting one <circle> per point
//! and draws them all into one embedded PNG instead: a million cells was a
//! 65.5 MB string and 1,000,039 DOM nodes, and is now 37 KB and 39 elements.
//!
//! The danger in that is silent. A wrong origin or a wrong scale in the pixmap
//! mapping still produces a plausible-looking figure - points, in a blob,
//! inside the axes - just not where the data says they are. Nothing about the
//! output looks broken. So the central test here renders identical data both
//! ways and checks the ink lands where the circles did.

use bl_core::value::Value;
use bl_runtime::bio_plots::call_bio_plots_builtin;
use resvg::tiny_skia::Pixmap;
use std::collections::HashMap;

/// Points in two clusters far apart on x, so a collapsed or shifted mapping
/// cannot hide.
fn points(n: usize) -> Value {
    let mut rows = Vec::with_capacity(n);
    for i in 0..n {
        let right = i % 2 == 0;
        let jitter = (i % 50) as f64 * 0.02;
        let mut record = HashMap::new();
        record.insert(
            "x".to_string(),
            Value::Float(if right { 100.0 } else { 0.0 } + jitter),
        );
        record.insert("y".to_string(), Value::Float(jitter * 2.0));
        record.insert("cluster".to_string(), Value::Int(right as i64));
        rows.push(Value::Record(record.into()));
    }
    Value::List(rows.into())
}

fn render(data: Value, opts: Vec<(&str, Value)>) -> String {
    let mut map = HashMap::new();
    for (key, value) in opts {
        map.insert(key.to_string(), value);
    }
    let args = if map.is_empty() {
        vec![data]
    } else {
        vec![data, Value::Record(map.into())]
    };
    match call_bio_plots_builtin("umap_plot", args) {
        Ok(Value::Str(svg)) => svg,
        other => panic!("umap_plot returned {other:?}"),
    }
}

fn attribute(tag: &str, name: &str) -> Option<f64> {
    let at = tag.find(&format!("{name}=\""))? + name.len() + 2;
    let rest = &tag[at..];
    rest[..rest.find('"')?].parse().ok()
}

/// The x coordinate of every drawn data point.
///
/// Radius 3 is a cell; the legend draws its swatches at radius 4, and counting
/// those as data made a 400-point plot look like 402 and put the rightmost
/// "point" out in the legend column.
fn circle_xs(svg: &str) -> Vec<f64> {
    svg.split("<circle")
        .skip(1)
        .filter_map(|tag| {
            let tag = &tag[..tag.find("/>")?];
            if (attribute(tag, "r")? - 3.0).abs() > 0.01 {
                return None;
            }
            attribute(tag, "cx")
        })
        .collect()
}

/// The embedded raster: its placement in user space and its decoded pixels.
fn raster(svg: &str) -> (f64, f64, f64, f64, Pixmap) {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let tag = svg
        .split("<image")
        .nth(1)
        .expect("no <image> - the points were not rasterised");
    let tag = &tag[..tag.find("/>").expect("unterminated <image>")];
    let marker = "base64,";
    let start = tag.find(marker).expect("no base64 payload") + marker.len();
    let payload = &tag[start..tag[start..].find('"').unwrap() + start];
    let bytes = STANDARD.decode(payload).expect("payload is not base64");
    let pixmap = Pixmap::decode_png(&bytes).expect("payload is not a PNG");
    (
        attribute(tag, "x").expect("no x"),
        attribute(tag, "y").expect("no y"),
        attribute(tag, "width").expect("no width"),
        attribute(tag, "height").expect("no height"),
        pixmap,
    )
}

/// Every inked pixel's x, mapped back into the SVG's user coordinates.
fn ink_xs(svg: &str) -> Vec<f64> {
    let (x, _, width, _, pixmap) = raster(svg);
    let scale = width / pixmap.width() as f64;
    let mut out = Vec::new();
    for (index, pixel) in pixmap.pixels().iter().enumerate() {
        if pixel.alpha() > 0 {
            let column = (index % pixmap.width() as usize) as f64;
            out.push(x + column * scale);
        }
    }
    out
}

fn span(values: &[f64]) -> (f64, f64) {
    (
        values.iter().cloned().fold(f64::MAX, f64::min),
        values.iter().cloned().fold(f64::MIN, f64::max),
    )
}

#[test]
fn the_raster_puts_points_where_the_vector_path_does() {
    // The test this file exists for. Identical data, both modes: the ink has to
    // occupy the same span of the plot as the circles, or the figure is drawn
    // in the wrong place and looks fine while doing it.
    let vector = render(points(400), vec![("raster", Value::Bool(false))]);
    let rastered = render(points(400), vec![("raster", Value::Bool(true))]);

    let (circle_lo, circle_hi) = span(&circle_xs(&vector));
    let (ink_lo, ink_hi) = span(&ink_xs(&rastered));

    // A circle of radius 3 inks 3 points either side of its centre, and
    // anti-aliasing reaches a fraction beyond that.
    const RADIUS: f64 = 3.0;
    const SLACK: f64 = 2.0;
    assert!(
        (ink_lo - (circle_lo - RADIUS)).abs() < SLACK,
        "left edge: circles start at {circle_lo:.1}, ink starts at {ink_lo:.1}"
    );
    assert!(
        (ink_hi - (circle_hi + RADIUS)).abs() < SLACK,
        "right edge: circles end at {circle_hi:.1}, ink ends at {ink_hi:.1}"
    );
}

#[test]
fn two_clusters_stay_two_clusters() {
    // The span test alone passes for a mapping that smears points across the
    // whole range. The data is two tight groups at opposite ends, so the middle
    // of the plot must be empty.
    let svg = render(points(400), vec![("raster", Value::Bool(true))]);
    let (x, _, width, _, pixmap) = raster(&svg);
    let scale = width / pixmap.width() as f64;

    let mut inked_columns = vec![false; pixmap.width() as usize];
    for (index, pixel) in pixmap.pixels().iter().enumerate() {
        if pixel.alpha() > 0 {
            inked_columns[index % pixmap.width() as usize] = true;
        }
    }
    let (lo, hi) = span(&ink_xs(&svg));
    let middle = ((lo + hi) / 2.0 - x) / scale;
    let window = pixmap.width() as f64 * 0.15;
    let empty = ((middle - window) as usize..(middle + window) as usize)
        .filter(|&column| column < inked_columns.len())
        .all(|column| !inked_columns[column]);
    assert!(empty, "the gap between the two clusters was filled in");
}

#[test]
fn small_plots_are_untouched() {
    // Every existing figure is vector, and hover, selection and infinite zoom
    // come with that. Below the threshold nothing may change.
    let svg = render(points(400), vec![]);
    assert_eq!(circle_xs(&svg).len(), 400, "a small plot lost its circles");
    assert!(!svg.contains("<image"), "a small plot was rasterised");
}

#[test]
fn large_plots_cost_a_constant_number_of_elements() {
    let svg = render(points(20_000), vec![]);
    assert!(svg.contains("<image"), "a large plot was not rasterised");
    assert_eq!(circle_xs(&svg).len(), 0, "circles were drawn as well");
    assert!(
        svg.len() < 400_000,
        "20k cells produced {} KB",
        svg.len() / 1024
    );
}

#[test]
fn the_raster_option_overrides_the_threshold_either_way() {
    let forced_on = render(points(10), vec![("raster", Value::Bool(true))]);
    assert!(forced_on.contains("<image"), "raster: true was ignored");

    let forced_off = render(points(20_000), vec![("raster", Value::Bool(false))]);
    assert!(!forced_off.contains("<image"), "raster: false was ignored");
    assert_eq!(circle_xs(&forced_off).len(), 20_000);
}

#[test]
fn named_raster_modes_and_custom_threshold_are_explicit() {
    let forced_on = render(points(10), vec![("raster", Value::Str("on".into()))]);
    assert!(forced_on.contains("<image"));

    let forced_off = render(points(6_000), vec![("raster", Value::Str("off".into()))]);
    assert!(!forced_off.contains("<image"));

    let below_custom = render(
        points(6_000),
        vec![
            ("raster", Value::Str("auto".into())),
            ("raster_threshold", Value::Int(10_000)),
        ],
    );
    assert!(!below_custom.contains("<image"));

    let above_custom = render(
        points(400),
        vec![
            ("raster", Value::Str("auto".into())),
            ("raster_threshold", Value::Int(100)),
        ],
    );
    assert!(above_custom.contains("<image"));
}

#[test]
fn raster_scale_controls_pixel_density_without_moving_the_layer() {
    let scale_one = render(
        points(400),
        vec![
            ("raster", Value::Bool(true)),
            ("raster_scale", Value::Int(1)),
        ],
    );
    let scale_two = render(
        points(400),
        vec![
            ("raster", Value::Bool(true)),
            ("raster_scale", Value::Int(2)),
        ],
    );
    let (x1, y1, width1, height1, pixmap1) = raster(&scale_one);
    let (x2, y2, width2, height2, pixmap2) = raster(&scale_two);
    assert_eq!((x1, y1, width1, height1), (x2, y2, width2, height2));
    assert_eq!(pixmap2.width(), pixmap1.width() * 2);
    assert_eq!(pixmap2.height(), pixmap1.height() * 2);
}

#[test]
fn invalid_raster_options_fail_instead_of_silently_changing_the_plot() {
    for options in [
        vec![("raster", Value::Str("sometimes".into()))],
        vec![("raster_threshold", Value::Int(0))],
        vec![("raster_scale", Value::Int(8))],
    ] {
        let map: HashMap<String, Value> = options
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect();
        let args = vec![points(10), Value::Record(map.into())];
        assert!(call_bio_plots_builtin("umap_plot", args).is_err());
    }
}

#[test]
fn axes_and_labels_stay_vector() {
    // The whole point of rasterising only the points: text must not become
    // pixels, or the figure stops being publication quality.
    let svg = render(points(20_000), vec![]);
    assert!(svg.contains("<text"), "the axis labels were rasterised too");
    assert!(
        svg.contains("UMAP1") || svg.contains("<line"),
        "no axes drawn"
    );
}

#[test]
fn cluster_colours_survive_rasterising() {
    let svg = render(points(20_000), vec![]);
    let (_, _, _, _, pixmap) = raster(&svg);
    let mut seen = std::collections::HashSet::new();
    for pixel in pixmap.pixels() {
        if pixel.alpha() == 0 {
            continue;
        }
        // Stored premultiplied; the points are drawn at 0.7 alpha.
        let colour = pixel.demultiply();
        seen.insert((colour.red(), colour.green(), colour.blue()));
    }
    // Two clusters, so both palette colours have to be in there. Anti-aliased
    // edges add blends, hence >= rather than ==.
    assert!(
        seen.len() >= 2,
        "the raster holds {} colours - the clusters were flattened into one",
        seen.len()
    );
}

#[test]
fn save_png_still_renders_a_rasterised_plot() {
    // resvg has to understand the embedded data: URI. Without the raster-images
    // feature it parses the <image> and draws nothing, giving a PNG of empty
    // axes - which looks like a plot, just without any data in it.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.png");
    let svg = render(points(20_000), vec![]);
    bl_runtime::plot::call_plot_builtin(
        "save_png",
        vec![
            Value::Str(svg),
            Value::Str(path.to_string_lossy().to_string()),
        ],
    )
    .expect("save_png failed");

    let bytes = std::fs::read(&path).unwrap();
    let pixmap = Pixmap::decode_png(&bytes).unwrap();
    let coloured = pixmap
        .pixels()
        .iter()
        .filter(|pixel| {
            let c = pixel.demultiply();
            // Anything that is not the white background or black axis text.
            c.alpha() > 0 && (c.red() as i32 - c.blue() as i32).abs() > 20
        })
        .count();
    assert!(
        coloured > 1000,
        "only {coloured} coloured pixels - the embedded point layer was dropped"
    );
}
