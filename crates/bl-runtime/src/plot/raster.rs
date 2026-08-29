//! Raster for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

/// `#rrggbb` to premultiplied-ready RGBA, with the alpha add_circle applies.
///
/// The rastered and vector paths have to agree on colour as well as position,
/// or the same plot changes appearance when it crosses the raster threshold.
/// add_circle draws every point at opacity 0.7, so that is baked in here.
pub(crate) fn hex_to_rgba(hex: &str, alpha: f64) -> [u8; 4] {
    let digits = hex.strip_prefix('#').unwrap_or(hex);
    let channel = |at: usize| -> u8 {
        digits
            .get(at..at + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
            .unwrap_or(0)
    };
    if digits.len() < 6 {
        return [0, 0, 0, (alpha.clamp(0.0, 1.0) * 255.0) as u8];
    }
    [
        channel(0),
        channel(2),
        channel(4),
        (alpha.clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

/// The opacity `add_circle` writes into every point. The raster path applies it
/// as an alpha byte instead, so a plot looks the same either side of the
/// threshold.
pub(crate) const POINT_ALPHA: f64 = 0.7;

/// Vector circles are one DOM node each. They are ruinous in quantity: at
/// 200,000 points a scatter measures 13.8 MB and 200,012 elements, against
/// 892 KB and 12 for the same figure rasterised. The raster is flat in the
/// point count -- it is bounded by the pixels of the plot area, not the data --
/// so the crossover is a size, not a ratio.
///
/// Below the threshold vector wins on both size and speed, which is why this is
/// a threshold rather than a mode.
pub(crate) const DEFAULT_RASTER_THRESHOLD: usize = 20_000;

/// Whether a scatter draws as circles or as one image, and at what
/// supersampling.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RasterChoice {
    pub(crate) enabled: bool,
    pub(crate) scale: f64,
}

/// Read the shared `raster`, `raster_threshold` and `raster_scale` options.
///
/// Every scatter-like plot takes the same three, spelled the same way and
/// erroring the same way, so a caller who learns them on one plot knows them on
/// all of them. `builtin` only names the plot in those errors.
pub(crate) fn raster_choice(
    opts: &HashMap<String, Value>,
    builtin: &str,
    count: usize,
) -> Result<RasterChoice> {
    let threshold = match opts.get("raster_threshold") {
        None => DEFAULT_RASTER_THRESHOLD,
        Some(Value::Int(value)) if *value > 0 => *value as usize,
        Some(Value::Float(value)) if value.is_finite() && *value >= 1.0 => *value as usize,
        Some(_) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{builtin}() option 'raster_threshold' must be a positive number"),
                None,
            ))
        }
    };
    let enabled = match opts.get("raster") {
        None => count >= threshold,
        Some(Value::Bool(value)) => *value,
        Some(Value::Str(value)) => match value.to_ascii_lowercase().as_str() {
            "auto" => count >= threshold,
            "on" | "true" => true,
            "off" | "false" => false,
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("{builtin}() option 'raster' must be 'auto', 'on', 'off', or Bool"),
                    None,
                ))
            }
        },
        Some(_) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{builtin}() option 'raster' must be 'auto', 'on', 'off', or Bool"),
                None,
            ))
        }
    };
    let scale = match opts.get("raster_scale") {
        None => 2.0,
        Some(Value::Int(value)) if (1..=4).contains(value) => *value as f64,
        Some(Value::Float(value)) if value.is_finite() && (1.0..=4.0).contains(value) => *value,
        Some(_) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{builtin}() option 'raster_scale' must be between 1 and 4"),
                None,
            ))
        }
    };
    Ok(RasterChoice { enabled, scale })
}

/// Whether a plot was asked to thin, and at what pixel size.
pub(crate) fn thin_requested(opts: &HashMap<String, Value>, builtin: &str) -> Result<bool> {
    match opts.get("thin") {
        None => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(Value::Str(value)) => match value.to_ascii_lowercase().as_str() {
            "on" | "true" => Ok(true),
            "off" | "false" => Ok(false),
            _ => Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{builtin}() option 'thin' must be 'on', 'off', or Bool"),
                None,
            )),
        },
        Some(_) => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{builtin}() option 'thin' must be 'on', 'off', or Bool"),
            None,
        )),
    }
}

