//! What BioLang claims to reproduce from ggplot2, pinned to R's own values.
//!
//! Every defect found while making the NHANES lesson match its source chapter
//! was a place where BioLang and ggplot2 disagreed about a documented
//! behaviour, not a rendering bug: a palette table copied from one group
//! count, `bins` read the matplotlib way, a scale expanded 8% instead of 5%,
//! and text measured 16% wide. None of those would be caught by diffing
//! against another plotting library, because every library has its own
//! defaults. They are caught by writing R's answer down.
//!
//! So each test here names the R behaviour it pins and asserts the rendered
//! output carries it. A failure means BioLang drifted away from ggplot2, which
//! is a decision worth making on purpose rather than by accident.

use bl_core::value::Value;
use bl_runtime::plot::call_plot_builtin;
use bl_runtime::stats::call_stats_builtin;
use std::collections::HashMap;

fn numbers(values: &[f64]) -> Value {
    Value::List(
        values
            .iter()
            .copied()
            .map(Value::Float)
            .collect::<Vec<_>>()
            .into(),
    )
}

fn strings(values: &[&str]) -> Value {
    Value::List(
        values
            .iter()
            .map(|value| Value::Str((*value).into()))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn options(pairs: &[(&str, Value)]) -> Value {
    Value::Record(
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect::<HashMap<_, _>>()
            .into(),
    )
}

fn svg(result: Value) -> String {
    match result {
        Value::Str(text) => text.to_string(),
        other => panic!("expected an SVG string, got {other:?}"),
    }
}

/// A scatter with two groups, which is the NHANES height/weight figure's shape.
fn two_group_scatter(extra: &[(&str, Value)]) -> String {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    let mut groups = Vec::new();
    for index in 0..60 {
        let position = f64::from(index);
        xs.push(150.0 + position * 0.5);
        ys.push(60.0 + position * 0.4 + f64::from(index % 7));
        groups.push(if index % 2 == 0 { "female" } else { "male" });
    }
    let mut pairs: Vec<(&str, Value)> = vec![("group", strings(&groups))];
    pairs.extend_from_slice(extra);
    svg(call_stats_builtin(
        "stats_relationship_plot",
        vec![numbers(&xs), numbers(&ys), options(&pairs)],
    )
    .expect("relationship plot failed"))
}

/// `scales::hue_pal()` walks evenly spaced hues for each group count, so a
/// fixed table is only ever right at the one count it was copied from.
#[test]
fn discrete_colours_are_r_hue_pal_at_every_group_count() {
    let expected: [(usize, &[&str]); 5] = [
        (2, &["#f8766d", "#00bfc4"]),
        (3, &["#f8766d", "#00ba38", "#619cff"]),
        (4, &["#f8766d", "#7cae00", "#00bfc4", "#c77cff"]),
        (5, &["#f8766d", "#a3a500", "#00bf7d", "#00b0f6", "#e76bf3"]),
        (
            6,
            &[
                "#f8766d", "#b79f00", "#00ba38", "#00bfc4", "#619cff", "#f564e3",
            ],
        ),
    ];
    for (count, colours) in expected {
        let labels: Vec<String> = (0..count).map(|index| format!("g{index}")).collect();
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        let mut groups = Vec::new();
        for round in 0..6 {
            for (index, label) in labels.iter().enumerate() {
                xs.push(f64::from(round) + index as f64 * 0.1);
                ys.push(f64::from(round) * 2.0 + index as f64);
                groups.push(label.as_str());
            }
        }
        let rendered = svg(call_stats_builtin(
            "stats_relationship_plot",
            vec![
                numbers(&xs),
                numbers(&ys),
                options(&[("group", strings(&groups))]),
            ],
        )
        .expect("relationship plot failed"));
        for colour in colours {
            assert!(
                rendered.contains(colour),
                "hue_pal({count}) should contain {colour}"
            );
        }
    }
}

/// `aes(col = ...)` maps colour but not fill, so `geom_smooth()` keeps its
/// default grey60 ribbon whatever the group colour is.
#[test]
fn confidence_ribbons_are_grey60_not_the_group_colour() {
    let rendered = two_group_scatter(&[("interval", Value::Str("confidence".into()))]);
    let ribbons = rendered.matches("fill=\"#999999\"").count();
    assert_eq!(ribbons, 2, "expected one grey60 ribbon per group");
    assert!(
        rendered.contains("fill-opacity=\"0.40\""),
        "geom_smooth's ribbon is drawn at alpha 0.4"
    );
}

/// `geom_smooth()` fits each group across that group's own x range. Spanning
/// the full axis is `fullrange = TRUE`, which the source chapter devotes an
/// exercise to warning against.
#[test]
fn fitted_lines_do_not_extrapolate_beyond_their_group() {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    let mut groups = Vec::new();
    // Two groups occupying clearly separated halves of the x axis.
    for index in 0..30 {
        xs.push(f64::from(index));
        ys.push(f64::from(index) * 0.5);
        groups.push("low");
    }
    for index in 60..90 {
        xs.push(f64::from(index));
        ys.push(f64::from(index) * 0.5);
        groups.push("high");
    }
    let rendered = svg(call_stats_builtin(
        "stats_relationship_plot",
        vec![
            numbers(&xs),
            numbers(&ys),
            options(&[("group", strings(&groups))]),
        ],
    )
    .expect("relationship plot failed"));

    let mut spans = Vec::new();
    for line in rendered.split("<line").skip(1) {
        if !line.contains("stroke-width=\"2.85\"") {
            continue;
        }
        let read = |name: &str| -> Option<f64> {
            let at = line.find(name)? + name.len();
            let rest = &line[at..];
            let end = rest.find('"')?;
            rest[..end].parse().ok()
        };
        if let (Some(x1), Some(x2)) = (read("x1=\""), read("x2=\"")) {
            spans.push((x1, x2));
        }
    }
    // Two fitted lines and two legend swatches; keep the two widest.
    spans.sort_by(|a, b| (b.1 - b.0).partial_cmp(&(a.1 - a.0)).unwrap());
    assert!(spans.len() >= 2, "expected two fitted lines");
    let (first, second) = (spans[0], spans[1]);
    assert!(
        (first.0 - second.0).abs() > 1.0 || (first.1 - second.1).abs() > 1.0,
        "both groups' fits span the same range, which is fullrange = TRUE: {first:?} vs {second:?}"
    );
}

/// ggplot2's continuous scales expand by 5% of the data range on each side.
///
/// This has to be measured, not eyeballed: an 8% expansion still puts every
/// point inside the panel and still looks like a scatter plot. The way to tell
/// is where the extreme point lands as a fraction of the panel.
#[test]
fn continuous_scales_expand_five_percent_per_side() {
    // Data spanning exactly 0..100. Expanded 5% per side the domain is
    // -5..105, so the minimum sits 5/110 = 4.545% across the panel. At 8% it
    // would sit at 8/116 = 6.897%, which is the drift this pins down.
    let values: Vec<f64> = (0..=100).map(f64::from).collect();
    let rendered = svg(call_stats_builtin(
        "stats_relationship_plot",
        vec![numbers(&values), numbers(&values), options(&[])],
    )
    .expect("relationship plot failed"));

    let attribute = |fragment: &str, name: &str| -> f64 {
        fragment
            .split(name)
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| panic!("no {name} in {fragment:.80}"))
    };
    let panel = rendered
        .split("<rect ")
        .find(|fragment| fragment.contains("fill=\"#ebebeb\""))
        .expect("no themed panel was drawn");
    let panel_left = attribute(panel, "x=\"");
    let panel_width = attribute(panel, "width=\"");

    let leftmost = rendered
        .split("<circle ")
        .skip(1)
        .map(|fragment| attribute(fragment, "cx=\""))
        .fold(f64::INFINITY, f64::min);
    let fraction = (leftmost - panel_left) / panel_width;
    assert!(
        (fraction - 0.045_454).abs() < 0.003,
        "the smallest value should sit 4.545% across the panel for a 5% expansion, \
         but sat at {:.3}% (8% expansion would give 6.897%)",
        fraction * 100.0
    );
}

/// `theme_grey()` is a grey panel with white gridlines inside a white figure;
/// `theme_classic()` is a white panel with no grid and black axes.
#[test]
fn the_named_themes_match_their_ggplot_originals() {
    let grey = two_group_scatter(&[("theme", Value::Str("ggplot".into()))]);
    assert!(
        grey.contains("fill=\"#ebebeb\""),
        "theme_grey needs its grey panel"
    );
    assert!(
        grey.contains("stroke=\"#ffffff\""),
        "theme_grey needs white gridlines"
    );

    let classic = svg(call_stats_builtin(
        "stats_group_plot",
        vec![
            numbers(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
            strings(&["a", "a", "a", "b", "b", "b"]),
            options(&[("theme", Value::Str("classic".into()))]),
        ],
    )
    .expect("group plot failed"));
    assert!(
        !classic.contains("fill=\"#ebebeb\""),
        "theme_classic has no grey panel"
    );
    assert!(
        classic.contains("#000000"),
        "theme_classic draws black axes"
    );
}

/// ggplot scale transformations move the geometry but keep tick labels in the
/// original units. Pre-transforming the vectors and drawing a linear axis is
/// not equivalent: it labels 1, 3 and 5 instead of 1e+01, 1e+03 and 1e+05.
#[test]
fn log10_relationship_axes_keep_source_unit_labels() {
    let rendered = svg(call_stats_builtin(
        "stats_relationship_plot",
        vec![
            numbers(&[1.0, 10.0, 100.0, 1_000.0, 100_000.0]),
            numbers(&[2.0, 20.0, 200.0, 2_000.0, 200_000.0]),
            options(&[
                ("fit", Value::Bool(false)),
                ("x_scale", Value::Str("log10".into())),
                ("y_scale", Value::Str("log10".into())),
                ("x_breaks", numbers(&[10.0, 1_000.0, 100_000.0])),
                ("y_breaks", numbers(&[10.0, 1_000.0, 100_000.0])),
            ]),
        ],
    )
    .expect("log relationship plot failed"));
    for label in ["1e+01", "1e+03", "1e+05"] {
        assert!(
            rendered.contains(&format!(">{label}</text>")),
            "log axis should retain source-unit label {label}"
        );
    }
}

/// A colour grouping changes the legend and point palette, not the meaning of
/// a continuous scale. MA plots need the same source-unit labels whether or
/// not significance is mapped to colour.
#[test]
fn grouped_log10_relationship_axes_keep_source_unit_labels() {
    let rendered = svg(call_stats_builtin(
        "stats_relationship_plot",
        vec![
            numbers(&[1.0, 10.0, 100.0, 1_000.0, 100_000.0]),
            numbers(&[-1.0, -0.5, 0.0, 0.5, 1.0]),
            options(&[
                ("group", strings(&["FALSE", "FALSE", "TRUE", "TRUE", "NA"])),
                ("fit", Value::Bool(false)),
                ("x_scale", Value::Str("log10".into())),
                ("x_breaks", numbers(&[1.0, 100.0, 10_000.0])),
                ("x_tick_format", Value::Str("plain".into())),
                ("note", Value::Bool(false)),
            ]),
        ],
    )
    .expect("grouped log relationship plot failed"));
    for label in ["1", "100", "10000"] {
        assert!(
            rendered.contains(&format!(">{label}</text>")),
            "grouped log axis should retain source-unit label {label}"
        );
    }
    assert!(!rendered.contains("complete pairs;"));
}

/// A discrete ggplot x scale has major gridlines at the category centres, not
/// only horizontal rules. This is visible in the source CRISPLD2 boxplot.
#[test]
fn categorical_ggplot_panels_include_vertical_major_gridlines() {
    let rendered = svg(call_stats_builtin(
        "stats_group_plot",
        vec![
            numbers(&[700.0, 800.0, 900.0, 3_000.0, 5_000.0, 6_000.0]),
            strings(&[
                "control", "control", "control", "treated", "treated", "treated",
            ]),
            options(&[("theme", Value::Str("ggplot".into()))]),
        ],
    )
    .expect("group plot failed"));
    let white_grid_lines = rendered.matches("stroke=\"#ffffff\"").count();
    assert!(
        white_grid_lines >= 5,
        "expected three horizontal and two categorical gridlines, got {white_grid_lines}"
    );
}

/// A base-R-style point panel needs open circles and a complete border. These
/// are explicit options so ordinary ggplot boxplots keep their defaults.
#[test]
fn group_plot_can_draw_open_points_and_a_full_panel_border() {
    let rendered = svg(call_stats_builtin(
        "stats_group_plot",
        vec![
            numbers(&[700.0, 800.0, 5_000.0, 6_000.0]),
            strings(&["control", "control", "treated", "treated"]),
            options(&[
                ("box", Value::Bool(false)),
                ("points", Value::Str("jitter".into())),
                ("point_style", Value::Str("open".into())),
                ("panel_border", Value::Bool(true)),
                ("theme", Value::Str("classic".into())),
            ]),
        ],
    )
    .expect("open point plot failed"));
    assert!(
        rendered.contains("fill=\"#ffffff\" stroke=\"#333333\""),
        "open circles should have a white centre and dark outline"
    );
    assert!(
        rendered.contains("fill=\"none\" stroke=\"#000000\""),
        "the requested base-style panel border should surround all four sides"
    );
}

/// `geom_boxplot()` is a white box outlined in grey20, with the median drawn
/// at twice the box line weight (`fatten = 2`), and only outliers marked.
#[test]
fn boxplots_use_ggplot_defaults() {
    let values: Vec<f64> = (0..40)
        .map(|index| if index == 39 { 500.0 } else { f64::from(index) })
        .collect();
    let labels = vec!["a"; 40];
    let rendered = svg(call_stats_builtin(
        "stats_group_plot",
        vec![numbers(&values), strings(&labels), options(&[])],
    )
    .expect("group plot failed"));
    assert!(
        rendered.contains("stroke=\"#333333\" stroke-width=\"1.42\""),
        "the box is outlined in grey20 at linewidth 0.5"
    );
    assert!(
        rendered.contains("stroke=\"#333333\" stroke-width=\"2.85\""),
        "the median is drawn at fatten = 2"
    );
    // The single extreme value is the only point that should be marked.
    let points = rendered.matches("<circle").count();
    assert!(
        points <= 3,
        "geom_boxplot marks outliers, not every observation; got {points}"
    );
}

/// `aes(fill = group)` uses the same discrete hue scale as ggplot2. Keep this
/// opt-in: an unmapped `geom_boxplot()` is still white, as tested above.
#[test]
fn mapped_boxplot_fill_uses_r_hue_colours() {
    let rendered = svg(call_stats_builtin(
        "stats_group_plot",
        vec![
            numbers(&[1.0, 2.0, 3.0, 2.0, 3.0, 4.0, 3.0, 4.0, 5.0]),
            strings(&[
                "BRCA", "BRCA", "BRCA", "OV", "OV", "OV", "UCEC", "UCEC", "UCEC",
            ]),
            options(&[
                ("theme", Value::Str("classic".into())),
                ("fill", Value::Str("group".into())),
                ("legend_title", Value::Str("dataset".into())),
                ("points", Value::Str("none".into())),
            ]),
        ],
    )
    .expect("filled group plot failed"));
    for colour in ["#f8766d", "#00ba38", "#619cff"] {
        assert!(
            rendered.contains(&format!("fill=\"{colour}\"")),
            "mapped boxplot should contain R hue {colour}"
        );
    }
    assert!(
        rendered.contains(">dataset</text>"),
        "mapped fill should identify the grouping variable in an outside legend"
    );
}

/// `bins = n` in ggplot2 is a width of range/(n-1) with the first bin centred
/// on the minimum, not an equal split of [min, max].
#[test]
fn histogram_bins_follow_ggplot_not_matplotlib() {
    let values: Vec<f64> = (0..100).map(f64::from).collect();
    let render = |rule: Option<&str>| -> String {
        let mut pairs: Vec<(&str, Value)> = vec![("bins", Value::Int(10))];
        if let Some(rule) = rule {
            pairs.push(("bin_rule", Value::Str(rule.into())));
        }
        svg(
            call_plot_builtin("histogram", vec![numbers(&values), options(&pairs)])
                .expect("histogram failed"),
        )
    };
    let default = render(None);
    let ggplot = render(Some("ggplot"));
    let span = render(Some("span"));
    assert_eq!(default, ggplot, "the ggplot rule is the default");
    assert_ne!(
        ggplot, span,
        "the two bin rules must not produce the same bars"
    );
    // ggplot2's bars abut; they are separated by a gap only in the legacy look.
    assert!(
        ggplot.contains("fill=\"#595959\""),
        "geom_histogram fills grey35"
    );
}

/// Text is measured from the font's own advance widths, so a label's width is
/// the sum of its glyphs rather than a character-class guess.
#[test]
fn labels_are_measured_with_real_font_metrics() {
    // A label of wide glyphs and one of narrow glyphs, same character count.
    let wide = two_group_scatter(&[("legend_title", Value::Str("WWWWWWWW".into()))]);
    let narrow = two_group_scatter(&[("legend_title", Value::Str("iiiiiiii".into()))]);
    let panel_of = |svg: &str| -> f64 {
        svg.split("<rect x=\"")
            .nth(1)
            .and_then(|rest| rest.split("width=\"").nth(1))
            .and_then(|rest| rest.split('"').next())
            .and_then(|value| value.parse().ok())
            .unwrap_or(0.0)
    };
    let wide_panel = panel_of(&wide);
    let narrow_panel = panel_of(&narrow);
    assert!(
        narrow_panel > wide_panel,
        "a legend of narrow glyphs should leave more room for the panel ({narrow_panel} vs {wide_panel})"
    );
    // Arial advances W at 944/1000 em and i at 222/1000, a ratio over four to
    // one; a character-class estimate collapses it to 0.90 against 0.30.
    assert!(
        narrow_panel - wide_panel > 20.0,
        "the difference should reflect real advance widths, got {}",
        narrow_panel - wide_panel
    );
}

/// `geom_point()` is opaque at a 2.6px radius: `size = 1.5` with `stroke =
/// 0.5` works out to that radius at 96dpi, and `alpha` is unset unless asked
/// for. Both were silently changeable before this test existed.
#[test]
fn scatter_points_carry_the_ggplot_marker_size_and_opacity() {
    let rendered = two_group_scatter(&[]);
    let data_points = rendered
        .split("<circle ")
        .skip(1)
        .filter(|fragment| fragment.contains("r=\"2.6\""))
        .count();
    assert!(
        data_points >= 60,
        "expected every observation drawn at r=2.6, got {data_points}"
    );
    assert!(
        rendered.contains("r=\"2.6\" fill=\"#f8766d\" opacity=\"1.00\""),
        "geom_point is opaque unless alpha is set"
    );

    // `alpha` is the escape hatch, and it has to actually reach the marks.
    let faded = two_group_scatter(&[("alpha", Value::Float(0.35))]);
    assert!(
        faded.contains("opacity=\"0.35\""),
        "the alpha option should reach the points"
    );
}

/// ggplot2 draws points in data order, so neither group sits on top of the
/// other where they overlap. Emitting one whole group and then the next is a
/// different picture from the same numbers.
#[test]
fn points_are_drawn_in_data_order_not_grouped_by_series() {
    // The helper alternates female/male row by row, so a data-order draw
    // alternates colours and a grouped draw does not.
    let rendered = two_group_scatter(&[]);
    let fills: Vec<&str> = rendered
        .split("<circle ")
        .skip(1)
        .filter(|fragment| fragment.contains("r=\"2.6\""))
        .filter_map(|fragment| {
            fragment
                .split("fill=\"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
        })
        .collect();
    assert!(fills.len() >= 60, "expected the full point cloud");
    let changes = fills.windows(2).filter(|pair| pair[0] != pair[1]).count();
    assert!(
        changes > fills.len() / 2,
        "colours should alternate with the rows ({changes} changes across {} points); \
         one change means each group was emitted as a block",
        fills.len()
    );
}

/// A `fixed` facet bins every panel against one set of edges.
///
/// Bar pixel width cannot tell the two modes apart: under `free_x` each panel
/// is scaled to its own domain, so ten bins fill the panel either way. What
/// changes is how many bins a group actually occupies - and that is the point
/// of a shared scale, that a narrow group looks narrow.
#[test]
fn a_fixed_facet_bins_every_panel_against_the_same_edges() {
    // One group covering 0..79, another covering 0..790. Shared edges put the
    // narrow group into a couple of bins at the left; per-panel edges spread
    // it across all ten.
    let mut values = Vec::new();
    let mut labels = Vec::new();
    for index in 0..80 {
        values.push(f64::from(index));
        labels.push("narrow");
    }
    for index in 0..80 {
        values.push(f64::from(index) * 10.0);
        labels.push("wide");
    }

    let bar_count = |scales: &str| -> usize {
        let rendered = svg(call_stats_builtin(
            "stats_facet_plot",
            vec![
                numbers(&values),
                strings(&labels),
                options(&[
                    ("bins", Value::Int(10)),
                    ("columns", Value::Int(2)),
                    ("scales", Value::Str(scales.into())),
                ]),
            ],
        )
        .expect("facet plot failed"));
        rendered
            .split("<rect ")
            .skip(1)
            .filter(|fragment| fragment.contains("fill=\"#595959\""))
            .count()
    };

    let fixed = bar_count("fixed");
    let free = bar_count("free_x");
    assert!(
        fixed < free,
        "a shared scale should collapse the narrow group into fewer bins \
         ({fixed} bars) than binning each panel separately ({free} bars)"
    );
    assert!(
        fixed <= 12,
        "the narrow group should occupy only the leftmost shared bins, got {fixed} bars total"
    );
}

/// The legacy marker opacity still governs everything that has not been moved
/// onto an explicit alpha: the boxplot's jitter overlay, and every biological
/// figure. Changing it silently restyles those, so it is pinned here even
/// though it is not a ggplot2 value.
#[test]
fn the_legacy_marker_opacity_is_pinned_at_070() {
    let values: Vec<f64> = (0..30).map(f64::from).collect();
    let labels = vec!["a"; 30];
    let rendered = svg(call_stats_builtin(
        "stats_group_plot",
        vec![
            numbers(&values),
            strings(&labels),
            options(&[("points", Value::Str("jitter".into()))]),
        ],
    )
    .expect("group plot failed"));
    assert!(
        rendered.contains("opacity=\"0.70\""),
        "the jitter overlay draws through add_circle's 0.7 default"
    );
}

/// `geom_smooth(fullrange = TRUE)` with `xlim(0, NA)` is the source chapter's
/// own warning against extrapolation, so the lesson has to be able to draw it.
/// It is opt-in precisely because the default must not do it.
#[test]
fn fullrange_and_limits_reproduce_the_extrapolation_warning() {
    let xs: Vec<f64> = (140..=200).map(f64::from).collect();
    let ys: Vec<f64> = xs.iter().map(|x| 0.92 * x - 73.7).collect();

    let render = |extra: &[(&str, Value)]| -> String {
        let mut pairs: Vec<(&str, Value)> = vec![("interval", Value::Str("confidence".into()))];
        pairs.extend_from_slice(extra);
        svg(call_stats_builtin(
            "stats_relationship_plot",
            vec![numbers(&xs), numbers(&ys), options(&pairs)],
        )
        .expect("relationship plot failed"))
    };
    let ticks = |svg: &str| -> Vec<f64> {
        svg.split("text-anchor=\"middle\"")
            .skip(1)
            .filter_map(|fragment| fragment.split('>').nth(1))
            .filter_map(|rest| rest.split('<').next())
            .filter_map(|text| text.parse::<f64>().ok())
            .collect()
    };

    let plain = render(&[]);
    let extrapolated = render(&[
        ("fullrange", Value::Bool(true)),
        (
            "x_limits",
            Value::List(vec![Value::Float(0.0), Value::Nil].into()),
        ),
    ]);
    assert_ne!(plain, extrapolated, "fullrange had no effect");

    let smallest = ticks(&extrapolated)
        .into_iter()
        .fold(f64::INFINITY, f64::min);
    assert!(
        smallest <= 0.0,
        "x_limits [0, nil] should carry the axis down to zero, smallest tick was {smallest}"
    );
    assert!(
        ticks(&plain).into_iter().fold(f64::INFINITY, f64::min) > 100.0,
        "without limits the axis should still start near the data"
    );
}

/// `aes(col = , size = )` on a residual plot, as the source chapter's
/// multiple-regression diagnostic uses. Size scales by area, which is how
/// ggplot2's `scale_size_continuous()` works and the only way the eye reads a
/// size scale correctly.
#[test]
fn residual_plots_map_colour_and_size_like_ggplot() {
    let xs: Vec<f64> = (0..40).map(f64::from).collect();
    let ys: Vec<f64> = xs.iter().map(|x| x * 1.5 + (x % 5.0)).collect();
    let groups: Vec<&str> = (0..40)
        .map(|index| if index % 2 == 0 { "No" } else { "Yes" })
        .collect();
    let ages: Vec<f64> = (0..40).map(|index| 20.0 + f64::from(index)).collect();

    let rendered = svg(call_stats_builtin(
        "stats_linear_diagnostic_plot",
        vec![
            numbers(&xs),
            numbers(&ys),
            options(&[
                ("view", Value::Str("residuals".into())),
                ("color", strings(&groups)),
                ("size", numbers(&ages)),
            ]),
        ],
    )
    .expect("residual plot failed"));

    // Two levels, so ggplot2's two-colour hue palette.
    assert!(rendered.contains("#f8766d") && rendered.contains("#00bfc4"));

    let radii: Vec<f64> = rendered
        .split("<circle ")
        .skip(1)
        .filter_map(|fragment| {
            fragment
                .split("r=\"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .and_then(|value| value.parse().ok())
        })
        .collect();
    let smallest = radii.iter().copied().fold(f64::INFINITY, f64::min);
    let largest = radii.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        largest > smallest * 2.0,
        "a mapped size should vary the markers, got {smallest} to {largest}"
    );
}

/// An aesthetic is indexed against the rows that were actually plotted, and
/// `complete_pairs` silently drops incomplete ones. A length that cannot line
/// up has to be an error rather than a quietly shifted figure.
#[test]
fn a_misaligned_aesthetic_is_refused_rather_than_shifted() {
    let xs = numbers(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let ys = numbers(&[2.0, 4.0, 5.0, 8.0, 11.0]);
    let error = call_stats_builtin(
        "stats_linear_diagnostic_plot",
        vec![
            xs,
            ys,
            options(&[
                ("view", Value::Str("residuals".into())),
                ("color", strings(&["a", "b"])),
            ]),
        ],
    )
    .expect_err("a short colour list should be refused");
    assert!(
        format!("{error}").contains("one entry per observation"),
        "the error should say what is wrong: {error}"
    );
}
