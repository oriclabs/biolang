//! Canvas for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

pub(crate) struct Scale {
    pub(crate) domain: (f64, f64),
    pub(crate) range: (f64, f64),
}

impl Scale {
    pub(crate) fn map(&self, v: f64) -> f64 {
        if (self.domain.1 - self.domain.0).abs() < f64::EPSILON {
            return (self.range.0 + self.range.1) / 2.0;
        }
        let t = (v - self.domain.0) / (self.domain.1 - self.domain.0);
        self.range.0 + t * (self.range.1 - self.range.0)
    }

    pub(crate) fn nice_ticks(&self, count: usize) -> Vec<f64> {
        if count == 0 || !self.domain.0.is_finite() || !self.domain.1.is_finite() {
            return Vec::new();
        }
        let reversed = self.domain.0 > self.domain.1;
        let lo = self.domain.0.min(self.domain.1);
        let hi = self.domain.0.max(self.domain.1);
        let span = hi - lo;
        if span <= f64::EPSILON {
            return vec![lo];
        }

        // Human-readable 1/2/5 × 10^k spacing, bounded to the data domain.
        // This avoids labels such as 1.4, 2.8, 4.2 on a 0..7 axis while not
        // moving marks or expanding the plotting domain.
        let raw_step = span / count as f64;
        let magnitude = 10.0_f64.powf(raw_step.log10().floor());
        let fraction = raw_step / magnitude;
        let nice_fraction = if fraction <= 1.0 {
            1.0
        } else if fraction <= 2.0 {
            2.0
        } else if fraction <= 5.0 {
            5.0
        } else {
            10.0
        };
        let step = nice_fraction * magnitude;
        let first = (lo / step).ceil() * step;
        let last = (hi / step).floor() * step;
        let mut ticks = Vec::new();
        let mut tick = first;
        while tick <= last + step * 1e-10 && ticks.len() <= count.saturating_mul(2) + 2 {
            ticks.push(if tick.abs() < step * 1e-12 { 0.0 } else { tick });
            tick += step;
        }
        if ticks.is_empty() {
            ticks.extend([lo, hi]);
        }
        if reversed {
            ticks.reverse();
        }
        ticks
    }
}

/// Advance widths for the Arial / Helvetica / Liberation Sans stack, in units
/// per 1000 em, for ASCII 32 through 126.
///
/// Those three faces are metric-compatible by design and are exactly the stack
/// every BioLang plot names, so these numbers are the real ones for almost
/// every viewer rather than an approximation of them.
const ADVANCE_PER_MILLE: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

/// Deterministic text width, used for margins and legends.
///
/// SVG leaves final shaping to the viewer, but a layout engine needs a width
/// before a browser exists. A table of real advance widths is both exact for
/// the named fonts and stable across the CLI, WASM, and tests, where the
/// character-class estimate this replaces ran up to 16% wide - enough to
/// visibly inflate margins and legend boxes.
pub(crate) fn estimate_text_width(text: &str, size: f64) -> f64 {
    let per_mille: u32 = text
        .chars()
        .map(|character| {
            let code = character as u32;
            if (32..127).contains(&code) {
                u32::from(ADVANCE_PER_MILLE[(code - 32) as usize])
            } else if character.is_control() {
                0
            } else {
                // Outside the table. Non-Latin glyphs are usually wider than
                // the Latin mean, so err on the generous side: a slightly wide
                // margin is survivable, a clipped label is not.
                820
            }
        })
        .sum();
    f64::from(per_mille) / 1000.0 * size
}

pub(crate) struct SvgCanvas {
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) margin: Margin,
    pub(crate) elements: Vec<String>,
    pub(crate) theme: PlotTheme,
    accessible_label: Option<String>,
    accessible_description: Option<String>,
}

pub(crate) struct Margin {
    pub(crate) top: f64,
    pub(crate) right: f64,
    pub(crate) bottom: f64,
    pub(crate) left: f64,
}

impl Default for Margin {
    fn default() -> Self {
        Self {
            top: 40.0,
            right: 20.0,
            bottom: 50.0,
            left: 60.0,
        }
    }
}