/// Keep at most one point per device pixel and drop the rest.
///
/// The contract is deliberately narrow, because this changes what a figure
/// shows and no figure should change quietly. A point is dropped only when
/// another point in the same set lands on the same device pixel, so coverage
/// is very nearly preserved and the extent of the cloud does not move in
/// either axis.
///
/// Very nearly, not exactly: a point is a disc, not a pixel. Two points can
/// share a cell and still have their discs reach a fraction of a pixel in
/// different directions, so an anti-aliased edge sliver can go unpainted.
/// Measured on a 60,000-variant Manhattan plot that was 54 pixels out of
/// 687,514 painted, or 0.008%, and no pixel was painted that would not have
/// been painted anyway.
///
/// What is genuinely lost is overdraw. Points are drawn at alpha 0.7, so a
/// saturated region normally accumulates towards opaque; painted once it
/// reads lighter. On that same figure the median painted pixel did not change
/// alpha at all and the 95th percentile moved by 3 of 255, but in the densest
/// pileups the change reached 138. Density stops being legible as shade, and
/// that is a real change to the figure -- which is why callers must opt in and
/// why they record the counts in the figure itself.
///
/// `rank` chooses the survivor within a cell: the largest value wins, so a
/// Manhattan plot passes -log10(p) and keeps the most significant variant in
/// every pixel -- the one a reader is looking for. Equal ranks resolve by
/// input order, so the result does not depend on hash iteration order.
///
/// Returns ascending indices into `points`, so relative draw order survives.
pub(crate) fn thin_to_pixel_grid(
    points: &[(f64, f64)],
    area: (f64, f64, f64, f64),
    scale: f64,
    rank: &[f64],
) -> Vec<usize> {
    use std::collections::hash_map::Entry;

    let (origin_x, origin_y, _, _) = area;
    let scale = scale.clamp(1.0, 4.0);
    let mut best: HashMap<(i64, i64), usize> = HashMap::with_capacity(points.len() / 4 + 1);
    for (index, &(px, py)) in points.iter().enumerate() {
        if !px.is_finite() || !py.is_finite() {
            continue;
        }
        let cell = (
            ((px - origin_x) * scale).floor() as i64,
            ((py - origin_y) * scale).floor() as i64,
        );
        match best.entry(cell) {
            Entry::Vacant(slot) => {
                slot.insert(index);
            }
            Entry::Occupied(mut slot) => {
                let held = *slot.get();
                let challenger = rank.get(index).copied().unwrap_or(0.0);
                let incumbent = rank.get(held).copied().unwrap_or(0.0);
                if challenger > incumbent {
                    slot.insert(index);
                }
            }
        }
    }
    let mut kept: Vec<usize> = best.into_values().collect();
    kept.sort_unstable();
    kept
}

/// Write a plot as a PNG: `save_png(svg, "figure.png", { scale: 2 })`.
///
/// Rasterises the SVG the plot builtins already return rather than rendering
/// twice. Everything here is downstream of the string `save_svg` would have
/// written, so a PNG cannot disagree with its SVG - and every existing plot got
/// PNG support the moment this landed, including the ones added after it.
///
/// `scale` multiplies the pixel dimensions without changing the drawing. The
/// equivalent `dpi` option uses the SVG/CSS baseline of 96 dpi, so `{dpi: 300}`
/// produces exactly 300/96 times the source pixels. Give one, not both.
///
/// The figure is the same size in inches and simply carries more pixels, which is
/// what a journal asking for 300 dpi wants. Default 2, because a 1x raster of a
/// 600-point figure looks soft on any modern display.
pub(super) fn builtin_save_png(args: Vec<Value>) -> Result<Value> {
    let svg = match &args[0] {
        Value::Str(s) => s.to_string(),
        Value::Record(_) => match builtin_render_plot(vec![args[0].clone()])? {
            Value::Str(svg) if svg.trim_start().starts_with("<svg") => svg.to_string(),
            other => {
                return Err(BioLangError::type_error(
                    format!(
                        "save_png() rendered PlotSpec as {}, expected SVG",
                        other.type_of()
                    ),
                    None,
                ))
            }
        },
        Value::Nil => {
            return Err(BioLangError::type_error(
                "save_png() received Nil — the plot function before the pipe likely failed or returned nothing".to_string(),
                None,
            ))
        }
        other => {
            return Err(BioLangError::type_error(
                format!("save_png() requires SVG Str or PlotSpec Record, got {}", other.type_of()),
                None,
            ))
        }
    };
    let path = match &args[1] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("save_png() requires Str (path), got {}", other.type_of()),
                None,
            ))
        }
    };
    // Options are the third argument here, not the second - args[1] is the path.
    // parse_options() always reads args[1], so calling it would have silently
    // ignored every option and left `scale` at its default.
    let opts = parse_options(&args[1..]);
    if opts.contains_key("scale") && opts.contains_key("dpi") {
        return Err(BioLangError::type_error(
            "save_png() accepts either scale or dpi, not both",
            None,
        ));
    }
    let scale = match opts.get("dpi") {
        Some(value) => {
            let dpi = value.as_float().unwrap_or(f64::NAN);
            if !dpi.is_finite() || dpi <= 0.0 {
                return Err(BioLangError::type_error(
                    format!("save_png() dpi must be a positive number, got {dpi}"),
                    None,
                ));
            }
            dpi / 96.0
        }
        None => get_opt_f64(&opts, "scale", 2.0),
    };
    if !(scale.is_finite() && scale > 0.0) {
        return Err(BioLangError::type_error(
            format!("save_png() scale must be a positive number, got {scale}"),
            None,
        ));
    }

    render_png(&svg, &path, scale)?;
    Ok(Value::Str(path))
}

