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
    id: usize,
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
            id: 0,
            name: name.trim().to_string(),
            branch_len: bl,
            children,
        },
        pos,
    ))
}

pub(super) fn builtin_phylo_tree(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let mut c = themed_canvas(w, h, &opts);
        if let Value::List(values) = &args[0] {
            render_phylo_facets(&mut c, values, &opts)?;
        } else {
            let newick = args[0].as_str().ok_or_else(|| {
                BioLangError::type_error(
                    "phylo_tree() requires a Newick Str or a List of Newick strings",
                    None,
                )
            })?;
            let root = parse_numbered_newick(newick)?;
            render_phylo_single(&mut c, &root, &opts)?;
        }
        let title = get_opt_str(&opts, "title", "Phylogenetic Tree");
        if !title.is_empty() {
            c.draw_title(title);
        }
        c.draw_subtitle(get_opt_str(&opts, "subtitle", ""));
        c.draw_caption(get_opt_str(&opts, "caption", ""));
        return Ok(Value::Str(c.render()));
    }

    let newick = args[0].as_str().ok_or_else(|| {
        BioLangError::type_error("phylo_tree() ASCII output requires a Newick Str", None)
    })?;
    let root = parse_newick(newick)?;

    let mut out = String::from("  Phylogenetic Tree\n");
    render_tree_ascii(&root, &mut out, "", true);
    write_output(&out);
    Ok(Value::Nil)
}

#[derive(Clone, Debug)]
struct PhyloPoint {
    id: usize,
    name: String,
    x: f64,
    y: f64,
    leaf: bool,
    min_y: f64,
    max_y: f64,
}

#[derive(Clone, Debug)]
struct PhyloLayout {
    points: HashMap<usize, PhyloPoint>,
    edges: Vec<(usize, usize)>,
    max_x: f64,
    leaf_count: usize,
}

fn parse_numbered_newick(newick: &str) -> Result<TreeNode> {
    let mut root = parse_newick(newick)?;
    let leaves = count_leaves(&root);
    let mut tip_id = 1usize;
    let mut internal_id = leaves + 1;
    assign_tree_ids(&mut root, &mut tip_id, &mut internal_id);
    Ok(root)
}

fn assign_tree_ids(node: &mut TreeNode, tip_id: &mut usize, internal_id: &mut usize) {
    if node.children.is_empty() {
        node.id = *tip_id;
        *tip_id += 1;
        return;
    }
    node.id = *internal_id;
    *internal_id += 1;
    for child in &mut node.children {
        assign_tree_ids(child, tip_id, internal_id);
    }
}

fn tree_height_edges(node: &TreeNode) -> usize {
    if node.children.is_empty() {
        0
    } else {
        1 + node
            .children
            .iter()
            .map(tree_height_edges)
            .max()
            .unwrap_or(0)
    }
}

fn collect_tip_names(node: &TreeNode, names: &mut Vec<String>) {
    if node.children.is_empty() {
        names.push(node.name.clone());
    } else {
        for child in &node.children {
            collect_tip_names(child, names);
        }
    }
}

fn requested_tip_order(
    root: &TreeNode,
    opts: &HashMap<String, Value>,
) -> Result<HashMap<String, usize>> {
    let mut leaves = Vec::new();
    collect_tip_names(root, &mut leaves);
    let order = match opts.get("tip_order") {
        Some(Value::List(items)) => items
            .iter()
            .map(|item| {
                item.as_str().map(str::to_string).ok_or_else(|| {
                    BioLangError::type_error("phylo_tree() tip_order must contain strings", None)
                })
            })
            .collect::<Result<Vec<_>>>()?,
        Some(_) => {
            return Err(BioLangError::type_error(
                "phylo_tree() tip_order must be a List",
                None,
            ))
        }
        None => leaves.clone(),
    };
    let mut expected = leaves;
    expected.sort();
    let mut observed = order.clone();
    observed.sort();
    if observed != expected {
        return Err(BioLangError::type_error(
            "phylo_tree() tip_order must name every leaf exactly once",
            None,
        ));
    }
    Ok(order
        .into_iter()
        .enumerate()
        .map(|(index, label)| (label, index))
        .collect())
}