/// Decimal places that keep every tick label on one axis distinct.
///
/// A fixed one-decimal format collides as soon as the tick step falls below
/// 0.1 — a 0..0.4 axis draws "0.1" twice, which reads as a rendering fault —
/// and it wastes a decimal on integer axes ("10.0" for a component number).
/// The cheapest rule that fixes both is to ask for the fewest decimals at
/// which no two labels are equal, since that is the property a reader needs.
pub(crate) fn tick_decimals(ticks: &[f64]) -> usize {
    const MAX_DECIMALS: usize = 4;
    for decimals in 0..MAX_DECIMALS {
        let mut labels: Vec<String> = ticks.iter().map(|t| format!("{t:.decimals$}")).collect();
        labels.sort();
        labels.dedup();
        if labels.len() == ticks.len() {
            return decimals;
        }
    }
    MAX_DECIMALS
}

impl SvgCanvas {
    pub(crate) fn new(width: f64, height: f64) -> Self {
        Self::with_theme(width, height, PlotTheme::from_name("biolang"))
    }

    pub(crate) fn with_theme(width: f64, height: f64, theme: PlotTheme) -> Self {
        Self {
            width,
            height,
            margin: Margin::default(),
            elements: Vec::new(),
            theme,
            accessible_label: None,
            accessible_description: None,
        }
    }

    pub(crate) fn plot_width(&self) -> f64 {
        self.width - self.margin.left - self.margin.right
    }
    pub(crate) fn plot_height(&self) -> f64 {
        self.height - self.margin.top - self.margin.bottom
    }

    pub(crate) fn add_rect(&mut self, x: f64, y: f64, w: f64, h: f64, fill: &str) {
        self.elements.push(format!(
            r#"<rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" fill="{fill}" />"#
        ));
    }

    /// `add_rect` with an outline, as ggplot2's `geom_boxplot()` draws its box.
    pub(crate) fn add_stroked_rect(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        fill: &str,
        stroke: &str,
        stroke_width: f64,
    ) {
        self.elements.push(format!(
            r#"<rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" fill="{fill}" stroke="{stroke}" stroke-width="{stroke_width:.2}" />"#
        ));
    }

    pub(crate) fn add_circle(&mut self, cx: f64, cy: f64, r: f64, fill: &str) {
        self.add_circle_with_opacity(cx, cy, r, fill, 0.7);
    }

    /// `add_circle` with the point opacity chosen by the caller.
    ///
    /// ggplot2's `geom_point()` is opaque unless `alpha` is set, so plots that
    /// reproduce an R figure need to override the 0.7 default.
    pub(crate) fn add_circle_with_opacity(
        &mut self,
        cx: f64,
        cy: f64,
        r: f64,
        fill: &str,
        opacity: f64,
    ) {
        self.elements.push(format!(
            r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="{fill}" opacity="{opacity:.2}" />"#
        ));
    }

    pub(crate) fn add_stroked_circle(
        &mut self,
        cx: f64,
        cy: f64,
        r: f64,
        fill: &str,
        stroke: &str,
        stroke_width: f64,
    ) {
        self.elements.push(format!(
            r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="{fill}" stroke="{stroke}" stroke-width="{stroke_width:.2}" />"#
        ));
    }

    pub(crate) fn add_line(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        stroke: &str,
        width: f64,
    ) {
        self.elements.push(format!(
            r#"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="{stroke}" stroke-width="{width}" />"#
        ));
    }

    /// A line with a regular dash pattern, used by R-style whiskers and
    /// reference guides. Keeping it here preserves the shared SVG escaping
    /// and makes the appearance consistent across native and browser output.
    pub(crate) fn add_dashed_line(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        stroke: &str,
        width: f64,
        dash: f64,
    ) {
        self.elements.push(format!(
            r#"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="{stroke}" stroke-width="{width}" stroke-dasharray="{dash:.1},{dash:.1}" />"#
        ));
    }

