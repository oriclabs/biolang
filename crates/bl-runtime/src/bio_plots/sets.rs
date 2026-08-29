//! Sets for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

pub(super) fn builtin_oncoprint(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "oncoprint")?;
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let samples = extract_str_col(table, get_opt_str(&opts, "sample", "sample"))?;
    let genes = extract_str_col(table, get_opt_str(&opts, "gene", "gene"))?;
    let mut_types =
        extract_str_col(table, "type").unwrap_or_else(|_| vec!["mutation".into(); samples.len()]);

    let sample_order: Vec<String> = {
        let mut s = Vec::new();
        for x in &samples {
            if !s.contains(x) {
                s.push(x.clone());
            }
        }
        s
    };
    let gene_order: Vec<String> = {
        let mut g = Vec::new();
        for x in &genes {
            if !g.contains(x) {
                g.push(x.clone());
            }
        }
        g
    };
    let mut grid: HashMap<(usize, usize), String> = HashMap::new();
    for j in 0..samples.len() {
        let si = sample_order.iter().position(|s| s == &samples[j]).unwrap();
        let gi = gene_order.iter().position(|g| g == &genes[j]).unwrap();
        grid.insert((gi, si), mut_types[j].clone());
    }

    let type_colors: HashMap<&str, &str> = [
        ("missense", "#e15759"),
        ("nonsense", "#333"),
        ("frameshift", "#4e79a7"),
        ("splice", "#76b7b2"),
        ("mutation", "#e15759"),
    ]
    .into();

    if fmt == "svg" {
        let cell = 12.0;
        let w = get_opt_f64(
            &opts,
            "width",
            (sample_order.len() as f64 * cell + 120.0).max(400.0),
        );
        let h = get_opt_f64(
            &opts,
            "height",
            (gene_order.len() as f64 * cell + 60.0).max(200.0),
        );
        let mut c = themed_canvas(w, h, &opts);
        c.margin.left = 100.0;
        let cw = c.plot_width() / sample_order.len().max(1) as f64;
        let ch = c.plot_height() / gene_order.len().max(1) as f64;
        for gi in 0..gene_order.len() {
            let y = c.margin.top + gi as f64 * ch;
            c.add_text(
                c.margin.left - 3.0,
                y + ch / 2.0 + 4.0,
                &gene_order[gi],
                "end",
                10.0,
            );
            for si in 0..sample_order.len() {
                let x = c.margin.left + si as f64 * cw;
                c.add_rect(x, y, cw - 1.0, ch - 1.0, "#f0f0f0");
                if let Some(mt) = grid.get(&(gi, si)) {
                    c.add_rect(
                        x,
                        y + ch * 0.15,
                        cw - 1.0,
                        ch * 0.7,
                        type_colors.get(mt.as_str()).copied().unwrap_or("#e15759"),
                    );
                }
            }
        }
        finish_themed_canvas(&mut c, &opts, "OncoPrint");
        return Ok(Value::Str(c.render()));
    }

    let max_gl = gene_order.iter().map(|g| g.len()).max().unwrap_or(4);
    let mut out = String::from("  OncoPrint\n");
    for gi in 0..gene_order.len() {
        out.push_str(&format!("  {:>w$}  ", gene_order[gi], w = max_gl));
        for si in 0..sample_order.len() {
            out.push(if grid.contains_key(&(gi, si)) {
                '█'
            } else {
                '·'
            });
        }
        out.push('\n');
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 14. venn ────────────────────────────────────────────────────

pub(super) fn builtin_venn(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let sets: Vec<(String, HashSet<String>)> = match &args[0] {
        Value::Record(map) => map
            .iter()
            .map(|(name, val)| {
                let items: HashSet<String> = match val {
                    Value::List(items) => items.iter().map(|v| format!("{v}")).collect(),
                    _ => HashSet::new(),
                };
                (name.clone(), items)
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "venn() requires Record of Lists",
                None,
            ))
        }
    };
    if sets.len() < 2 || sets.len() > 4 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "venn() needs 2-4 sets",
            None,
        ));
    }
    let names: Vec<&str> = sets.iter().map(|(n, _)| n.as_str()).collect();
    let set_refs: Vec<&HashSet<String>> = sets.iter().map(|(_, s)| s).collect();

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 500.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let mut c = themed_canvas(w, h, &opts);
        let (cx, cy) = (w / 2.0, h / 2.0);
        let r = w.min(h) * 0.25;
        let colors = ["#4e79a7", "#e15759", "#59a14f", "#edc948"];
        let offsets: Vec<(f64, f64)> = match sets.len() {
            2 => vec![(-r * 0.35, 0.0), (r * 0.35, 0.0)],
            3 => vec![(-r * 0.3, -r * 0.2), (r * 0.3, -r * 0.2), (0.0, r * 0.3)],
            _ => vec![
                (-r * 0.3, -r * 0.3),
                (r * 0.3, -r * 0.3),
                (-r * 0.3, r * 0.3),
                (r * 0.3, r * 0.3),
            ],
        };
        for (j, (dx, dy)) in offsets.iter().enumerate() {
            c.elements.push(format!(
                r#"<circle cx="{:.1}" cy="{:.1}" r="{r:.1}" fill="{}" opacity="0.25" stroke="{}" stroke-width="2" />"#,
                cx + dx, cy + dy, colors[j], colors[j]
            ));
            c.add_text(cx + dx * 2.5, cy + dy * 2.5, names[j], "middle", 12.0);
        }
        if sets.len() >= 2 {
            let inter: usize = set_refs[0].intersection(set_refs[1]).count();
            c.add_text(cx, cy, &inter.to_string(), "middle", 14.0);
        }
        finish_themed_canvas(&mut c, &opts, "Venn Diagram");
        return Ok(Value::Str(c.render()));
    }

    let mut out = String::from("  Venn Diagram\n");
    for (name, set) in &sets {
        out.push_str(&format!("  {name}: {} items\n", set.len()));
    }
    out.push('\n');
    for i in 0..sets.len() {
        for j in (i + 1)..sets.len() {
            let inter = set_refs[i].intersection(set_refs[j]).count();
            out.push_str(&format!("  {} ∩ {} = {}\n", names[i], names[j], inter));
        }
    }
    let mut common: HashSet<String> = set_refs[0].clone();
    for s in &set_refs[1..] {
        common = common.intersection(s).cloned().collect();
    }
    out.push_str(&format!("  All: {} shared\n", common.len()));
    write_output(&out);
    Ok(Value::Nil)
}