fn build_phylo_layout(root: &TreeNode, opts: &HashMap<String, Value>) -> Result<PhyloLayout> {
    let tip_order = requested_tip_order(root, opts)?;
    let use_lengths = get_opt_str(opts, "branch_length", "scaled") != "none";
    let max_height = tree_height_edges(root) as f64;
    let mut points = HashMap::new();
    let mut edges = Vec::new();

    fn walk(
        node: &TreeNode,
        parent_x: f64,
        use_lengths: bool,
        max_height: f64,
        tip_order: &HashMap<String, usize>,
        points: &mut HashMap<usize, PhyloPoint>,
        edges: &mut Vec<(usize, usize)>,
    ) -> Result<(f64, f64, f64)> {
        let x = if use_lengths {
            parent_x + node.branch_len
        } else {
            max_height - tree_height_edges(node) as f64
        };
        if node.children.is_empty() {
            let y = *tip_order.get(&node.name).ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::NameError,
                    format!(
                        "phylo_tree() leaf '{}' is missing from tip_order",
                        node.name
                    ),
                    None,
                )
            })? as f64
                + 0.5;
            points.insert(
                node.id,
                PhyloPoint {
                    id: node.id,
                    name: node.name.clone(),
                    x,
                    y,
                    leaf: true,
                    min_y: y,
                    max_y: y,
                },
            );
            return Ok((y, y, y));
        }
        let mut child_ys = Vec::new();
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for child in &node.children {
            edges.push((node.id, child.id));
            let (child_y, child_min, child_max) =
                walk(child, x, use_lengths, max_height, tip_order, points, edges)?;
            child_ys.push(child_y);
            min_y = min_y.min(child_min);
            max_y = max_y.max(child_max);
        }
        let y = child_ys.iter().sum::<f64>() / child_ys.len().max(1) as f64;
        points.insert(
            node.id,
            PhyloPoint {
                id: node.id,
                name: node.name.clone(),
                x,
                y,
                leaf: false,
                min_y,
                max_y,
            },
        );
        Ok((y, min_y, max_y))
    }

    walk(
        root,
        0.0,
        use_lengths,
        max_height,
        &tip_order,
        &mut points,
        &mut edges,
    )?;
    let max_x = points
        .values()
        .map(|point| point.x)
        .fold(0.0f64, f64::max)
        .max(1.0);
    Ok(PhyloLayout {
        points,
        edges,
        max_x,
        leaf_count: tip_order.len(),
    })
}