    /// A line with independently controlled dash and gap lengths. R's
    /// numbered line types use unequal patterns (notably `linetype = 3`,
    /// whose one-to-three dotted rhythm is common in ggtree figures).
    pub(crate) fn add_patterned_line(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        stroke: &str,
        width: f64,
        dash: f64,
        gap: f64,
    ) {
        self.elements.push(format!(
            r#"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="{stroke}" stroke-width="{width}" stroke-dasharray="{dash:.1},{gap:.1}" />"#
        ));
    }

    /// Add a connected, unfilled path. Keeping this primitive on the shared
    /// canvas lets forecast and diagnostic plots use the same escaping,
    /// sizing, theme, and export path as the established plot families.
    pub(crate) fn add_polyline(&mut self, points: &[(f64, f64)], colour: &str, width: f64) {
        if points.len() < 2 {
            return;
        }
        let points = points
            .iter()
            .map(|(x, y)| format!("{x:.2},{y:.2}"))
            .collect::<Vec<_>>()
            .join(" ");
        self.elements.push(format!(
            r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="{:.2}" stroke-linejoin="round" stroke-linecap="round"/>"#,
            points,
            colour,
            width
        ));
    }

    /// Add a filled polygon with explicit opacity, used for confidence bands.
    pub(crate) fn add_polygon_with_opacity(
        &mut self,
        points: &[(f64, f64)],
        colour: &str,
        opacity: f64,
    ) {
        if points.len() < 3 {
            return;
        }
        let points = points
            .iter()
            .map(|(x, y)| format!("{x:.2},{y:.2}"))
            .collect::<Vec<_>>()
            .join(" ");
        self.elements.push(format!(
            r#"<polygon points="{}" fill="{}" fill-opacity="{:.3}" stroke="none"/>"#,
            points,
            colour,
            opacity.clamp(0.0, 1.0)
        ));
    }

    /// Draw a cloud of points as one embedded raster instead of one element
    /// each.
    ///
    /// A scatter of n cells costs n DOM nodes and roughly 65 bytes of markup
    /// apiece. At the 2700 cells of PBMC3k that is nothing; at the hundreds of
    /// thousands a current atlas holds it is tens of megabytes of string and a
    /// browser that stops responding. Measured: one million points is 65.5 MB
    /// and 1,000,039 elements.
    ///
    /// Rasterising the points bounds both by the pixel area rather than the
    /// cell count, while the axes, ticks, labels and legend stay real SVG - so
    /// the text is still vector and still crisp at any zoom. This is what
    /// ggrastr and Seurat's `raster = TRUE` do, for the same reason.
    ///
    /// `points` are in the same user-space coordinates as every other element,
    /// so a caller can switch between this and add_circle without moving
    /// anything; the mapping into the pixmap happens here.
    pub(crate) fn add_point_raster(
        &mut self,
        points: &[(f64, f64, [u8; 4])],
        radius: f64,
        area: (f64, f64, f64, f64),
        raster_scale: f64,
    ) {
        use base64::{engine::general_purpose::STANDARD, Engine};

        let (x, y, width, height) = area;
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        // Supersampled, so a 3-point dot does not turn into a hard square and
        // the raster survives being viewed at 2x.
        let scale = raster_scale.clamp(1.0, 4.0);
        let pixel_width = (width * scale).ceil().max(1.0) as u32;
        let pixel_height = (height * scale).ceil().max(1.0) as u32;
        let Some(mut pixmap) = tiny_skia::Pixmap::new(pixel_width, pixel_height) else {
            return;
        };

        let mut paint = tiny_skia::Paint {
            anti_alias: true,
            ..Default::default()
        };
        for &(px, py, [r, g, b, a]) in points {
            // Into the pixmap's own coordinates: the raster covers the plot
            // area only, so subtract its origin.
            let cx = ((px - x) * scale) as f32;
            let cy = ((py - y) * scale) as f32;
            let Some(circle) = tiny_skia::PathBuilder::from_circle(cx, cy, (radius * scale) as f32)
            else {
                continue;
            };
            paint.set_color(tiny_skia::Color::from_rgba8(r, g, b, a));
            pixmap.fill_path(
                &circle,
                &paint,
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );
        }

        let Ok(png) = pixmap.encode_png() else {
            return;
        };
        // `href` rather than `xlink:href`: SVG 2, understood by every current
        // browser and by resvg, which is what save_png rasterises with.
        self.elements.push(format!(
            r#"<image x="{x:.1}" y="{y:.1}" width="{width:.1}" height="{height:.1}" href="data:image/png;base64,{}" />"#,
            STANDARD.encode(png)
        ));
    }