// ── 15. upset ───────────────────────────────────────────────────

pub(super) fn builtin_upset(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let sets: Vec<(String, HashSet<String>)> = match &args[0] {
        Value::Record(map) => map
            .iter()
            .map(|(n, v)| {
                let items: HashSet<String> = match v {
                    Value::List(l) => l.iter().map(|x| format!("{x}")).collect(),
                    _ => HashSet::new(),
                };
                (n.clone(), items)
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "upset() requires Record of Lists",
                None,
            ))
        }
    };
    let n = sets.len();
    if n < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "upset() needs >= 2 sets",
            None,
        ));
    }

    // Compute all intersection combinations
    let all_items: HashSet<String> = sets.iter().flat_map(|(_, s)| s.iter().cloned()).collect();
    let mut combos: Vec<(Vec<bool>, usize)> = Vec::new();
    for mask in 1..(1u32 << n) {
        let membership: Vec<bool> = (0..n).map(|i| mask & (1 << i) != 0).collect();
        let count = all_items
            .iter()
            .filter(|item| (0..n).all(|i| membership[i] == sets[i].1.contains(*item)))
            .count();
        if count > 0 {
            combos.push((membership, count));
        }
    }
    combos.sort_by(|a, b| b.1.cmp(&a.1));

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let mut c = themed_canvas(w, h, &opts);
        c.margin.left = 100.0;
        c.margin.bottom = 80.0;
        let nc = combos.len().min(20);
        let bar_area_h = c.plot_height() * 0.6;
        let dot_area_h = c.plot_height() * 0.4;
        let bar_w = c.plot_width() / nc as f64;
        let max_count = combos.iter().map(|(_, c)| *c).max().unwrap_or(1) as f64;
        for (ci, (membership, count)) in combos.iter().take(nc).enumerate() {
            let x = c.margin.left + ci as f64 * bar_w + bar_w * 0.15;
            let bw = bar_w * 0.7;
            let bh = (*count as f64 / max_count) * bar_area_h;
            c.add_rect(x, c.margin.top + bar_area_h - bh, bw, bh, PALETTE[0]);
            c.add_text(
                x + bw / 2.0,
                c.margin.top + bar_area_h - bh - 5.0,
                &count.to_string(),
                "middle",
                9.0,
            );
            // Dot matrix
            let dot_top = c.margin.top + bar_area_h + 10.0;
            for (si, &active) in membership.iter().enumerate() {
                let dy = dot_top + si as f64 * (dot_area_h / n as f64);
                let dx = x + bw / 2.0;
                c.add_circle(dx, dy + 5.0, 4.0, if active { "#333" } else { "#ddd" });
            }
        }
        // Set labels
        let dot_top = c.margin.top + bar_area_h + 10.0;
        for (si, (name, _)) in sets.iter().enumerate() {
            let y = dot_top + si as f64 * (dot_area_h / n as f64) + 9.0;
            c.add_text(c.margin.left - 5.0, y, name, "end", 10.0);
        }
        finish_themed_canvas(&mut c, &opts, "UpSet Plot");
        return Ok(Value::Str(c.render()));
    }

    let max_name = sets.iter().map(|(n, _)| n.len()).max().unwrap_or(4);
    let nc = combos.len().min(15);
    let max_count = combos.iter().map(|(_, c)| *c).max().unwrap_or(1);
    let mut out = String::from("  UpSet Plot\n");
    // Bar row
    out.push_str(&format!("  {:>w$}  ", "count", w = max_name));
    for (_, count) in combos.iter().take(nc) {
        let _bar_len = (*count as f64 / max_count as f64 * 5.0).ceil() as usize;
        out.push_str(&format!("{:>3} ", count));
    }
    out.push('\n');
    // Dot matrix
    for (si, (name, _)) in sets.iter().enumerate() {
        out.push_str(&format!("  {:>w$}  ", name, w = max_name));
        for (membership, _) in combos.iter().take(nc) {
            out.push_str(if membership[si] { " ●  " } else { " ·  " });
        }
        out.push('\n');
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 16. sequence_logo ───────────────────────────────────────────

pub(super) fn builtin_sequence_logo(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let seqs: Vec<String> = match &args[0] {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Str(s) => s.clone(),
                Value::DNA(seq) | Value::RNA(seq) | Value::Protein(seq) => seq.data.clone(),
                _ => String::new(),
            })
            .filter(|s| !s.is_empty())
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "sequence_logo() requires List of sequences",
                None,
            ))
        }
    };
    if seqs.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "sequence_logo() empty input",
            None,
        ));
    }
    let seq_len = seqs[0].len();
    let n = seqs.len() as f64;
    let is_dna = seqs[0].chars().all(|c| "ACGTUacgtu".contains(c));
    let alphabet_size: f64 = if is_dna { 4.0 } else { 20.0 };
    let max_bits = alphabet_size.log2();

    // Compute per-position information content
    let mut positions: Vec<Vec<(char, f64)>> = Vec::new(); // (char, height) per position
    for pos in 0..seq_len {
        let mut counts: HashMap<char, f64> = HashMap::new();
        for seq in &seqs {
            if let Some(ch) = seq.chars().nth(pos) {
                *counts.entry(ch.to_ascii_uppercase()).or_insert(0.0) += 1.0;
            }
        }
        let entropy: f64 = counts
            .values()
            .map(|&c| {
                let p = c / n;
                if p > 0.0 {
                    -p * p.log2()
                } else {
                    0.0
                }
            })
            .sum();
        let ic = max_bits - entropy;
        let mut chars: Vec<(char, f64)> =
            counts.iter().map(|(&ch, &c)| (ch, (c / n) * ic)).collect();
        chars.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        positions.push(chars);
    }

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", (seq_len as f64 * 30.0 + 80.0).min(1200.0));
        let h = get_opt_f64(&opts, "height", 200.0);
        let mut c = themed_canvas(w, h, &opts);
        let col_w = c.plot_width() / seq_len as f64;
        let y_scale = Scale {
            domain: (0.0, max_bits),
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };
        let char_colors: HashMap<char, &str> = [
            ('A', "#4caf50"),
            ('T', "#f44336"),
            ('U', "#f44336"),
            ('G', "#ff9800"),
            ('C', "#2196f3"),
        ]
        .into();
        for (pos, chars) in positions.iter().enumerate() {
            let x = c.margin.left + pos as f64 * col_w;
            let mut y_bottom = y_scale.map(0.0);
            for &(ch, height) in chars {
                let _y_top = y_scale.map(height);
                let letter_h = y_bottom - y_scale.map(height);
                if letter_h > 1.0 {
                    let color = char_colors.get(&ch).copied().unwrap_or("#333");
                    let font_size = (letter_h * 0.9).min(col_w * 0.9);
                    let escaped = format!("{ch}");
                    c.elements.push(format!(
                        r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="{font_size:.0}" font-family="monospace" font-weight="bold" fill="{color}">{escaped}</text>"#,
                        x + col_w / 2.0, y_bottom
                    ));
                }
                y_bottom -= letter_h;
            }
        }
        let dy = Scale {
            domain: (0.0, max_bits),
            range: (0.0, max_bits),
        };
        c.draw_y_axis(&dy, "bits");
        finish_themed_canvas(&mut c, &opts, "Sequence Logo");
        return Ok(Value::Str(c.render()));
    }

    // ASCII logo: show top char per position with height indicator
    let mut out = String::from("  Sequence Logo\n  ");
    for chars in positions.iter() {
        if let Some(&(ch, _)) = chars.last() {
            out.push(ch);
        } else {
            out.push(' ');
        }
    }
    out.push_str("\n  ");
    for chars in positions.iter() {
        let total_ic: f64 = chars.iter().map(|(_, h)| h).sum();
        let bar = if total_ic > max_bits * 0.75 {
            '█'
        } else if total_ic > max_bits * 0.5 {
            '▄'
        } else if total_ic > max_bits * 0.25 {
            '▂'
        } else {
            '▁'
        };
        out.push(bar);
    }
    out.push_str(&format!("\n  (n={}, len={})\n", seqs.len(), seq_len));
    write_output(&out);
    Ok(Value::Nil)
}