fn phylo_bool(opts: &HashMap<String, Value>, key: &str, default: bool) -> bool {
    opts.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn svg_dash_line(
    canvas: &mut SvgCanvas,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    colour: &str,
    width: f64,
    line_type: &str,
) {
    match line_type {
        "dashed" | "2" => canvas.add_dashed_line(x1, y1, x2, y2, colour, width, 6.0),
        "dotted" | "3" => {
            canvas.add_patterned_line(x1, y1, x2, y2, colour, width, width, width * 3.0)
        }
        _ => canvas.add_line(x1, y1, x2, y2, colour, width),
    }
}

fn render_phylo_single(
    canvas: &mut SvgCanvas,
    root: &TreeNode,
    opts: &HashMap<String, Value>,
) -> Result<()> {
    let show_tip_labels = phylo_bool(opts, "show_tip_labels", true);
    let show_tip_points = phylo_bool(opts, "show_tip_points", true);
    let show_node_points = phylo_bool(opts, "show_node_points", false);
    let show_node_labels = phylo_bool(opts, "show_node_labels", false);
    let show_scale = phylo_bool(opts, "scale_axis", false);
    let layout_name = get_opt_str(opts, "layout", "rectangular").to_ascii_lowercase();
    let line_colour = get_opt_str(opts, "line_color", "#111111");
    let line_width = get_opt_f64(opts, "line_width", 1.2).clamp(0.3, 8.0);
    let line_type = opts
        .get("line_type")
        .map(|value| match value {
            Value::Int(number) => number.to_string(),
            _ => value.as_str().unwrap_or("solid").to_string(),
        })
        .unwrap_or_else(|| "solid".to_string());
    let layout = build_phylo_layout(root, opts)?;

    canvas.margin.left = get_opt_f64(opts, "left_padding", 30.0);
    canvas.margin.right = get_opt_f64(
        opts,
        "right_padding",
        if show_tip_labels { 70.0 } else { 30.0 },
    );
    canvas.margin.top = get_opt_f64(
        opts,
        "top_padding",
        if get_opt_str(opts, "title", "").is_empty() {
            22.0
        } else {
            50.0
        },
    );
    canvas.margin.bottom =
        get_opt_f64(opts, "bottom_padding", if show_scale { 45.0 } else { 22.0 });
    let left = canvas.margin.left;
    let top = canvas.margin.top;
    let width = canvas.plot_width();
    let height = canvas.plot_height();
    let domain_max = get_opt_f64(opts, "x_max", layout.max_x).max(layout.max_x);
    let x_expand = if layout_name == "circular" {
        0.0
    } else {
        get_opt_f64(opts, "x_expand", 0.0).clamp(0.0, 0.5)
    };
    let expanded_x = domain_max * x_expand;
    let x_scale = Scale {
        domain: (-expanded_x, domain_max + expanded_x),
        range: (left, left + width),
    };
    let y_scale = Scale {
        domain: (0.0, layout.leaf_count as f64),
        range: (top, top + height),
    };
    let position = |point: &PhyloPoint| -> (f64, f64) {
        if layout_name == "circular" {
            let radius = x_scale.map(point.x) - left;
            let max_radius = width.min(height) * 0.47;
            let scaled_radius = radius / width.max(1.0) * max_radius;
            let angle = -std::f64::consts::FRAC_PI_2
                + 2.0 * std::f64::consts::PI * point.y / layout.leaf_count.max(1) as f64;
            (
                left + width / 2.0 + scaled_radius * angle.cos(),
                top + height / 2.0 + scaled_radius * angle.sin(),
            )
        } else {
            (x_scale.map(point.x), y_scale.map(point.y))
        }
    };

    if layout_name != "circular" {
        if let Some(Value::List(highlights)) = opts.get("clade_highlights") {
            for highlight in highlights.iter() {
                let Value::Record(record) = highlight else {
                    continue;
                };
                let Some(node) = record.get("node").and_then(Value::as_int) else {
                    continue;
                };
                let Some(point) = layout.points.get(&(node as usize)) else {
                    continue;
                };
                let fill = record
                    .get("fill")
                    .and_then(Value::as_str)
                    .unwrap_or("#FFD700");
                let opacity = record
                    .get("opacity")
                    .and_then(Value::as_float)
                    .unwrap_or(0.45)
                    .clamp(0.0, 1.0);
                let x = x_scale.map(point.x) - 2.0;
                let descendant_max_x = layout
                    .points
                    .values()
                    .filter(|candidate| {
                        candidate.leaf && candidate.y >= point.min_y && candidate.y <= point.max_y
                    })
                    .map(|candidate| candidate.x)
                    .fold(point.x, f64::max);
                let highlight_right = x_scale.map(descendant_max_x) + 2.0;
                let y = y_scale.map((point.min_y - 0.48).max(0.0));
                let y2 = y_scale.map((point.max_y + 0.48).min(layout.leaf_count as f64));
                canvas.elements.push(format!(
                    r#"<rect x="{x:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{fill}" fill-opacity="{opacity:.3}" stroke="none" data-clade-node="{}"/>"#,
                    y.min(y2),
                    highlight_right - x,
                    (y2 - y).abs(),
                    point.id
                ));
            }
        }
    }

    if layout_name == "rectangular" {
        for point in layout.points.values().filter(|point| !point.leaf) {
            let x = x_scale.map(point.x);
            // A rectangular phylogram's vertical connector joins the positions
            // of this node's immediate children.  Descendant extrema are kept
            // separately for clade highlights; using them here stretches every
            // connector across the entire clade and does not match ape/ggtree.
            let mut child_ys = layout.edges.iter().filter_map(|(parent_id, child_id)| {
                (*parent_id == point.id).then(|| layout.points[child_id].y)
            });
            let Some(first_child_y) = child_ys.next() else {
                continue;
            };
            let (min_child_y, max_child_y) = child_ys.fold(
                (first_child_y, first_child_y),
                |(minimum, maximum), child_y| (minimum.min(child_y), maximum.max(child_y)),
            );
            let y1 = y_scale.map(min_child_y);
            let y2 = y_scale.map(max_child_y);
            svg_dash_line(canvas, x, y1, x, y2, line_colour, line_width, &line_type);
        }
    }
    for &(parent_id, child_id) in &layout.edges {
        let parent = &layout.points[&parent_id];
        let child = &layout.points[&child_id];
        let (px, py) = position(parent);
        let (cx, cy) = position(child);
        if layout_name == "rectangular" {
            svg_dash_line(canvas, px, cy, cx, cy, line_colour, line_width, &line_type);
        } else {
            svg_dash_line(canvas, px, py, cx, cy, line_colour, line_width, &line_type);
        }
    }

    if layout_name != "circular" {
        if let Some(Value::List(links)) = opts.get("taxa_links") {
            let labels = layout
                .points
                .values()
                .filter(|point| point.leaf)
                .map(|point| (point.name.as_str(), point))
                .collect::<HashMap<_, _>>();
            for link in links.iter() {
                let Value::Record(record) = link else {
                    continue;
                };
                let Some(from) = record.get("from").and_then(Value::as_str) else {
                    continue;
                };
                let Some(to) = record.get("to").and_then(Value::as_str) else {
                    continue;
                };
                let (Some(start), Some(end)) = (labels.get(from), labels.get(to)) else {
                    continue;
                };
                let (x1, y1) = position(start);
                let (x2, y2) = position(end);
                let colour = record
                    .get("color")
                    .and_then(Value::as_str)
                    .unwrap_or("#555555");
                let curvature = record
                    .get("curvature")
                    .and_then(Value::as_float)
                    .unwrap_or(0.35);
                let dashed = record
                    .get("dashed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let bend = width * curvature * 0.22;
                canvas.elements.push(format!(
                    r#"<path d="M {x1:.2} {y1:.2} C {:.2} {:.2}, {:.2} {:.2}, {x2:.2} {y2:.2}" fill="none" stroke="{colour}" stroke-width="1.3"{} data-taxa-link="{from},{to}"/>"#,
                    x1 + bend,
                    y1,
                    x2 + bend,
                    y2,
                    if dashed { " stroke-dasharray=\"5,5\"" } else { "" }
                ));
            }
        }
    }

    let tip_colour = get_opt_str(opts, "tip_color", PALETTE[0]);
    let tip_label_colour = get_opt_str(opts, "tip_label_color", canvas.theme.text_colour);
    let node_colour = get_opt_str(opts, "node_color", "#DAA520");
    let node_opacity = get_opt_f64(opts, "node_opacity", 0.5).clamp(0.0, 1.0);
    let tip_radius = get_opt_f64(opts, "tip_radius", 3.0).clamp(1.0, 10.0);
    let node_radius = get_opt_f64(opts, "node_radius", 6.0).clamp(1.0, 14.0);
    let tip_shape = get_opt_str(opts, "tip_shape", "circle");
    for point in layout.points.values() {
        let (x, y) = position(point);
        if point.leaf && show_tip_points {
            if tip_shape == "diamond" {
                canvas.add_polygon_with_opacity(
                    &[
                        (x, y - tip_radius),
                        (x + tip_radius, y),
                        (x, y + tip_radius),
                        (x - tip_radius, y),
                    ],
                    tip_colour,
                    1.0,
                );
            } else {
                canvas.add_circle_with_opacity(x, y, tip_radius, tip_colour, 1.0);
            }
        } else if !point.leaf && show_node_points {
            canvas.add_circle_with_opacity(x, y, node_radius, node_colour, node_opacity);
        }
        if point.leaf && show_tip_labels && layout_name != "circular" {
            canvas.add_text_styled(
                x + 6.0,
                y + 4.0,
                &point.name,
                "start",
                get_opt_f64(opts, "tip_label_size", 11.0),
                "normal",
                tip_label_colour,
            );
        }
        if show_node_labels && layout_name != "circular" {
            canvas.add_text(x + 5.0, y + 4.0, &point.id.to_string(), "start", 10.0);
        }
    }

    if layout_name != "circular" {
        if let Some(Value::List(labels)) = opts.get("clade_labels") {
            for label in labels.iter() {
                let Value::Record(record) = label else {
                    continue;
                };
                let Some(node) = record.get("node").and_then(Value::as_int) else {
                    continue;
                };
                let Some(point) = layout.points.get(&(node as usize)) else {
                    continue;
                };
                let text = record.get("label").and_then(Value::as_str).unwrap_or("");
                let colour = record
                    .get("color")
                    .and_then(Value::as_str)
                    .unwrap_or("#CC2222");
                let offset = record
                    .get("offset")
                    .and_then(Value::as_float)
                    .unwrap_or(0.8);
                let align = record
                    .get("align")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let descendant_max_x = layout
                    .points
                    .values()
                    .filter(|candidate| {
                        candidate.leaf && candidate.y >= point.min_y && candidate.y <= point.max_y
                    })
                    .map(|candidate| candidate.x)
                    .fold(point.x, f64::max);
                let bar_x = if align {
                    x_scale.map(layout.max_x + offset)
                } else {
                    x_scale.map(descendant_max_x + offset)
                };
                let y1 = y_scale.map(point.min_y);
                let y2 = y_scale.map(point.max_y);
                canvas.add_line(bar_x, y1, bar_x, y2, colour, 1.2);
                canvas.add_text_styled(
                    bar_x + 6.0,
                    (y1 + y2) / 2.0 + 4.0,
                    text,
                    "start",
                    11.0,
                    "normal",
                    colour,
                );
            }
        }
    }

    if show_scale && layout_name != "circular" {
        canvas.draw_x_axis_with_tick_domain(
            &x_scale,
            (0.0, domain_max),
            get_opt_str(opts, "x_label", ""),
        );
    }
    canvas.set_accessible_description(format!(
        "Phylogenetic tree with {} tips in {} layout; branch lengths {}.",
        layout.leaf_count,
        layout_name,
        if get_opt_str(opts, "branch_length", "scaled") == "none" {
            "suppressed"
        } else {
            "shown"
        }
    ));
    Ok(())
}

fn render_phylo_facets(
    canvas: &mut SvgCanvas,
    values: &[Value],
    opts: &HashMap<String, Value>,
) -> Result<()> {
    if values.is_empty() {
        return Err(BioLangError::type_error(
            "phylo_tree() requires at least one Newick tree",
            None,
        ));
    }
    let columns = get_opt_usize(opts, "columns", 4).clamp(1, 8);
    let rows = values.len().div_ceil(columns);
    // Match ggplot2's facet_wrap geometry rather than treating each facet as
    // a touching cell.  The explicit gutters are especially important for
    // dense 50- and 100-tip trees: without them the strip labels and branches
    // visually run into the neighbouring panel.
    canvas.margin.left = 11.0;
    canvas.margin.right = 8.0;
    canvas.margin.top = if get_opt_str(opts, "title", "").is_empty() {
        8.0
    } else {
        31.0
    };
    canvas.margin.bottom = 8.0;
    let column_gap = get_opt_f64(opts, "facet_column_gap", 12.0).clamp(0.0, 40.0);
    let row_gap = get_opt_f64(opts, "facet_row_gap", 8.0).clamp(0.0, 40.0);
    let panel_width =
        (canvas.plot_width() - column_gap * columns.saturating_sub(1) as f64) / columns as f64;
    let panel_height =
        (canvas.plot_height() - row_gap * rows.saturating_sub(1) as f64) / rows as f64;
    for (index, value) in values.iter().enumerate() {
        let newick = value.as_str().ok_or_else(|| {
            BioLangError::type_error("phylo_tree() tree List must contain strings", None)
        })?;
        let root = parse_numbered_newick(newick)?;
        let layout = build_phylo_layout(&root, &HashMap::new())?;
        let column = index % columns;
        let row = index / columns;
        let x0 = canvas.margin.left + column as f64 * (panel_width + column_gap);
        let y0 = canvas.margin.top + row as f64 * (panel_height + row_gap);
        let strip_h = 22.0;
        canvas.add_stroked_rect(x0, y0, panel_width, strip_h, "#D9D9D9", "#333333", 0.5);
        canvas.add_text(
            x0 + panel_width / 2.0,
            y0 + 16.0,
            &format!("Tree #{}", index + 1),
            "middle",
            12.0,
        );
        let left = x0 + 9.5;
        let top = y0 + strip_h + 2.5;
        let width = (panel_width - 18.75).max(1.0);
        let height = (panel_height - strip_h - 8.0).max(1.0);
        let x_scale = Scale {
            domain: (0.0, layout.max_x),
            range: (left, left + width),
        };
        let y_scale = Scale {
            domain: (0.0, layout.leaf_count as f64),
            // ggtree numbers tips from the bottom of a panel upward.  A
            // faceted multiPhylo plot therefore displays the traversal order
            // bottom-to-top, unlike BioLang's labelled single-tree view.
            range: (top + height, top),
        };
        for point in layout.points.values().filter(|point| !point.leaf) {
            let x = x_scale.map(point.x);
            let mut child_ys = layout.edges.iter().filter_map(|(parent_id, child_id)| {
                (*parent_id == point.id).then(|| layout.points[child_id].y)
            });
            let Some(first_child_y) = child_ys.next() else {
                continue;
            };
            let (min_child_y, max_child_y) = child_ys.fold(
                (first_child_y, first_child_y),
                |(minimum, maximum), child_y| (minimum.min(child_y), maximum.max(child_y)),
            );
            canvas.add_line(
                x,
                y_scale.map(min_child_y),
                x,
                y_scale.map(max_child_y),
                "#111111",
                0.9,
            );
        }
        for (parent_id, child_id) in layout.edges {
            let parent = &layout.points[&parent_id];
            let child = &layout.points[&child_id];
            canvas.add_line(
                x_scale.map(parent.x),
                y_scale.map(child.y),
                x_scale.map(child.x),
                y_scale.map(child.y),
                "#111111",
                0.9,
            );
        }
    }
    canvas.set_accessible_description(format!(
        "Faceted comparison of {} phylogenetic trees in {} columns.",
        values.len(),
        columns
    ));
    Ok(())
}

pub(super) fn count_leaves(node: &TreeNode) -> usize {
    if node.children.is_empty() {
        1
    } else {
        node.children.iter().map(count_leaves).sum()
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