    /// Draw a scatter as vector circles or as one raster image.
    ///
    /// The choice is the caller's; this exists so every plot makes it the same
    /// way and so the two paths cannot drift apart in appearance. Colours stay
    /// strings on the vector path -- converting them to bytes and back would
    /// silently blacken any named colour, since `hex_to_rgba` only understands
    /// hex.
    pub(crate) fn add_scatter<S: AsRef<str>>(
        &mut self,
        points: &[(f64, f64, S)],
        radius: f64,
        area: (f64, f64, f64, f64),
        choice: RasterChoice,
    ) {
        if choice.enabled {
            let dots: Vec<(f64, f64, [u8; 4])> = points
                .iter()
                .map(|(x, y, fill)| (*x, *y, hex_to_rgba(fill.as_ref(), POINT_ALPHA)))
                .collect();
            self.add_point_raster(&dots, radius, area, choice.scale);
            return;
        }
        for (x, y, fill) in points {
            self.add_circle(*x, *y, radius, fill.as_ref());
        }
    }

    /// The rectangle a scatter's points occupy, for `add_scatter`.
    pub(crate) fn point_area(&self) -> (f64, f64, f64, f64) {
        (
            self.margin.left,
            self.margin.top,
            self.plot_width(),
            self.plot_height(),
        )
    }

    pub(crate) fn add_text(&mut self, x: f64, y: f64, text: &str, anchor: &str, size: f64) {
        self.add_text_styled(x, y, text, anchor, size, "normal", self.theme.text_colour);
    }

    pub(crate) fn add_text_styled(
        &mut self,
        x: f64,
        y: f64,
        text: &str,
        anchor: &str,
        size: f64,
        weight: &str,
        fill: &str,
    ) {
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        self.elements.push(format!(
            r#"<text x="{x:.1}" y="{y:.1}" text-anchor="{anchor}" font-size="{size}" font-family="{}" font-weight="{weight}" fill="{fill}">{escaped}</text>"#,
            self.theme.font_family
        ));
    }