// ── 17. phylo_tree ──────────────────────────────────────────────

#[derive(Clone)]
pub(super) struct TreeNode {
    name: String,
    branch_len: f64,
    children: Vec<TreeNode>,
}

pub(super) fn parse_newick(s: &str) -> Result<TreeNode> {
    let s = s.trim().trim_end_matches(';');
    let (node, _) = parse_newick_node(s.as_bytes(), 0)?;
    Ok(node)
}

pub(super) fn parse_newick_node(data: &[u8], mut pos: usize) -> Result<(TreeNode, usize)> {
    let mut children = Vec::new();
    if pos < data.len() && data[pos] == b'(' {
        pos += 1; // skip '('
        loop {
            let (child, new_pos) = parse_newick_node(data, pos)?;
            children.push(child);
            pos = new_pos;
            if pos >= data.len() || data[pos] != b',' {
                break;
            }
            pos += 1; // skip ','
        }
        if pos < data.len() && data[pos] == b')' {
            pos += 1;
        }
    }
    // Parse name
    let mut name = String::new();
    while pos < data.len() && !b",):;".contains(&data[pos]) && data[pos] != b':' {
        name.push(data[pos] as char);
        pos += 1;
    }
    // Parse branch length
    let mut bl = 0.0;
    if pos < data.len() && data[pos] == b':' {
        pos += 1;
        let start = pos;
        while pos < data.len()
            && (data[pos].is_ascii_digit()
                || data[pos] == b'.'
                || data[pos] == b'-'
                || data[pos] == b'e'
                || data[pos] == b'E')
        {
            pos += 1;
        }
        if let Ok(v) = std::str::from_utf8(&data[start..pos])
            .unwrap_or("0")
            .parse::<f64>()
        {
            bl = v;
        }
    }
    Ok((
        TreeNode {
            name: name.trim().to_string(),
            branch_len: bl,
            children,
        },
        pos,
    ))
}