#[cfg(feature = "native")]
pub(super) fn configure_generic_font_families(db: &mut resvg::usvg::fontdb::Database) {
    // fontdb's generic defaults are Windows font names (Arial, Times New Roman,
    // and Courier New). A Linux machine can therefore have plenty of fonts but
    // still render every `font-family="sans-serif"` label as nothing. Point the
    // CSS generic families at fonts that are actually present on this machine.
    let families: Vec<String> = db
        .faces()
        .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
        .collect();

    let choose = |preferred: &[&str]| {
        preferred
            .iter()
            .find_map(|candidate| {
                families
                    .iter()
                    .find(|name| name.eq_ignore_ascii_case(candidate))
                    .cloned()
            })
            .or_else(|| families.first().cloned())
    };

    if let Some(family) = choose(&[
        "Arial",
        "DejaVu Sans",
        "Liberation Sans",
        "Noto Sans",
        "Ubuntu",
        "Segoe UI",
        "Helvetica",
    ]) {
        db.set_sans_serif_family(family);
    }
    if let Some(family) = choose(&[
        "Times New Roman",
        "DejaVu Serif",
        "Liberation Serif",
        "Noto Serif",
        "Georgia",
    ]) {
        db.set_serif_family(family);
    }
    if let Some(family) = choose(&[
        "Courier New",
        "DejaVu Sans Mono",
        "Liberation Mono",
        "Noto Sans Mono",
        "Consolas",
    ]) {
        db.set_monospace_family(family);
    }
}

#[cfg(feature = "native")]
pub(super) fn svg_font_database() -> std::sync::Arc<resvg::usvg::fontdb::Database> {
    use resvg::usvg;

    // Both PNG export and terminal previews use this cache. Loading the system
    // font database for every plot makes a quick REPL preview feel needlessly
    // slow and can also make two render paths choose different fallback fonts.
    static FONTS: std::sync::OnceLock<std::sync::Arc<usvg::fontdb::Database>> =
        std::sync::OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            configure_generic_font_families(&mut db);
            std::sync::Arc::new(db)
        })
        .clone()
}

#[cfg(feature = "native")]
pub(super) fn render_png(svg: &str, path: &str, scale: f64) -> Result<()> {
    use resvg::{tiny_skia, usvg};

    let png_error = |message: String| BioLangError::runtime(ErrorKind::IOError, message, None);

    let options = usvg::Options {
        fontdb: svg_font_database(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(svg, &options)
        .map_err(|e| png_error(format!("save_png() could not parse the SVG: {e}")))?;

    let size = tree.size();
    let width = (size.width() as f64 * scale).round().max(1.0);
    let height = (size.height() as f64 * scale).round().max(1.0);
    // A scale of 1e6 on a 600-point figure asks for a 360-gigapixel buffer.
    // Pixmap::new returns None rather than aborting, so say why.
    let mut pixmap = tiny_skia::Pixmap::new(width as u32, height as u32).ok_or_else(|| {
        png_error(format!(
            "save_png() cannot allocate a {width:.0}x{height:.0} image — lower the scale"
        ))
    })?;

    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale as f32, scale as f32),
        &mut pixmap.as_mut(),
    );
    pixmap
        .save_png(path)
        .map_err(|e| png_error(format!("save_png() write failed: {e}")))?;
    Ok(())
}

#[cfg(not(feature = "native"))]
pub(super) fn render_png(_svg: &str, _path: &str, _scale: f64) -> Result<()> {
    // The WASM build has no rasteriser, but the browser does. Failing with the
    // alternative named beats failing with "unknown builtin".
    Err(BioLangError::runtime(
        ErrorKind::IOError,
        "save_png() is not available in this build — use save_svg(), which every browser and \
         image tool can rasterise"
            .to_string(),
        None,
    ))
}