    pub(crate) fn add_axis_title(
        &mut self,
        x: f64,
        y: f64,
        text: &str,
        axis: &str,
        angle: Option<f64>,
    ) {
        if text.is_empty() {
            return;
        }
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        let transform = angle
            .map(|angle| format!(r#" transform="rotate({angle},{x:.1},{y:.1})""#))
            .unwrap_or_default();
        self.elements.push(format!(
            r#"<text x="{x:.1}" y="{y:.1}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" data-biolang-axis-title="{axis}"{transform}>{escaped}</text>"#,
            self.theme.axis_title_size, self.theme.font_family, self.theme.text_colour
        ));
    }

    pub(crate) fn add_text_rotated(
        &mut self,
        x: f64,
        y: f64,
        text: &str,
        angle: f64,
        anchor: &str,
        size: f64,
    ) {
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        self.elements.push(format!(
            r#"<text x="{x:.1}" y="{y:.1}" text-anchor="{anchor}" font-size="{size}" font-family="{}" fill="{}" transform="rotate({angle},{x:.1},{y:.1})">{escaped}</text>"#,
            self.theme.font_family,
            self.theme.text_colour
        ));
    }

    /// Fit the panel around its actual labels rather than relying on one set of
    /// margins for every data range and figure size.
    pub(crate) fn fit_cartesian_layout(
        &mut self,
        x_ticks: &[f64],
        y_ticks: &[f64],
        x_label: &str,
        y_label: &str,
        title: &str,
        subtitle: &str,
        caption: &str,
        right_reserve: f64,
    ) {
        if !self.theme.is_adaptive() {
            return;
        }
        let x_decimals = tick_decimals(x_ticks);
        let y_decimals = tick_decimals(y_ticks);
        let widest_y = y_ticks
            .iter()
            .map(|tick| estimate_text_width(&format!("{tick:.y_decimals$}"), self.theme.tick_size))
            .fold(0.0, f64::max);
        let widest_x_half = x_ticks
            .iter()
            .map(|tick| {
                estimate_text_width(&format!("{tick:.x_decimals$}"), self.theme.tick_size) / 2.0
            })
            .fold(0.0, f64::max);

        self.margin.left = (widest_y + if y_label.is_empty() { 24.0 } else { 42.0 })
            .max(46.0)
            .min(self.width * 0.32);
        self.margin.right = (18.0 + right_reserve + widest_x_half * 0.25)
            .max(20.0)
            .min(self.width * 0.42);
        self.margin.top = if title.is_empty() {
            22.0
        } else if subtitle.is_empty() {
            48.0
        } else {
            66.0
        };
        self.margin.bottom = 24.0
            + if x_label.is_empty() { 10.0 } else { 24.0 }
            + if caption.is_empty() { 0.0 } else { 20.0 };
    }

    /// Draw publication-theme grid lines before marks are added.
    pub(crate) fn draw_cartesian_grid(&mut self, x_scale: &Scale, y_scale: &Scale) {
        self.draw_cartesian_grid_with_ticks(
            x_scale,
            y_scale,
            &x_scale.nice_ticks(5),
            &y_scale.nice_ticks(5),
        );
    }

    /// Draw a cartesian panel using caller-supplied tick positions.
    ///
    /// Positions are expressed in the scales' coordinate systems. This is
    /// useful for transformed axes: a raw value of 1,000 is placed at 3 on a
    /// base-10 scale while its label can remain "1000" or "1e+03".
    pub(crate) fn draw_cartesian_grid_with_ticks(
        &mut self,
        x_scale: &Scale,
        y_scale: &Scale,
        x_ticks: &[f64],
        y_ticks: &[f64],
    ) {
        if self.theme.grid_width <= 0.0 {
            return;
        }
        let left = self.margin.left;
        let right = left + self.plot_width();
        let top = self.margin.top;
        let bottom = top + self.plot_height();
        self.add_rect(
            left,
            top,
            self.plot_width(),
            self.plot_height(),
            self.theme.panel_colour,
        );
        let mapped_x = Scale {
            domain: x_scale.domain,
            range: (left, right),
        };
        for tick in x_ticks {
            let x = mapped_x.map(*tick);
            self.add_line(
                x,
                top,
                x,
                bottom,
                self.theme.grid_colour,
                self.theme.grid_width,
            );
        }
        let mapped_y = Scale {
            domain: y_scale.domain,
            range: (bottom, top),
        };
        for tick in y_ticks {
            let y = mapped_y.map(*tick);
            self.add_line(
                left,
                y,
                right,
                y,
                self.theme.grid_colour,
                self.theme.grid_width,
            );
        }
    }

    /// The panel and its horizontal gridlines only.
    ///
    /// A categorical x axis has no numeric ticks to rule against, so a plot
    /// with groups along the bottom still needs the themed panel that
    /// `draw_cartesian_grid` would otherwise supply.
    pub(crate) fn draw_categorical_grid(&mut self, y_scale: &Scale) {
        self.draw_categorical_grid_for_groups(y_scale, 0);
    }

    /// Draw a themed categorical panel, including ggplot2's vertical major
    /// grid line through each category centre.
    pub(crate) fn draw_categorical_grid_for_groups(&mut self, y_scale: &Scale, group_count: usize) {
        self.draw_categorical_grid_with_ticks(y_scale, group_count, &y_scale.nice_ticks(5));
    }

    pub(crate) fn draw_categorical_grid_with_ticks(
        &mut self,
        y_scale: &Scale,
        group_count: usize,
        y_ticks: &[f64],
    ) {
        if self.theme.grid_width <= 0.0 {
            return;
        }
        let left = self.margin.left;
        let right = left + self.plot_width();
        let top = self.margin.top;
        let bottom = top + self.plot_height();
        self.add_rect(
            left,
            top,
            self.plot_width(),
            self.plot_height(),
            self.theme.panel_colour,
        );
        if group_count > 0 {
            let step = self.plot_width() / group_count as f64;
            for index in 0..group_count {
                let x = left + step * (index as f64 + 0.5);
                self.add_line(
                    x,
                    top,
                    x,
                    bottom,
                    self.theme.grid_colour,
                    self.theme.grid_width,
                );
            }
        }
        let mapped_y = Scale {
            domain: y_scale.domain,
            range: (bottom, top),
        };
        for tick in y_ticks {
            let y = mapped_y.map(*tick);
            self.add_line(
                left,
                y,
                right,
                y,
                self.theme.grid_colour,
                self.theme.grid_width,
            );
        }
    }

    /// Outline the complete plotting panel, as base R does by default.
    pub(crate) fn draw_panel_border(&mut self) {
        self.add_stroked_rect(
            self.margin.left,
            self.margin.top,
            self.plot_width(),
            self.plot_height(),
            "none",
            self.theme.axis_colour,
            self.theme.axis_width,
        );
    }

    pub(crate) fn draw_x_axis(&mut self, scale: &Scale, label: &str) {
        self.draw_x_axis_with_tick_domain(scale, scale.domain, label);
    }

    /// Draw an x axis at explicit positions with explicit labels.
    pub(crate) fn draw_x_axis_with_ticks(
        &mut self,
        scale: &Scale,
        ticks: &[(f64, String)],
        label: &str,
    ) {
        let y = self.margin.top + self.plot_height();
        self.add_line(
            self.margin.left,
            y,
            self.margin.left + self.plot_width(),
            y,
            self.theme.axis_colour,
            self.theme.axis_width,
        );
        let x_scale = Scale {
            domain: scale.domain,
            range: (self.margin.left, self.margin.left + self.plot_width()),
        };
        for (tick, text) in ticks {
            let x = x_scale.map(*tick);
            self.add_line(
                x,
                y,
                x,
                y + 5.0,
                self.theme.axis_colour,
                self.theme.axis_width,
            );
            self.add_text(x, y + 18.0, text, "middle", self.theme.tick_size);
        }
        self.add_axis_title(
            self.margin.left + self.plot_width() / 2.0,
            if self.theme.is_adaptive() {
                y + 36.0
            } else {
                self.height - 5.0
            },
            label,
            "x",
            None,
        );
    }

    /// Draw an axis using `scale` for pixel placement while choosing labels
    /// from `tick_domain`. Plot expansion can then leave breathing room around
    /// the data without inventing out-of-range tick labels.
    pub(crate) fn draw_x_axis_with_tick_domain(
        &mut self,
        scale: &Scale,
        tick_domain: (f64, f64),
        label: &str,
    ) {
        let y = self.margin.top + self.plot_height();
        self.add_line(
            self.margin.left,
            y,
            self.margin.left + self.plot_width(),
            y,
            self.theme.axis_colour,
            self.theme.axis_width,
        );
        let x_scale = Scale {
            domain: scale.domain,
            range: (self.margin.left, self.margin.left + self.plot_width()),
        };
        let ticks = Scale {
            domain: tick_domain,
            range: tick_domain,
        }
        .nice_ticks(5);
        let decimals = tick_decimals(&ticks);
        for tick in ticks {
            let x = x_scale.map(tick);
            self.add_line(
                x,
                y,
                x,
                y + 5.0,
                self.theme.axis_colour,
                self.theme.axis_width,
            );
            self.add_text(
                x,
                y + 18.0,
                &format!("{tick:.decimals$}"),
                "middle",
                self.theme.tick_size,
            );
        }
        self.add_axis_title(
            self.margin.left + self.plot_width() / 2.0,
            if self.theme.is_adaptive() {
                y + 36.0
            } else {
                self.height - 5.0
            },
            label,
            "x",
            None,
        );
    }

    /// An x axis of labels rather than numbers, for a bar chart.
    ///
    /// A bar chart's x column is almost always categories, and a category has
    /// no numeric position: reading it through `extract_table_col` turns
    /// "Biology" into NaN and draws an axis running 0.0 to 1.0 underneath bars
    /// that have nothing to do with those numbers. This puts each category's
    /// own name under its group instead.
    ///
    /// Labels are thinned when there are more of them than the axis can fit,
    /// because overlapping text is less readable than fewer labels.
    pub(crate) fn draw_category_axis(&mut self, labels: &[String], axis_label: &str) {
        let y = self.margin.top + self.plot_height();
        self.add_line(
            self.margin.left,
            y,
            self.margin.left + self.plot_width(),
            y,
            self.theme.axis_colour,
            self.theme.axis_width,
        );
        if !labels.is_empty() {
            let slot = self.plot_width() / labels.len() as f64;
            // Roughly 46px of room per label before they start to collide.
            let step = (46.0 / slot).ceil().max(1.0) as usize;
            for (index, label) in labels.iter().enumerate().step_by(step) {
                let x = self.margin.left + slot * (index as f64 + 0.5);
                self.add_line(
                    x,
                    y,
                    x,
                    y + 5.0,
                    self.theme.axis_colour,
                    self.theme.axis_width,
                );
                self.add_text(x, y + 18.0, label, "middle", self.theme.tick_size);
            }
        }
        if !axis_label.is_empty() {
            self.add_axis_title(
                self.margin.left + self.plot_width() / 2.0,
                if self.theme.is_adaptive() {
                    y + 36.0
                } else {
                    self.height - 5.0
                },
                axis_label,
                "x",
                None,
            );
        }
    }

    pub(crate) fn draw_y_axis(&mut self, scale: &Scale, label: &str) {
        self.draw_y_axis_with_tick_domain(scale, scale.domain, label);
    }

    /// Draw a y axis at explicit positions with explicit labels.
    pub(crate) fn draw_y_axis_with_ticks(
        &mut self,
        scale: &Scale,
        ticks: &[(f64, String)],
        label: &str,
    ) {
        self.draw_y_axis_with_ticks_rotated(scale, ticks, label, 0.0);
    }

    pub(crate) fn draw_y_axis_with_ticks_rotated(
        &mut self,
        scale: &Scale,
        ticks: &[(f64, String)],
        label: &str,
        rotation: f64,
    ) {
        let x = self.margin.left;
        self.add_line(
            x,
            self.margin.top,
            x,
            self.margin.top + self.plot_height(),
            self.theme.axis_colour,
            self.theme.axis_width,
        );
        let y_scale = Scale {
            domain: scale.domain,
            range: (self.margin.top + self.plot_height(), self.margin.top),
        };
        for (tick, text) in ticks {
            let y = y_scale.map(*tick);
            self.add_line(
                x - 5.0,
                y,
                x,
                y,
                self.theme.axis_colour,
                self.theme.axis_width,
            );
            if rotation.abs() <= f64::EPSILON {
                self.add_text(x - 8.0, y + 4.0, text, "end", self.theme.tick_size);
            } else {
                self.add_text_rotated(x - 17.0, y, text, rotation, "middle", self.theme.tick_size);
            }
        }
        self.add_axis_title(
            15.0,
            self.margin.top + self.plot_height() / 2.0,
            label,
            "y",
            Some(-90.0),
        );
    }

    /// Y-axis counterpart to [`Self::draw_x_axis_with_tick_domain`].
    pub(crate) fn draw_y_axis_with_tick_domain(
        &mut self,
        scale: &Scale,
        tick_domain: (f64, f64),
        label: &str,
    ) {
        let x = self.margin.left;
        self.add_line(
            x,
            self.margin.top,
            x,
            self.margin.top + self.plot_height(),
            self.theme.axis_colour,
            self.theme.axis_width,
        );
        let y_scale = Scale {
            domain: scale.domain,
            range: (self.margin.top + self.plot_height(), self.margin.top),
        };
        let ticks = Scale {
            domain: tick_domain,
            range: tick_domain,
        }
        .nice_ticks(5);
        let decimals = tick_decimals(&ticks);
        for tick in ticks {
            let y = y_scale.map(tick);
            self.add_line(
                x - 5.0,
                y,
                x,
                y,
                self.theme.axis_colour,
                self.theme.axis_width,
            );
            self.add_text(
                x - 8.0,
                y + 4.0,
                &format!("{tick:.decimals$}"),
                "end",
                self.theme.tick_size,
            );
        }
        self.add_axis_title(
            15.0,
            self.margin.top + self.plot_height() / 2.0,
            label,
            "y",
            Some(-90.0),
        );
    }

    pub(crate) fn draw_title(&mut self, title: &str) {
        self.accessible_label = Some(title.to_string());
        if self.theme.is_adaptive() {
            self.add_text_styled(
                self.margin.left,
                24.0,
                title,
                "start",
                self.theme.title_size,
                "600",
                self.theme.text_colour,
            );
        } else {
            self.add_text(
                self.width / 2.0,
                25.0,
                title,
                "middle",
                self.theme.title_size,
            );
        }
    }

    pub(crate) fn draw_title_centered(&mut self, title: &str) {
        self.accessible_label = Some(title.to_string());
        self.add_text_styled(
            self.width / 2.0,
            25.0,
            title,
            "middle",
            self.theme.title_size,
            "600",
            self.theme.text_colour,
        );
    }

    pub(crate) fn draw_subtitle(&mut self, subtitle: &str) {
        if subtitle.is_empty() {
            return;
        }
        self.add_text_styled(
            self.margin.left,
            42.0,
            subtitle,
            "start",
            self.theme.subtitle_size,
            "normal",
            "#5f6368",
        );
    }

    pub(crate) fn draw_caption(&mut self, caption: &str) {
        if caption.is_empty() {
            return;
        }
        self.add_text_styled(
            self.width - self.margin.right,
            self.height - 5.0,
            caption,
            "end",
            self.theme.caption_size,
            "normal",
            "#686d76",
        );
    }

    pub(crate) fn set_accessible_description(&mut self, description: impl Into<String>) {
        self.accessible_description = Some(description.into());
    }

    pub(crate) fn render(&self) -> String {
        let escape = |value: &str| {
            value
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
        };
        let label = escape(self.accessible_label.as_deref().unwrap_or("BioLang plot"));
        let description = escape(
            self.accessible_description
                .as_deref()
                .unwrap_or("BioLang data visualization."),
        );
        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" role="img" aria-label="{}" data-biolang-theme="{}" focusable="false"><title>{}</title><desc>{}</desc>"#,
            self.width,
            self.height,
            self.width,
            self.height,
            label,
            self.theme.name,
            label,
            description
        );
        svg.push_str(&format!(
            r#"<rect width="{}" height="{}" fill="{}" />"#,
            self.width, self.height, self.theme.background_colour
        ));
        for el in &self.elements {
            svg.push_str(el);
        }
        svg.push_str("</svg>");
        svg
    }
}

// ── Option parsing helpers ──────────────────────────────────────

/// A key naming each series, drawn inside the top right of the plot area.
///
/// Only when there is more than one: a legend for a single series is a caption
/// repeating the axis label.
pub(super) fn draw_legend(canvas: &mut SvgCanvas, names: &[String]) {
    if names.len() < 2 {
        return;
    }
    let panel_right = canvas.margin.left + canvas.plot_width();
    let outside = canvas.theme.is_adaptive();
    for (index, name) in names.iter().enumerate() {
        let y = canvas.margin.top + 14.0 + 18.0 * index as f64;
        let swatch_start = if outside {
            panel_right + 14.0
        } else {
            panel_right - 30.0
        };
        let swatch_end = swatch_start + 22.0;
        canvas.add_line(
            swatch_start,
            y,
            swatch_end,
            y,
            PALETTE[index % PALETTE.len()],
            3.0,
        );
        canvas.add_text(
            if outside {
                swatch_end + 6.0
            } else {
                swatch_start - 6.0
            },
            y + 4.0,
            name,
            if outside { "start" } else { "end" },
            canvas.theme.legend_size,
        );
    }
}

pub(super) fn legend_reserve_width(theme: PlotTheme, names: &[String]) -> f64 {
    if !theme.is_adaptive() || names.len() < 2 {
        return 0.0;
    }
    let widest = names
        .iter()
        .map(|name| estimate_text_width(name, theme.legend_size))
        .fold(0.0, f64::max);
    (52.0 + widest).clamp(90.0, 210.0)
}