pub(super) fn builtin_phylo_tree(args: Vec<Value>) -> Result<Value> {
    let newick = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(BioLangError::type_error(
                "phylo_tree() requires Str (Newick format)",
                None,
            ))
        }
    };
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let root = parse_newick(&newick)?;

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let mut c = themed_canvas(w, h, &opts);
        c.margin.left = 40.0;
        c.margin.right = 100.0;
        let leaves = count_leaves(&root);
        let max_depth = max_tree_depth(&root);
        let ml = c.margin.left;
        let mt = c.margin.top;
        let pw = c.plot_width();
        let ph = c.plot_height();
        draw_tree_svg(&mut c, &root, 0.0, max_depth, 0, leaves, ml, mt, pw, ph);
        finish_themed_canvas(&mut c, &opts, "Phylogenetic Tree");
        return Ok(Value::Str(c.render()));
    }

    let mut out = String::from("  Phylogenetic Tree\n");
    render_tree_ascii(&root, &mut out, "", true);
    write_output(&out);
    Ok(Value::Nil)
}

pub(super) fn count_leaves(node: &TreeNode) -> usize {
    if node.children.is_empty() {
        1
    } else {
        node.children.iter().map(count_leaves).sum()
    }
}

pub(super) fn max_tree_depth(node: &TreeNode) -> f64 {
    if node.children.is_empty() {
        node.branch_len
    } else {
        node.branch_len
            + node
                .children
                .iter()
                .map(max_tree_depth)
                .fold(0.0f64, f64::max)
    }
}

