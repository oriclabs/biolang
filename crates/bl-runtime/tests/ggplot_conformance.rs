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