pub(super) fn render_tree_ascii(node: &TreeNode, out: &mut String, prefix: &str, is_last: bool) {
    let connector = if prefix.is_empty() {
        ""
    } else if is_last {
        "└── "
    } else {
        "├── "
    };
    let label = if node.name.is_empty() {
        String::new()
    } else {
        format!(" {}", node.name)
    };
    let bl = if node.branch_len > 0.0 {
        format!(":{:.4}", node.branch_len)
    } else {
        String::new()
    };
    out.push_str(&format!("  {prefix}{connector}{label}{bl}\n"));
    let child_prefix = if prefix.is_empty() {
        String::new()
    } else if is_last {
        format!("{prefix}    ")
    } else {
        format!("{prefix}│   ")
    };
    for (i, child) in node.children.iter().enumerate() {
        render_tree_ascii(child, out, &child_prefix, i == node.children.len() - 1);
    }
}

pub(super) fn draw_tree_svg(
    c: &mut SvgCanvas,
    node: &TreeNode,
    x: f64,
    max_d: f64,
    leaf_idx: usize,
    total_leaves: usize,
    left: f64,
    top: f64,
    pw: f64,
    ph: f64,
) -> (f64, usize) {
    let x_pos = left + (x / max_d.max(0.001)) * pw;
    if node.children.is_empty() {
        let y_pos = top + (leaf_idx as f64 + 0.5) / total_leaves as f64 * ph;
        c.add_circle(x_pos, y_pos, 3.0, PALETTE[0]);
        if !node.name.is_empty() {
            c.add_text(x_pos + 8.0, y_pos + 4.0, &node.name, "start", 10.0);
        }
        return (y_pos, leaf_idx + 1);
    }
    let mut child_ys = Vec::new();
    let mut li = leaf_idx;
    for child in &node.children {
        let child_x = x + child.branch_len;
        let (cy, new_li) = draw_tree_svg(
            c,
            child,
            child_x,
            max_d,
            li,
            total_leaves,
            left,
            top,
            pw,
            ph,
        );
        let cx = left + (child_x / max_d.max(0.001)) * pw;
        c.add_line(x_pos, cy, cx, cy, "#333", 1.5);
        child_ys.push(cy);
        li = new_li;
    }
    if child_ys.len() >= 2 {
        let y_min = child_ys.iter().cloned().fold(f64::INFINITY, f64::min);
        let y_max = child_ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        c.add_line(x_pos, y_min, x_pos, y_max, "#333", 1.5);
    }
    let mid_y = child_ys.iter().sum::<f64>() / child_ys.len() as f64;
    (mid_y, li)
}

// ── 18. lollipop ────────────────────────────────────────────────

pub(super) fn builtin_upset_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let title = get_opt_str(&opts, "title", "UpSet Plot").to_string();
    let min_size = get_opt_f64(&opts, "min_size", 1.0) as usize;

    let sets: Vec<(String, HashSet<String>)> = match &args[0] {
        Value::Record(map) => map
            .iter()
            .map(|(n, v)| {
                let items: HashSet<String> = match v {
                    Value::List(l) => l.iter().map(|x| format!("{x}")).collect(),
                    _ => HashSet::new(),
                };
                (n.clone(), items)
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "upset_plot() requires Record of Lists",
                None,
            ))
        }
    };
    let n = sets.len();
    if n < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "upset_plot() needs >= 2 sets",
            None,
        ));
    }

    // Compute all intersection combinations (exclusive membership)
    let all_items: HashSet<String> = sets.iter().flat_map(|(_, s)| s.iter().cloned()).collect();
    let mut combos: Vec<(Vec<bool>, usize)> = Vec::new();
    for mask in 1..(1u32 << n) {
        let membership: Vec<bool> = (0..n).map(|i| mask & (1 << i) != 0).collect();
        let count = all_items
            .iter()
            .filter(|item| (0..n).all(|i| membership[i] == sets[i].1.contains(*item)))
            .count();
        if count >= min_size {
            combos.push((membership, count));
        }
    }
    combos.sort_by(|a, b| b.1.cmp(&a.1));

    // Set sizes
    let set_sizes: Vec<usize> = sets.iter().map(|(_, s)| s.len()).collect();
    let max_set_size = *set_sizes.iter().max().unwrap_or(&1);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 700.0);
        let h = get_opt_f64(&opts, "height", 500.0);
        let mut c = SvgCanvas::new(w, h);
        let left_bar_w = 100.0;
        c.margin.left = left_bar_w + 60.0;
        c.margin.bottom = 40.0;
        let nc = combos.len().min(25);
        let dot_area_h = n as f64 * 20.0 + 20.0;
        let bar_area_h = c.plot_height() - dot_area_h;
        let bar_w = if nc > 0 {
            c.plot_width() / nc as f64
        } else {
            c.plot_width()
        };
        let max_count = combos.iter().map(|(_, c)| *c).max().unwrap_or(1) as f64;
        let dot_top = c.margin.top + bar_area_h + 15.0;

        // Top: intersection size bars
        for (ci, (membership, count)) in combos.iter().take(nc).enumerate() {
            let x = c.margin.left + ci as f64 * bar_w + bar_w * 0.15;
            let bw = bar_w * 0.7;
            let bh = (*count as f64 / max_count) * bar_area_h * 0.9;
            c.add_rect(x, c.margin.top + bar_area_h - bh, bw, bh, "#333");
            c.add_text(
                x + bw / 2.0,
                c.margin.top + bar_area_h - bh - 5.0,
                &count.to_string(),
                "middle",
                9.0,
            );

            // Bottom: dot matrix
            let dx = x + bw / 2.0;
            let mut active_ys: Vec<f64> = Vec::new();
            for (si, &active) in membership.iter().enumerate() {
                let dy = dot_top + si as f64 * 20.0;
                c.add_circle(dx, dy, 5.0, if active { "#333" } else { "#ddd" });
                if active {
                    active_ys.push(dy);
                }
            }
            // Connect active dots with a line
            if active_ys.len() > 1 {
                let y_min = active_ys.iter().cloned().fold(f64::INFINITY, f64::min);
                let y_max = active_ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                c.add_line(dx, y_min, dx, y_max, "#333", 2.0);
            }
        }

        // Left: set size bars and labels
        for (si, (name, _)) in sets.iter().enumerate() {
            let y = dot_top + si as f64 * 20.0;
            c.add_text(c.margin.left - left_bar_w - 5.0, y + 4.0, name, "end", 10.0);
            let bar_len = (set_sizes[si] as f64 / max_set_size as f64) * left_bar_w * 0.9;
            c.add_rect(
                c.margin.left - bar_len - 2.0,
                y - 6.0,
                bar_len,
                12.0,
                PALETTE[si % PALETTE.len()],
            );
            c.add_text(
                c.margin.left - bar_len - 8.0,
                y + 4.0,
                &set_sizes[si].to_string(),
                "end",
                8.0,
            );
        }

        c.draw_title(&title);
        return Ok(Value::Str(c.render()));
    }

    // ASCII fallback
    let max_name = sets.iter().map(|(n, _)| n.len()).max().unwrap_or(4);
    let nc = combos.len().min(15);
    let max_count = combos.iter().map(|(_, c)| *c).max().unwrap_or(1);
    let mut out = format!("  {title}\n");
    out.push_str(&format!("  {:>w$}  ", "count", w = max_name));
    for (_, count) in combos.iter().take(nc) {
        let _bar_len = (*count as f64 / max_count as f64 * 5.0).ceil() as usize;
        out.push_str(&format!("{:>3} ", count));
    }
    out.push('\n');
    for (si, (name, _)) in sets.iter().enumerate() {
        out.push_str(&format!("  {:>w$}  ", name, w = max_name));
        for (membership, _) in combos.iter().take(nc) {
            out.push_str(if membership[si] { " ●  " } else { " ·  " });
        }
        out.push_str(&format!("  ({})\n", set_sizes[si]));
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 24. alignment_view (MSA) ────────────────────────────────────
