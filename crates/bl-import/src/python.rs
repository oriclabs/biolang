/// BioPython → BioLang converter.
/// Uses line-level pattern matching and indent-to-brace conversion.
/// Complex cases are marked with `# TODO:` for manual review.
use regex::{Captures, Regex};
use std::sync::LazyLock;

struct CallMap {
    pattern: Regex,
    replacement: &'static str,
}

impl CallMap {
    fn new(pat: &str, repl: &'static str) -> Self {
        Self {
            pattern: Regex::new(pat).unwrap_or_else(|e| panic!("bad regex '{pat}': {e}")),
            replacement: repl,
        }
    }

    fn apply(&self, line: &str) -> String {
        self.pattern
            .replace_all(line, |caps: &Captures| {
                let mut out = String::from(self.replacement);
                for i in 1..=caps.len().saturating_sub(1) {
                    out = out.replace(&format!("${i}"), caps.get(i).map_or("", |m| m.as_str()));
                }
                out
            })
            .into_owned()
    }
}

fn build_call_maps() -> Vec<CallMap> {
    vec![
        // ── Scanpy / AnnData ──────────────────────────────────────
        // AnnData subscript access → dot notation.  Must be early so
        // adata.obs["col"] is rewritten before generic patterns fire.
        CallMap::new(r#"(\w+)\.obs\[['"](\w+)['"]\]"#, "$1.obs.$2"),
        CallMap::new(r#"(\w+)\.var\[['"](\w+)['"]\]"#, "$1.var.$2"),
        CallMap::new(r#"(\w+)\.obsm\[['"](\w+)['"]\]"#, "$1.obsm.$2"),
        CallMap::new(r#"(\w+)\.obsp\[['"](\w+)['"]\]"#, "$1.obsp.$2"),
        CallMap::new(r#"(\w+)\.uns\[['"](\w+)['"]\]"#, "$1.uns.$2"),
        // AnnData property shortcuts
        CallMap::new(r"\b(\w+)\.n_obs\b", "n_cells($1)"),
        CallMap::new(r"\b(\w+)\.n_vars\b", "n_genes($1)"),
        CallMap::new(r"\b(\w+)\.obs_names\b", "$1.barcodes"),
        CallMap::new(r"\b(\w+)\.var_names\b", "$1.genes"),
        CallMap::new(r"\b(\w+)\.X\b", "$1.matrix"),
        // AnnData methods
        CallMap::new(r"(\w+)\.copy\(\)", "$1  # copy is implicit in BioLang"),
        CallMap::new(
            r"(\w+)\.write_h5ad\(([^)]+?)\)",
            "# TODO: write_h5ad — use write_csv($1, $2) for tabular export",
        ),
        // Data loading — specific (extra kwargs) before bare form
        CallMap::new(r"sc\.read_10x_mtx\(([^,)]+)[^)]*\)", "read_10x($1)"),
        CallMap::new(
            r"sc\.read_10x_h5\(([^,)]+)[^)]*\)",
            "read_10x($1)  # H5 format; use read_10x for MEX",
        ),
        CallMap::new(
            r"sc\.read_h5ad\(([^)]+?)\)",
            "load_h5ad($1)  # TODO: .h5ad not yet supported; use read_10x for MEX",
        ),
        CallMap::new(
            r"(?:anndata|ad)\.read_h5ad\(([^)]+?)\)",
            "load_h5ad($1)  # TODO: .h5ad not yet supported; use read_10x for MEX",
        ),
        // sc.external must come before sc.tl/sc.pp (longer prefix wins)
        CallMap::new(
            r#"sc\.external\.pp\.harmony_integrate\((\w+),\s*key\s*=\s*['"]?(\w+)['"]?[^)]*\)"#,
            "sc_integrate($1, $1.obs.$2)",
        ),
        CallMap::new(
            r"sc\.external\.tl\.phenograph\((\w+)[^)]*\)",
            "leiden($1)  # PhenoGraph ≈ Leiden",
        ),
        // sc.pp.* — specific (with keyword args) before bare form
        CallMap::new(
            r"sc\.pp\.normalize_total\((\w+),\s*target_sum\s*=\s*([^,)]+)[^)]*\)",
            "normalize_total($1, $2)  # in-place in Scanpy; assign result in BioLang",
        ),
        CallMap::new(
            r"sc\.pp\.normalize_total\((\w+)[^)]*\)",
            "normalize_total($1)  # in-place in Scanpy; assign result in BioLang",
        ),
        CallMap::new(
            r"sc\.pp\.highly_variable_genes\((\w+),\s*n_top_genes\s*=\s*([^,)]+)[^)]*\)",
            "highly_variable_genes($1, $2)  # in-place in Scanpy; assign result in BioLang",
        ),
        CallMap::new(
            r"sc\.pp\.highly_variable_genes\((\w+)[^)]*\)",
            "highly_variable_genes($1)  # in-place in Scanpy; assign result in BioLang",
        ),
        CallMap::new(
            r"sc\.pp\.pca\((\w+),\s*n_comps\s*=\s*([^,)]+)[^)]*\)",
            "pca($1, n_components=$2)",
        ),
        CallMap::new(r"sc\.pp\.pca\((\w+)[^)]*\)", "pca($1)"),
        CallMap::new(
            r"sc\.pp\.filter_cells\((\w+)[^)]*\)",
            "cell_qc($1)  # in-place in Scanpy; assign result in BioLang",
        ),
        CallMap::new(
            r"sc\.pp\.filter_genes\((\w+)[^)]*\)",
            "gene_qc($1)  # in-place in Scanpy; assign result in BioLang",
        ),
        CallMap::new(
            r"sc\.pp\.log1p\((\w+)[^)]*\)",
            "log1p_transform($1)  # in-place in Scanpy; assign result in BioLang",
        ),
        CallMap::new(r"sc\.pp\.scale\((\w+)[^)]*\)", "scale_matrix($1)"),
        CallMap::new(
            r#"sc\.pp\.combat\((\w+),\s*key\s*=\s*['"]?(\w+)['"]?[^)]*\)"#,
            "sc_integrate($1, $1.obs.$2)  # ComBat batch correction",
        ),
        CallMap::new(
            r"sc\.pp\.regress_out\((\w+)[^)]*\)",
            "# TODO: regress_out — no direct BioLang equiv",
        ),
        CallMap::new(r"sc\.pp\.scrublet\((\w+)[^)]*\)", "doublet_score($1)"),
        // sc.tl.* — specific before bare form
        CallMap::new(
            r"sc\.tl\.pca\((\w+),\s*n_comps\s*=\s*([^,)]+)[^)]*\)",
            "pca($1, n_components=$2)",
        ),
        CallMap::new(r"sc\.tl\.pca\((\w+)[^)]*\)", "pca($1)"),
        CallMap::new(
            r"sc\.tl\.neighbors\((\w+),\s*n_neighbors\s*=\s*([^,)]+)[^)]*\)",
            "knn_graph($1, k=$2)",
        ),
        CallMap::new(r"sc\.tl\.neighbors\((\w+)[^)]*\)", "knn_graph($1)"),
        CallMap::new(
            r"sc\.tl\.umap\((\w+),\s*n_components\s*=\s*([^,)]+)[^)]*\)",
            "umap($1, dims=$2)",
        ),
        CallMap::new(r"sc\.tl\.umap\((\w+)[^)]*\)", "umap($1)"),
        CallMap::new(
            r"sc\.tl\.leiden\((\w+),\s*resolution\s*=\s*([^,)]+)[^)]*\)",
            "leiden($1, resolution=$2)",
        ),
        CallMap::new(r"sc\.tl\.leiden\((\w+)[^)]*\)", "leiden($1)"),
        CallMap::new(
            r"sc\.tl\.louvain\((\w+),\s*resolution\s*=\s*([^,)]+)[^)]*\)",
            "louvain($1, resolution=$2)",
        ),
        CallMap::new(r"sc\.tl\.louvain\((\w+)[^)]*\)", "louvain($1)"),
        CallMap::new(
            r"sc\.tl\.diffmap\((\w+)[^)]*\)",
            "pca($1)  # TODO: diffusion map; pca used as approximation",
        ),
        CallMap::new(
            r"sc\.tl\.dpt\((\w+)[^)]*\)",
            "diffusion_pseudotime($1.obsm.X_diffmap, $1.obsp.distances, 0)",
        ),
        // score_genes_cell_cycle before score_genes (longer name wins)
        CallMap::new(
            r#"sc\.tl\.score_genes_cell_cycle\((\w+),\s*s_genes\s*=\s*([^,)]+),\s*g2m_genes\s*=\s*([^,)]+)[^)]*\)"#,
            "cell_cycle_score($1, $2, $3)",
        ),
        CallMap::new(
            r"sc\.tl\.score_genes\((\w+),\s*([^,)]+)[^)]*\)",
            "module_score($1, $2)",
        ),
        CallMap::new(
            r#"sc\.tl\.rank_genes_groups\((\w+),\s*groupby\s*=\s*['"]?(\w+)['"]?[^)]*\)"#,
            "diff_expr_grouped($1, group_by=$2)  # TODO: review output format",
        ),
        CallMap::new(
            r"sc\.tl\.rank_genes_groups\((\w+)[^)]*\)",
            "diff_expr_grouped($1)  # TODO: review output format",
        ),
        CallMap::new(
            r"sc\.tl\.tsne\((\w+)[^)]*\)",
            "umap($1)  # TODO: t-SNE; using UMAP as approximation",
        ),
        // sc.pl.* — with-args patterns before bare form.
        // Use lazy .+? for gene/color list args so commas inside lists
        // don't terminate the capture before groupby/color keyword.
        CallMap::new(
            r#"sc\.pl\.umap\((\w+),\s*color\s*=\s*['"]?(\w+)['"]?[^)]*\)"#,
            "scatter($1, color_by=$2)",
        ),
        CallMap::new(r"sc\.pl\.umap\((\w+)[^)]*\)", "scatter($1)  # UMAP plot"),
        CallMap::new(
            r#"sc\.pl\.tsne\((\w+),\s*color\s*=\s*['"]?(\w+)['"]?[^)]*\)"#,
            "scatter($1, color_by=$2)",
        ),
        CallMap::new(r"sc\.pl\.tsne\((\w+)[^)]*\)", "scatter($1)"),
        // violin/heatmap/dotplot with groupby before bare form
        CallMap::new(
            r#"sc\.pl\.violin\((\w+),\s*(.+?),\s*groupby\s*=\s*['"]?(\w+)['"]?[^)]*\)"#,
            "violin($1, genes=$2, group_by=$3)",
        ),
        CallMap::new(r"sc\.pl\.violin\((\w+),\s*([^)]+)\)", "violin($1, genes=$2)"),
        CallMap::new(
            r#"sc\.pl\.heatmap\((\w+),\s*(.+?),\s*groupby\s*=\s*['"]?(\w+)['"]?[^)]*\)"#,
            "heatmap($1, genes=$2, group_by=$3)",
        ),
        CallMap::new(
            r#"sc\.pl\.dotplot\((\w+),\s*(.+?),\s*groupby\s*=\s*['"]?(\w+)['"]?[^)]*\)"#,
            "dot_plot($1, genes=$2, group_by=$3)  # TODO: dot_plot not yet in BioLang",
        ),
        CallMap::new(
            r#"sc\.pl\.matrixplot\((\w+),\s*(.+?),\s*groupby\s*=\s*['"]?(\w+)['"]?[^)]*\)"#,
            "heatmap($1, genes=$2, group_by=$3)",
        ),
        CallMap::new(
            r"sc\.pl\.rank_genes_groups\((\w+)[^)]*\)",
            "# TODO: visualize diff expr results",
        ),
        CallMap::new(r"sc\.pl\.embedding\((\w+)[^)]*\)", "scatter($1)"),
        // ── SeqIO ──────────────────────────────────────────────────
        CallMap::new(
            r#"SeqIO\.parse\(([^,)]+),\s*['"]fasta['"]\)"#,
            "read_fasta($1)",
        ),
        CallMap::new(
            r#"SeqIO\.parse\(([^,)]+),\s*['"]fastq['"]\)"#,
            "read_fastq($1)",
        ),
        CallMap::new(
            r#"SeqIO\.parse\(([^,)]+),\s*['"]genbank['"]\)"#,
            "read_fasta($1)  # TODO: genbank→fasta conversion needed",
        ),
        CallMap::new(
            r#"SeqIO\.parse\(([^,)]+),\s*['"]gb['"]\)"#,
            "read_fasta($1)  # TODO: genbank→fasta conversion needed",
        ),
        CallMap::new(
            r#"SeqIO\.read\(([^,)]+),\s*['"]fasta['"]\)"#,
            "first(read_fasta($1))",
        ),
        CallMap::new(
            r#"SeqIO\.read\(([^,)]+),\s*['"]fastq['"]\)"#,
            "first(read_fastq($1))",
        ),
        CallMap::new(
            r#"SeqIO\.write\(([^,)]+),\s*([^,)]+),\s*['"]fasta['"]\)"#,
            "write_fasta($1, $2)",
        ),
        CallMap::new(
            r#"SeqIO\.write\(([^,)]+),\s*([^,)]+),\s*['"]fastq['"]\)"#,
            "write_fastq($1, $2)",
        ),
        CallMap::new(
            r#"SeqIO\.convert\(([^,)]+),\s*['"]fasta['"],\s*([^,)]+),\s*['"]fastq['"]\)"#,
            "write_fastq(read_fasta($1), $2)",
        ),
        // ── Seq constructors ───────────────────────────────────────
        CallMap::new(r"\bSeq\(([^)]+?)\)", "dna($1)"),
        CallMap::new(r"\bMutableSeq\(([^)]+?)\)", "dna($1)"),
        // ── SeqUtils ───────────────────────────────────────────────
        CallMap::new(r"\bGC\(([^)]+?)\)", "gc_content($1) * 100.0"),
        CallMap::new(
            r"\bGC_skew\(([^)]+?)\)",
            "gc_content($1)  # TODO: gc_skew() not yet available",
        ),
        CallMap::new(r"MeltingTemp\.Tm_Wallace\(([^)]+?)\)", "tm($1)"),
        CallMap::new(r"MeltingTemp\.Tm_GC\(([^)]+?)\)", "tm($1)"),
        CallMap::new(r"\bMT\.Tm_Wallace\(([^)]+?)\)", "tm($1)"),
        CallMap::new(r"\bcodon_usage_table\([^)]*\)", "codon_usage"),
        CallMap::new(
            r"\bCodonAdaptationIndex\b",
            "# TODO: CAI (codon adaptation index) not yet in BioLang",
        ),
        CallMap::new(
            r"ProteinAnalysis\(([^)]+?)\)",
            "$1  # TODO: ProtParam — use molecular_weight/isoelectric_point (coming soon)",
        ),
        // ── Seq method calls → BioLang builtins ───────────────────
        CallMap::new(r"(\w+)\.reverse_complement\(\)", "reverse_complement($1)"),
        CallMap::new(r"(\w+)\.complement\(\)", "complement($1)"),
        CallMap::new(r"(\w+)\.transcribe\(\)", "transcribe($1)"),
        CallMap::new(r"(\w+)\.translate\(\)", "translate($1)"),
        CallMap::new(
            r"(\w+)\.back_transcribe\(\)",
            "reverse_complement(transcribe($1))",
        ),
        // ── Pairwise alignment ────────────────────────────────────
        CallMap::new(
            r"pairwise2\.align\.globalxx\(([^,)]+),\s*([^)]+?)\)",
            r#"align($1, $2, mode="global")"#,
        ),
        CallMap::new(
            r"pairwise2\.align\.localxx\(([^,)]+),\s*([^)]+?)\)",
            r#"align($1, $2, mode="local")"#,
        ),
        CallMap::new(
            r"pairwise2\.align\.globalms\(([^,)]+),\s*([^,)]+),\s*([^,)]+),\s*([^,)]+),\s*([^,)]+),\s*[^)]+?\)",
            r#"align($1, $2, mode="global", match_score=$3, mismatch=$4, gap=$5)"#,
        ),
        CallMap::new(
            r"pairwise2\.align\.localms\(([^,)]+),\s*([^,)]+),\s*([^,)]+),\s*([^,)]+),\s*([^,)]+),\s*[^)]+?\)",
            r#"align($1, $2, mode="local", match_score=$3, mismatch=$4, gap=$5)"#,
        ),
        CallMap::new(
            r"\bPairwiseAligner\(\)",
            r#"# aligner config: use align(seq1, seq2, mode="global")"#,
        ),
        // ── Entrez ────────────────────────────────────────────────
        CallMap::new(
            r"Entrez\.esearch\(db=([^,)]+),\s*term=([^)]+?)\)",
            "ncbi_search($1, $2)",
        ),
        CallMap::new(
            r#"Entrez\.efetch\([^)]*db=['"]nucleotide['"][^)]*id=([^,)]+)[^)]*\)"#,
            "ncbi_fetch($1)",
        ),
        CallMap::new(
            r#"Entrez\.efetch\([^)]*db=['"]protein['"][^)]*id=([^,)]+)[^)]*\)"#,
            "ncbi_fetch($1)",
        ),
        CallMap::new(
            r#"Entrez\.efetch\([^)]*db=['"]gene['"][^)]*id=([^,)]+)[^)]*\)"#,
            "ncbi_gene($1)",
        ),
        CallMap::new(r"Entrez\.read\(([^)]+?)\)", "$1"),
        CallMap::new(
            r"Entrez\.email\s*=\s*(.+)",
            "# email: $1  # not needed in BioLang",
        ),
        // ── Motifs / PWM ──────────────────────────────────────────
        CallMap::new(r"motifs\.create\(([^)]+?)\)", "pwm($1)"),
        CallMap::new(
            r#"motifs\.parse\(([^,)]+),\s*['"]JASPAR['"][^)]*\)"#,
            "# TODO: JASPAR format not yet supported — use pwm() with your matrix",
        ),
        CallMap::new(r"(\w+)\.pssm\.search\(([^,)]+)\)", "pwm_scan($2, $1)"),
        // ── Restriction enzymes ───────────────────────────────────
        CallMap::new(
            r"(\w+)\.catalyze\(([^)]+?)\)",
            r#"restriction_sites($2, enzyme="$1")"#,
        ),
        CallMap::new(
            r"RestrictionBatch\(([^)]+?)\)",
            "# TODO: RestrictionBatch → call restriction_sites() per enzyme",
        ),
        // ── Record attribute shorthand (inside for loops) ─────────
        CallMap::new(r"\brecord\.id\b", "r.id"),
        CallMap::new(r"\brecord\.seq\b", "r.seq"),
        CallMap::new(r"\brecord\.description\b", "r.desc"),
        CallMap::new(r"\brecord\.name\b", "r.id"),
        // ── print → println ───────────────────────────────────────
        CallMap::new(
            r#"print\(f["'](.*?)["']\)"#,
            r#"println("$1")  # TODO: adjust f-string placeholders"#,
        ),
        CallMap::new(r"print\(([^)]+?)\)", "println($1)"),
        // ── os / pathlib ──────────────────────────────────────────
        CallMap::new(r"os\.path\.join\(([^)]+?)\)", "path_join($1)"),
        CallMap::new(r"os\.path\.exists\(([^)]+?)\)", "file_exists($1)"),
        CallMap::new(r"os\.path\.basename\(([^)]+?)\)", "path_basename($1)"),
        CallMap::new(r"os\.path\.dirname\(([^)]+?)\)", "path_dirname($1)"),
        CallMap::new(r"os\.makedirs\(([^)]+?),?\s*exist_ok=True\)", "mkdir($1)"),
        CallMap::new(r"os\.makedirs\(([^)]+?)\)", "mkdir($1)"),
        CallMap::new(r"os\.listdir\(([^)]+?)\)", "list_dir($1)"),
        CallMap::new(r"os\.path\.isfile\(([^)]+?)\)", "file_exists($1)"),
        CallMap::new(r"os\.path\.isdir\(([^)]+?)\)", "dir_exists($1)"),
        CallMap::new(r"Path\(([^)]+?)\)\.read_text\(\)", "read_file($1)"),
        CallMap::new(
            r"Path\(([^)]+?)\)\.write_text\(([^)]+?)\)",
            "write_file($1, $2)",
        ),
        // ── json ──────────────────────────────────────────────────
        CallMap::new(r"json\.loads\(([^)]+?)\)", "json_parse($1)"),
        CallMap::new(r"json\.dumps\(([^)]+?)\)", "json_dump($1)"),
        CallMap::new(r"json\.load\(([^)]+?)\)", "json_parse(read_file($1))"),
        CallMap::new(
            r"json\.dump\(([^,)]+),\s*([^)]+?)\)",
            "write_file($2, json_dump($1))",
        ),
        // ── re (regex) ────────────────────────────────────────────
        CallMap::new(r"re\.search\(([^,)]+),\s*([^)]+?)\)", "match_re($2, $1)"),
        CallMap::new(
            r"re\.findall\(([^,)]+),\s*([^)]+?)\)",
            "find_all_re($2, $1)",
        ),
        CallMap::new(
            r"re\.sub\(([^,)]+),\s*([^,)]+),\s*([^)]+?)\)",
            "replace_re($3, $1, $2)",
        ),
        CallMap::new(r"re\.match\(([^,)]+),\s*([^)]+?)\)", "match_re($2, $1)"),
        CallMap::new(
            r"re\.compile\(([^)]+?)\)",
            "$1  # TODO: pre-compile regex → use match_re/find_all_re",
        ),
        // ── numpy ─────────────────────────────────────────────────
        CallMap::new(r"\bnp\.array\(([^)]+?)\)", "matrix($1)"),
        CallMap::new(r"\bnp\.zeros\(([^)]+?)\)", "zeros($1)"),
        CallMap::new(r"\bnp\.ones\(([^)]+?)\)", "ones($1)"),
        CallMap::new(r"\bnp\.mean\(([^)]+?)\)", "mean($1)"),
        CallMap::new(r"\bnp\.std\(([^)]+?)\)", "stdev($1)"),
        CallMap::new(r"\bnp\.var\(([^)]+?)\)", "variance($1)"),
        CallMap::new(r"\bnp\.sum\(([^)]+?)\)", "sum($1)"),
        CallMap::new(r"\bnp\.max\(([^)]+?)\)", "max($1)"),
        CallMap::new(r"\bnp\.min\(([^)]+?)\)", "min($1)"),
        CallMap::new(r"\bnp\.median\(([^)]+?)\)", "median($1)"),
        CallMap::new(r"\bnp\.log2\(([^)]+?)\)", "log2($1)"),
        CallMap::new(r"\bnp\.log10\(([^)]+?)\)", "log10($1)"),
        CallMap::new(r"\bnp\.log\(([^)]+?)\)", "log($1)"),
        CallMap::new(r"\bnp\.exp\(([^)]+?)\)", "exp($1)"),
        CallMap::new(r"\bnp\.sqrt\(([^)]+?)\)", "sqrt($1)"),
        CallMap::new(r"\bnp\.abs\(([^)]+?)\)", "abs($1)"),
        CallMap::new(r"\bnp\.transpose\(([^)]+?)\)", "transpose($1)"),
        CallMap::new(r"\bnp\.dot\(([^,)]+),\s*([^)]+?)\)", "mat_mul($1, $2)"),
        CallMap::new(r"\bnp\.concatenate\(([^)]+?)\)", "concat($1)"),
        CallMap::new(r"\bnp\.unique\(([^)]+?)\)", "unique($1)"),
        CallMap::new(
            r"\bnp\.where\(([^)]+?)\)",
            "# TODO: np.where($1) → filter/map",
        ),
        CallMap::new(r"\bnp\.arange\(([^)]+?)\)", "range($1)"),
        CallMap::new(
            r"\bnp\.linspace\(([^,)]+),\s*([^,)]+),\s*([^)]+?)\)",
            "linspace($1, $2, $3)",
        ),
        // ── pandas ────────────────────────────────────────────────
        CallMap::new(r"\bpd\.read_csv\(([^)]+?)\)", "read_csv($1)"),
        CallMap::new(r"\bpd\.read_table\(([^)]+?)\)", "read_tsv($1)"),
        CallMap::new(r"\bpd\.DataFrame\(([^)]+?)\)", "table($1)"),
        CallMap::new(r"\bpd\.concat\(([^)]+?)\)", "concat($1)"),
        CallMap::new(
            r"\bpd\.merge\(([^,)]+),\s*([^,)]+)[^)]*\)",
            "join($1, $2)  # TODO: check merge keys",
        ),
        CallMap::new(
            r"(\w+)\.to_csv\(([^)]+?),\s*index=False[^)]*\)",
            "write_csv($1, $2)",
        ),
        CallMap::new(r"(\w+)\.to_csv\(([^)]+?)\)", "write_csv($1, $2)"),
        CallMap::new(r"(\w+)\.head\((\d+)\)", "head($1, $2)"),
        CallMap::new(r"(\w+)\.head\(\)", "head($1, 6)"),
        CallMap::new(r"(\w+)\.tail\((\d+)\)", "tail($1, $2)"),
        CallMap::new(r"(\w+)\.describe\(\)", "summary($1)"),
        CallMap::new(r"(\w+)\.dropna\(\)", "drop_na($1)"),
        CallMap::new(r"(\w+)\.fillna\(([^)]+?)\)", "fill_na($1, $2)"),
        CallMap::new(
            r#"(\w+)\.sort_values\(['"]([^'"]+)['"]\)"#,
            r#"sort_by($1, "$2")"#,
        ),
        CallMap::new(r"(\w+)\.groupby\(([^)]+?)\)", "group_by($1, $2)"),
        CallMap::new(
            r"(\w+)\.reset_index\([^)]*\)",
            "$1  # reset_index not needed in BioLang",
        ),
        CallMap::new(r"(\w+)\.shape\[0\]", "nrows($1)"),
        CallMap::new(r"(\w+)\.shape\[1\]", "ncols($1)"),
        CallMap::new(r"(\w+)\.columns\.tolist\(\)", "col_names($1)"),
        // ── string methods → pipe style ───────────────────────────
        CallMap::new(r"(\w+)\.strip\(\)", "trim($1)"),
        CallMap::new(r"(\w+)\.lstrip\(\)", "trim_start($1)"),
        CallMap::new(r"(\w+)\.rstrip\(\)", "trim_end($1)"),
        CallMap::new(r"(\w+)\.upper\(\)", "to_upper($1)"),
        CallMap::new(r"(\w+)\.lower\(\)", "to_lower($1)"),
        CallMap::new(r"(\w+)\.split\(([^)]+?)\)", "split($1, $2)"),
        CallMap::new(r"(\w+)\.startswith\(([^)]+?)\)", "starts_with($1, $2)"),
        CallMap::new(r"(\w+)\.endswith\(([^)]+?)\)", "ends_with($1, $2)"),
        CallMap::new(
            r"(\w+)\.replace\(([^,)]+),\s*([^)]+?)\)",
            "str_replace($1, $2, $3)",
        ),
        CallMap::new(r"'([^']*)'\s*\.\s*join\(([^)]+?)\)", "join($2, \"$1\")"),
        CallMap::new(r#""([^"]*)"\s*\.\s*join\(([^)]+?)\)"#, "join($2, \"$1\")"),
        // ── scipy.stats → BioLang stats ───────────────────────────
        CallMap::new(
            r"scipy\.stats\.ttest_ind\(([^,)]+),\s*([^)]+?)\)",
            "ttest($1, $2)",
        ),
        CallMap::new(
            r"scipy\.stats\.mannwhitneyu\(([^,)]+),\s*([^)]+?)\)",
            "wilcoxon($1, $2)",
        ),
        CallMap::new(
            r"scipy\.stats\.chi2_contingency\(([^)]+?)\)",
            "chi_square($1)",
        ),
        CallMap::new(
            r"scipy\.stats\.pearsonr\(([^,)]+),\s*([^)]+?)\)",
            "cor($1, $2)",
        ),
        CallMap::new(
            r"scipy\.stats\.spearmanr\(([^,)]+),\s*([^)]+?)\)",
            r#"cor($1, $2, method="spearman")"#,
        ),
        CallMap::new(
            r"scipy\.stats\.norm\b",
            "# TODO: scipy.stats.norm → use normal_dist() or stats builtins",
        ),
        // ── sklearn → BioLang ML ──────────────────────────────────
        CallMap::new(
            r"KMeans\(n_clusters=(\d+)[^)]*\)",
            "kmeans($1)  # TODO: call .fit() on data",
        ),
        CallMap::new(
            r"PCA\(n_components=(\d+)[^)]*\)",
            "pca  # TODO: pca(data, n_components=$1)",
        ),
        CallMap::new(
            r"TSNE\(n_components=(\d+)[^)]*\)",
            "tsne  # TODO: tsne(data, n_components=$1)",
        ),
        CallMap::new(r"UMAP\([^)]*\)", "umap  # TODO: umap(data)"),
        CallMap::new(
            r"RandomForestClassifier\([^)]*\)",
            "# TODO: RandomForest not in BioLang",
        ),
        CallMap::new(
            r"LinearRegression\(\)",
            "# TODO: use lm() for linear regression",
        ),
        // ── Common Python builtins ────────────────────────────────
        CallMap::new(r"\bsorted\(([^)]+?)\)", "sort($1)"),
        CallMap::new(r"\blist\(([^)]+?)\)", "to_list($1)"),
        CallMap::new(r"\bset\(([^)]+?)\)", "set($1)"),
        CallMap::new(r"\bdict\(\)", "{}"),
        CallMap::new(r"\bstr\(([^)]+?)\)", "to_string($1)"),
        CallMap::new(r"\bfloat\(([^)]+?)\)", "float($1)"),
        CallMap::new(r"\bint\(([^)]+?)\)", "int($1)"),
        CallMap::new(r"\babs\(([^)]+?)\)", "abs($1)"),
        CallMap::new(r"\benumerate\(([^)]+?)\)", "enumerate($1)"),
        CallMap::new(r"\bzip\(([^)]+?)\)", "zip($1)"),
        CallMap::new(
            r"\bsorted\(([^,)]+),\s*key=([^,)]+),\s*reverse=True\)",
            "sort_by_desc($1, $2)",
        ),
        CallMap::new(r"\bsorted\(([^,)]+),\s*reverse=True\)", "reverse(sort($1))"),
        CallMap::new(r"\bmap\(([^,)]+),\s*([^)]+?)\)", "$2 |> map($1)"),
        CallMap::new(r"\bfilter\(([^,)]+),\s*([^)]+?)\)", "$2 |> filter($1)"),
        CallMap::new(r"\bsum\(([^)]+?)\)", "sum($1)"),
        CallMap::new(r"\bmax\(([^)]+?)\)", "max($1)"),
        CallMap::new(r"\bmin\(([^)]+?)\)", "min($1)"),
        CallMap::new(r"\bround\(([^)]+?)\)", "round($1)"),
        CallMap::new(r#"\bopen\(([^,)]+),\s*['"]r['"]\)"#, "read_file($1)"),
        CallMap::new(r"\bopen\(([^)]+?)\)", "read_file($1)"),
        // ── scVelo / RNA velocity ─────────────────────────────────
        // Data loading
        CallMap::new(r"scv\.read\(([^,)]+)[^)]*\)", "read_10x($1)  # scVelo loom; use read_10x for MTX"),
        CallMap::new(r"scv\.read_loom\(([^)]+?)\)", "# TODO: loom format not supported; convert to MTX first"),
        // Preprocessing (specific before bare)
        CallMap::new(r"scv\.pp\.filter_and_normalize\((\w+)[^)]*\)", "cell_qc($1) |> normalize_total |> log1p_transform"),
        CallMap::new(r"scv\.pp\.moments\((\w+)[^)]*\)", "knn_graph($1)  # scVelo moments ≈ KNN smoothing"),
        // Tools (specific mode= before bare)
        CallMap::new(
            r"scv\.tl\.velocity\((\w+),\s*mode\s*=\s*[^,)]+[^)]*\)",
            "velocity_estimate($1.spliced, $1.unspliced)  # TODO: dynamical model",
        ),
        CallMap::new(r"scv\.tl\.velocity\((\w+)[^)]*\)", "velocity_estimate($1.spliced, $1.unspliced)"),
        CallMap::new(r"scv\.tl\.velocity_graph\((\w+)[^)]*\)", "knn_graph($1)  # velocity graph ≈ KNN"),
        CallMap::new(
            r"scv\.tl\.velocity_embedding\((\w+)[^)]*\)",
            "umap($1)  # TODO: velocity-projected UMAP",
        ),
        CallMap::new(
            r"scv\.tl\.latent_time\((\w+)[^)]*\)",
            "diffusion_pseudotime($1.obsm.X_diffmap, $1.obsp.distances, 0)",
        ),
        // Plotting
        CallMap::new(r"scv\.pl\.velocity_embedding_stream\((\w+)[^)]*\)", "scatter($1)  # TODO: velocity stream plot"),
        CallMap::new(
            r"scv\.pl\.velocity\((\w+),\s*var_names\s*=\s*([^,)]+)[^)]*\)",
            "scatter($1, color_by=$2)",
        ),
        // AnnData layers access (must be after obs/obsm already handled above)
        CallMap::new(r#"(\w+)\.layers\[['"]spliced['"]\]"#, "$1.spliced"),
        CallMap::new(r#"(\w+)\.layers\[['"]unspliced['"]\]"#, "$1.unspliced"),
        CallMap::new(r#"(\w+)\.layers\[['"](\w+)['"]\]"#, "$1.layers.$2"),
        // ── pySCENIC / GRN ───────────────────────────────────────
        CallMap::new(
            r"pyscenic\.grn\(([^,)]+),\s*([^)]+?)\)",
            "coexpression_network($1, $2)  # TODO: map TF names to gene indices",
        ),
        CallMap::new(
            r"ctx\.prune2df\([^)]*\)",
            "# TODO: motif pruning not in BioLang GRN — see import \"grn\" as grn",
        ),
        CallMap::new(
            r"ctx\.df2regulons\([^)]*\)",
            "# TODO: regulon construction — build manually from grn.top_targets()",
        ),
        CallMap::new(r"aucell\.create_rankings\(([^)]+?)\)", "# TODO: AUCell ranking — use module_score() per regulon"),
        CallMap::new(r"aucell\.enrichment\(([^,)]+),\s*([^)]+?)\)", "module_score($1, $2)  # AUCell ≈ module_score"),
        // ── CellChat / LIANA ─────────────────────────────────────
        CallMap::new(
            r"liana\.mt\.cellchat\((\w+)[^)]*\)",
            "lr_score($1.matrix, $1.obs.cluster, lr_pairs)  # use cc.score(obj) from \"cellchat\"",
        ),
        CallMap::new(r"liana\.rank_aggregate\((\w+)[^)]*\)", "cc.score($1)  # approximate via cellchat package"),
        // ── Scrublet / doublet detection ─────────────────────────
        // Most specific first (method chain), then constructors
        CallMap::new(
            r"scrublet\.Scrublet\(([^)]+?)\)\.scrub_doublets\(\)",
            "doublet_score($1)",
        ),
        CallMap::new(r"scr\.Scrublet\(([^)]+?)\)", "doublet_score($1)"),
        CallMap::new(r"scrub\.scrub_doublets\(\)", "doublet_score($1)  # TODO: bind to correct matrix var"),
        // ── Cell type annotation ──────────────────────────────────
        CallMap::new(
            r"celltypist\.annotate\((\w+),\s*model\s*=\s*([^,)]+)[^)]*\)",
            "reference_classify($1.matrix, $2.matrix, $2.labels)  # TODO: load model",
        ),
        CallMap::new(
            r"celltypist\.models\.download_models\([^)]*\)",
            "# TODO: download reference — provide ref_matrix and ref_labels manually",
        ),
        // ── VCF / variant analysis ────────────────────────────────
        // pysam
        CallMap::new(
            r"pysam\.VariantFile\(([^)]+?)\)",
            "vcf_parse($1)",
        ),
        CallMap::new(r"(\w+)\.fetch\(([^,)]+),\s*([^,)]+),\s*([^)]+?)\)", "$1 |> vcf_filter(chrom=$2, start=$3, end=$4)"),
        // cyvcf2
        CallMap::new(r"cyvcf2\.VCF\(([^)]+?)\)", "vcf_parse($1)"),
        CallMap::new(r#"(\w+)\.INFO\[['"](\w+)['"]\]"#, "$1.info.$2"),
        CallMap::new(r"(\w+)\.ALT", "$1.alt"),
        CallMap::new(r"(\w+)\.REF", "$1.ref"),
        CallMap::new(r"(\w+)\.QUAL", "$1.qual"),
        CallMap::new(r"(\w+)\.FILTER", "$1.filter"),
        // allel
        CallMap::new(
            r"allel\.read_vcf\(([^,)]+)[^)]*\)",
            "vcf_parse($1)",
        ),
        CallMap::new(r"allel\.GenotypeArray\(([^)]+?)\)", "$1.genotypes"),
        CallMap::new(
            r"allel\.ts_tv_ratio\(([^)]+?)\)",
            "titv_ratio($1)",
        ),
        CallMap::new(
            r"allel\.allele_counts_to_frequencies\(([^)]+?)\)",
            "allele_freq($1)",
        ),
        // ── Salmon / bulk RNA-seq ─────────────────────────────────
        // pydeseq2 — most specific (with keyword args) first
        CallMap::new(
            r"pydeseq2\.dds\.DeseqDataSet\(counts\s*=\s*([^,)]+),\s*metadata\s*=\s*([^,)]+)[^)]*\)",
            "parse_salmon($1, metadata=$2)  # TODO: use parse_featurecounts() or parse_salmon() + size_factors()",
        ),
        CallMap::new(
            r"DeseqDataSet\(counts\s*=\s*([^,)]+),\s*metadata\s*=\s*([^,)]+)[^)]*\)",
            "parse_salmon($1, metadata=$2)  # TODO: adjust to actual loader",
        ),
        CallMap::new(
            r"DeseqStats\(([^,)]+)[^)]*\)",
            "diff_expr($1)  # TODO: provide size_factors; import \"differential\" as de",
        ),
        CallMap::new(
            r"(\w+)\.summary\(\)",
            "variant_summary($1)  # TODO: verify context (DeseqStats or vcf)",
        ),
        CallMap::new(r"(\w+)\.results_df", "$1.results"),
        // ── Phylogenetics ─────────────────────────────────────────
        // ete3
        CallMap::new(
            r#"ete3\.Tree\(['"]([^'"]+)['"]\)"#,
            r#"nw_parse(read_file("$1"))"#,
        ),
        CallMap::new(r"ete3\.Tree\(([^)]+?)\)", "nw_parse($1)"),
        CallMap::new(
            r"(\w+)\.get_distance\(([^)]+?)\)",
            "patristic_distance($1, $2)",
        ),
        CallMap::new(r"(\w+)\.get_leaf_names\(\)", "tree_leaves($1)"),
        CallMap::new(r"(\w+)\.get_leaves\(\)", "tree_leaves($1)"),
        // dendropy
        CallMap::new(
            r#"dendropy\.Tree\.get\(path\s*=\s*['"]([^'"]+)['"],\s*schema\s*=\s*['"]newick['"][^)]*\)"#,
            r#"nw_parse(read_file("$1"))"#,
        ),
        CallMap::new(
            r"dendropy\.Tree\.get\(([^)]+?)\)",
            "nw_parse($1)  # TODO: extract Newick string first",
        ),
        CallMap::new(
            r"dendropy\.TaxonNamespace\([^)]*\)",
            "# TODO: taxon namespace not needed in BioLang",
        ),
        // Bio.Phylo (already handled in convert_import, but for inline parse calls)
        CallMap::new(
            r#"Phylo\.read\(['"]([^'"]+)['"]\s*,\s*['"]newick['"][^)]*\)"#,
            r#"nw_parse(read_file("$1"))"#,
        ),
        CallMap::new(
            r"Phylo\.read\(([^,)]+),\s*[^)]+\)",
            "nw_parse(read_file($1))",
        ),
        // ── deeptools / pybedtools ────────────────────────────────
        // pybedtools — specific before bare
        CallMap::new(
            r"pybedtools\.BedTool\(([^)]+?)\)",
            "read_bed($1)",
        ),
        CallMap::new(
            r"(\w+)\.merge\(d\s*=\s*(\d+)[^)]*\)",
            "merge_peaks($1, distance=$2)",
        ),
        CallMap::new(r"(\w+)\.merge\([^)]*\)", "merge_peaks($1)"),
        CallMap::new(
            r"(\w+)\.intersect\(([^,)]+)[^)]*\)",
            "bed_intersect($1, $2)",
        ),
        CallMap::new(r"(\w+)\.subtract\(([^,)]+)[^)]*\)", "bed_subtract($1, $2)"),
        CallMap::new(r"(\w+)\.sort\([^)]*\)", "sort_by($1, \"chrom\")"),
        // deeptools — plot functions become TODO (visual-only)
        CallMap::new(
            r"deeptools\.plotHeatmap\([^)]*\)",
            "# TODO: deeptools plotHeatmap — use heatmap() builtin",
        ),
        CallMap::new(
            r"deeptools\.plotProfile\([^)]*\)",
            "# TODO: deeptools plotProfile — use line_plot()",
        ),
        CallMap::new(
            r"deeptools\.bamCoverage\(([^,)]+)[^)]*\)",
            "depth($1)  # TODO: full bamCoverage options not supported",
        ),
        // ── QIIME2 / scikit-bio ───────────────────────────────────
        // skbio diversity
        CallMap::new(
            r"skbio\.diversity\.alpha_diversity\(([^,)]+),\s*([^,)]+),\s*metric\s*=\s*([^)]+?)\)",
            "alpha_diversity($1, method=$3)  # TODO: pass OTU table",
        ),
        CallMap::new(
            r"skbio\.diversity\.alpha_diversity\(([^,)]+),\s*([^)]+?)\)",
            "alpha_diversity($1)",
        ),
        CallMap::new(
            r"skbio\.diversity\.beta_diversity\(([^,)]+),\s*([^,)]+),\s*metric\s*=\s*([^)]+?)\)",
            "beta_diversity($1, method=$3)",
        ),
        CallMap::new(
            r"skbio\.diversity\.beta_diversity\(([^,)]+),\s*([^)]+?)\)",
            "beta_diversity($1)",
        ),
        // biom
        CallMap::new(
            r"biom\.load_table\(([^)]+?)\)",
            "read_tsv($1)  # TODO: biom format — convert to TSV first",
        ),
        CallMap::new(r"(\w+)\.subsample\(([^)]+?)\)", "rarefaction($1, depth=$2)"),
        CallMap::new(
            r"(\w+)\.norm\(([^)]+?)\)",
            "relative_abundance($1)  # biom norm ≈ relative_abundance",
        ),
        CallMap::new(r"(\w+)\.to_dataframe\(\)", "table($1)"),
        // qiime2
        CallMap::new(
            r"qiime2\.Artifact\.import_data\([^)]*\)",
            "# TODO: QIIME2 artifact import — use read_tsv() + OTU table functions",
        ),
        CallMap::new(
            r"qiime2\.Artifact\.load\(([^)]+?)\)",
            "# TODO: load QIIME2 artifact: $1",
        ),
    ]
}

struct ConvertState {
    output: String,
    todos: usize,
    block_stack: Vec<usize>, // indent levels of open brace blocks
    indent_unit: usize,
}

impl ConvertState {
    fn new() -> Self {
        Self {
            output: String::new(),
            todos: 0,
            block_stack: vec![],
            indent_unit: 4,
        }
    }

    fn push_line(&mut self, indent: usize, content: &str) {
        self.output.push_str(&" ".repeat(indent));
        self.output.push_str(content);
        self.output.push('\n');
        if content.contains("# TODO") {
            self.todos += 1;
        }
    }

    fn close_blocks_down_to(&mut self, indent: usize) {
        while self.block_stack.last().is_some_and(|&b| indent <= b) {
            let b = self.block_stack.pop().unwrap();
            self.output.push_str(&" ".repeat(b));
            self.output.push_str("}\n");
        }
    }

    fn close_all_blocks(&mut self) {
        while let Some(b) = self.block_stack.pop() {
            self.output.push_str(&" ".repeat(b));
            self.output.push_str("}\n");
        }
    }
}

pub fn convert(source: &str, filename: &str) -> String {
    let call_maps = build_call_maps();
    let mut state = ConvertState::new();

    state.output.push_str(&format!(
        "# Converted from Python: {filename}\n\
         # Review all `# TODO:` markers before running.\n\
         # Validate with: bl check <output>.bl\n\n"
    ));

    let lines: Vec<&str> = source.lines().collect();

    // Auto-detect indent unit from first indented line
    for line in &lines {
        let trimmed = line.trim_start();
        if !trimmed.is_empty() && line.starts_with(' ') {
            let spaces = line.len() - trimmed.len();
            if spaces > 0 {
                state.indent_unit = spaces;
                break;
            }
        }
    }

    let mut i = 0;
    while i < lines.len() {
        let raw = lines[i];
        let trimmed = raw.trim();
        let curr_indent = leading_spaces(raw);

        // ── Blank line ────────────────────────────────────────────
        if trimmed.is_empty() {
            state.output.push('\n');
            i += 1;
            continue;
        }

        // ── Multi-line docstrings ─────────────────────────────────
        if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
            let quote = if trimmed.starts_with("\"\"\"") {
                "\"\"\""
            } else {
                "'''"
            };
            let rest = trimmed.trim_start_matches(quote);
            // Single-line docstring
            if rest.contains(quote) {
                let doc = rest.trim_end_matches(quote).trim();
                state.close_blocks_down_to(curr_indent);
                state.push_line(curr_indent, &format!("# {doc}"));
                i += 1;
                continue;
            }
            // Multi-line: collect until closing triple-quote
            let mut parts = vec![rest.to_string()];
            i += 1;
            while i < lines.len() {
                let dl = lines[i].trim();
                if dl.contains(quote) {
                    let end = dl.trim_end_matches(quote).trim();
                    if !end.is_empty() {
                        parts.push(end.to_string());
                    }
                    i += 1;
                    break;
                }
                parts.push(dl.to_string());
                i += 1;
            }
            state.close_blocks_down_to(curr_indent);
            for part in &parts {
                if !part.is_empty() {
                    state.push_line(curr_indent, &format!("# {part}"));
                }
            }
            continue;
        }

        // ── Comments pass through ─────────────────────────────────
        if trimmed.starts_with('#') {
            state.close_blocks_down_to(curr_indent);
            state.push_line(curr_indent, trimmed);
            i += 1;
            continue;
        }

        // ── elif / else must be handled BEFORE block closing ──────
        if trimmed.starts_with("elif ") {
            // Close the if/elif body (its indent was curr_indent), but keep the block open
            // We pop the current block and replace with a `} else if {`
            if state.block_stack.last().is_some_and(|&b| b == curr_indent) {
                state.block_stack.pop();
                state.output.push_str(&" ".repeat(curr_indent));
                state.output.push_str("} ");
            }
            let cond_raw = trimmed
                .trim_start_matches("elif ")
                .trim_end_matches(':')
                .trim();
            let cond = transform_expr(cond_raw, &call_maps);
            state.output.push_str(&format!("else if {cond} {{\n"));
            state.block_stack.push(curr_indent);
            i += 1;
            continue;
        }

        if trimmed == "else:" {
            if state.block_stack.last().is_some_and(|&b| b == curr_indent) {
                state.block_stack.pop();
                state.output.push_str(&" ".repeat(curr_indent));
                state.output.push_str("} ");
            }
            state.output.push_str("else {\n");
            state.block_stack.push(curr_indent);
            i += 1;
            continue;
        }

        if trimmed.starts_with("except") {
            if state.block_stack.last().is_some_and(|&b| b == curr_indent) {
                state.block_stack.pop();
                state.output.push_str(&" ".repeat(curr_indent));
                state.output.push_str("} ");
            }
            state
                .output
                .push_str("# TODO: except — BioLang has no try/catch; use error return values {\n");
            state.block_stack.push(curr_indent);
            state.todos += 1;
            i += 1;
            continue;
        }

        if trimmed == "finally:" {
            if state.block_stack.last().is_some_and(|&b| b == curr_indent) {
                state.block_stack.pop();
                state.output.push_str(&" ".repeat(curr_indent));
                state.output.push_str("} ");
            }
            state.output.push_str("# TODO: finally {\n");
            state.block_stack.push(curr_indent);
            state.todos += 1;
            i += 1;
            continue;
        }

        // ── General block closing ─────────────────────────────────
        state.close_blocks_down_to(curr_indent);

        // ── import / from … import ────────────────────────────────
        if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
            let converted = convert_import(trimmed, &mut state.todos);
            if !converted.is_empty() {
                state.push_line(curr_indent, &converted);
            }
            i += 1;
            continue;
        }

        // ── class definition ──────────────────────────────────────
        if trimmed.starts_with("class ") {
            let name = trimmed
                .trim_start_matches("class ")
                .trim_end_matches(':')
                .split('(')
                .next()
                .unwrap_or("Unknown")
                .trim();
            state.push_line(
                curr_indent,
                &format!("# TODO: class {name} — BioLang has no classes; refactor to functions"),
            );
            state.todos += 1;
            // Still open a fake block to absorb the class body
            state.block_stack.push(curr_indent);
            i += 1;
            continue;
        }

        // ── def / async def ──────────────────────────────────────
        if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
            let inner = trimmed
                .trim_start_matches("async def ")
                .trim_start_matches("def ")
                .trim_end_matches(':')
                .trim();
            let inner = strip_type_hints(inner);
            let async_note = if trimmed.starts_with("async") {
                "  # async"
            } else {
                ""
            };
            let line = format!("fn {inner} {{{async_note}");
            state.push_line(curr_indent, &line);
            state.block_stack.push(curr_indent);
            i += 1;
            continue;
        }

        // ── for loop ──────────────────────────────────────────────
        if trimmed.starts_with("for ") && trimmed.ends_with(':') {
            let inner = trimmed
                .trim_start_matches("for ")
                .trim_end_matches(':')
                .trim();
            let inner = transform_expr(inner, &call_maps);
            // Rename common iterator variables
            let inner = inner
                .replace("record in ", "r in ")
                .replace("rec in ", "r in ")
                .replace("seq_record in ", "r in ");
            let line = format!("for {inner} {{");
            state.push_line(curr_indent, &line);
            state.block_stack.push(curr_indent);
            i += 1;
            continue;
        }

        // ── while loop ────────────────────────────────────────────
        if trimmed.starts_with("while ") && trimmed.ends_with(':') {
            let cond = trimmed
                .trim_start_matches("while ")
                .trim_end_matches(':')
                .trim();
            let cond = transform_expr(cond, &call_maps);
            state.push_line(curr_indent, &format!("while {cond} {{"));
            state.block_stack.push(curr_indent);
            i += 1;
            continue;
        }

        // ── if ────────────────────────────────────────────────────
        if trimmed.starts_with("if ") && trimmed.ends_with(':') {
            let cond = trimmed
                .trim_start_matches("if ")
                .trim_end_matches(':')
                .trim();
            let cond = transform_expr(cond, &call_maps);
            state.push_line(curr_indent, &format!("if {cond} {{"));
            state.block_stack.push(curr_indent);
            i += 1;
            continue;
        }

        // ── try ───────────────────────────────────────────────────
        if trimmed == "try:" {
            state.push_line(
                curr_indent,
                "# TODO: try — BioLang has no exceptions; use result/error pattern",
            );
            state.todos += 1;
            state.block_stack.push(curr_indent);
            i += 1;
            continue;
        }

        // ── with open(...) ────────────────────────────────────────
        if trimmed.starts_with("with open(") {
            static RE_WITH: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r"with open\(([^,)]+)[^)]*\)\s*as\s+(\w+)").unwrap());
            if let Some(caps) = RE_WITH.captures(trimmed) {
                let fname = &caps[1];
                let var = &caps[2];
                state.push_line(
                    curr_indent,
                    &format!(
                    "# with open({fname}) as {var}: → use read_fasta/read_csv/read_file directly"
                ),
                );
                state.block_stack.push(curr_indent);
            } else {
                state.push_line(curr_indent, &format!("# TODO: {trimmed}"));
                state.todos += 1;
                state.block_stack.push(curr_indent);
            }
            i += 1;
            continue;
        }

        // ── with (generic) ────────────────────────────────────────
        if trimmed.starts_with("with ") && trimmed.ends_with(':') {
            state.push_line(
                curr_indent,
                &format!("# TODO: with {} — rewrite as direct call", &trimmed[5..]),
            );
            state.todos += 1;
            state.block_stack.push(curr_indent);
            i += 1;
            continue;
        }

        // ── pass ──────────────────────────────────────────────────
        if trimmed == "pass" {
            state.push_line(curr_indent, "// empty");
            i += 1;
            continue;
        }

        // ── return ────────────────────────────────────────────────
        if trimmed.starts_with("return ") {
            let expr = transform_expr(trimmed.trim_start_matches("return ").trim(), &call_maps);
            state.push_line(curr_indent, &expr);
            i += 1;
            continue;
        }
        if trimmed == "return" {
            // `nil`, not `null` — BioLang has no `null`, so emitting it produced
            // a program that failed on an undefined variable at the first use.
            state.push_line(curr_indent, "nil");
            i += 1;
            continue;
        }

        // ── raise ─────────────────────────────────────────────────
        if trimmed.starts_with("raise ") {
            let msg = trimmed.trim_start_matches("raise ");
            state.push_line(
                curr_indent,
                &format!("# TODO: raise {msg} → use error() or return error value"),
            );
            state.todos += 1;
            i += 1;
            continue;
        }

        // ── assert ────────────────────────────────────────────────
        if trimmed.starts_with("assert ") {
            let cond = transform_expr(trimmed.trim_start_matches("assert "), &call_maps);
            state.push_line(
                curr_indent,
                &format!("# assert {cond}  # TODO: if !({cond}) {{ error(...) }}"),
            );
            i += 1;
            continue;
        }

        // ── break / continue ─────────────────────────────────────
        if trimmed == "break" || trimmed == "continue" {
            state.push_line(curr_indent, trimmed);
            i += 1;
            continue;
        }

        // ── List comprehension ────────────────────────────────────
        // [expr for x in iterable] or [expr for x in iterable if cond]
        {
            static LC_RE: LazyLock<Regex> = LazyLock::new(|| {
                Regex::new(r"^\[(.+)\s+for\s+(\w+)\s+in\s+(.+?)(?:\s+if\s+(.+))?\]$").unwrap()
            });
            if let Some(caps) = LC_RE.captures(trimmed) {
                let body_expr = transform_expr(caps.get(1).map_or("", |m| m.as_str()), &call_maps);
                let var = caps.get(2).map_or("x", |m| m.as_str());
                let iterable = transform_expr(caps.get(3).map_or("", |m| m.as_str()), &call_maps);
                let filter_part = caps
                    .get(4)
                    .map(|m| {
                        let cond = transform_expr(m.as_str(), &call_maps);
                        format!(" |> filter(fn({var}) -> {cond})")
                    })
                    .unwrap_or_default();
                let out = format!("{iterable}{filter_part} |> map(fn({var}) -> {body_expr})");
                // Check if this is on the RHS of an assignment
                state.push_line(curr_indent, &out);
                i += 1;
                continue;
            }
        }

        // ── Assignment: x = expr  (but not ==, +=, etc.) ─────────
        {
            static AUG_RE: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r"^\w[\w.]*\s*[+\-*/%&|^]=").unwrap());
            static ASSIGN_RE: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r"^([a-zA-Z_]\w*(?:\[.*?\])?)\s*=\s*(.+)$").unwrap());

            if AUG_RE.is_match(trimmed) {
                // x += y → let x = x + y (simple augmented assignment)
                let transformed = transform_augmented(trimmed, &call_maps);
                state.push_line(curr_indent, &transformed);
                i += 1;
                continue;
            }

            if let Some(caps) = ASSIGN_RE.captures(trimmed) {
                let varname = caps.get(1).map_or("", |m| m.as_str());
                let expr = caps.get(2).map_or("", |m| m.as_str()).trim();
                // Skip if it's a comparison (the original had ==, we'd have matched `x = `)
                // Also skip attribute assignment like `obj.attr = val` → # TODO
                if varname.contains('.') {
                    let expr_t = transform_expr(expr, &call_maps);
                    state.push_line(
                        curr_indent,
                        &format!("# TODO: attribute set {varname} = {expr_t}"),
                    );
                    state.todos += 1;
                } else {
                    let expr_t = transform_expr(expr, &call_maps);
                    // Detect if it's a type annotation: `x: Type = val`
                    if varname.contains(':') {
                        let name = varname.split(':').next().unwrap_or(varname).trim();
                        state.push_line(curr_indent, &format!("let {name} = {expr_t}"));
                    } else {
                        state.push_line(curr_indent, &format!("let {varname} = {expr_t}"));
                    }
                }
                i += 1;
                continue;
            }
        }

        // ── Standalone expression (function call, etc.) ───────────
        {
            let out = transform_expr(trimmed, &call_maps);
            state.push_line(curr_indent, &out);
        }

        i += 1;
    }

    // Close any remaining open blocks
    state.close_all_blocks();

    // Summary footer
    if state.todos > 0 {
        state.output.push_str(&format!(
            "\n# Conversion complete: {} TODO item(s) require manual attention.\n",
            state.todos
        ));
    } else {
        state
            .output
            .push_str("\n# Conversion complete: no TODO items — review output before running.\n");
    }

    state.output
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn leading_spaces(line: &str) -> usize {
    let mut count = 0;
    for ch in line.chars() {
        match ch {
            ' ' => count += 1,
            '\t' => count += 4,
            _ => break,
        }
    }
    count
}

/// Apply all call-map regex substitutions to an expression string.
fn transform_expr(expr: &str, maps: &[CallMap]) -> String {
    // Python's `and`, `or` and `not` are spelled the same way in BioLang, so
    // they need no substitution — there were three here that replaced each with
    // itself, which said the opposite of what they did.
    let mut result = expr
        .replace("None", "nil") // BioLang has no `null`
        .replace("True", "true")
        .replace("False", "false")
        // BioLang has no `is`, so `x is None` came out as a parse error while the
        // converter reported nothing to review. Identity and equality part ways
        // in Python for objects, but `is` is overwhelmingly written against None,
        // where the two agree. Longest form first, or `is not` loses its `not`.
        .replace(" is not ", " != ")
        .replace(" is ", " == ")
        .replace("**", "^") // Python power → BioLang
        .replace("//", "/") // integer div → regular div (approximate)
        .replace("f\"", "\"") // strip f-string prefix (content stays, {} stay)
        .replace("f'", "'");

    for map in maps {
        result = map.apply(&result);
    }
    result
}

/// Handle augmented assignment: x += 1 → let x = x + 1
fn transform_augmented(line: &str, maps: &[CallMap]) -> String {
    let re = Regex::new(r"^(\w+)\s*([+\-*/%])=\s*(.+)$").unwrap();
    if let Some(caps) = re.captures(line) {
        let var = &caps[1];
        let op = &caps[2];
        let rhs = transform_expr(&caps[3], maps);
        return format!("let {var} = {var} {op} {rhs}");
    }
    transform_expr(line, maps)
}

/// Strip Python type hints from function signature: `foo(x: int, y: str = "a")` → `foo(x, y = "a")`
fn strip_type_hints(sig: &str) -> String {
    // State machine: skip characters inside `: TypeAnnotation` until `,`, `=`, or `)`
    let mut result = String::new();
    let mut in_ann = false;
    let mut depth = 0i32; // bracket/paren nesting inside annotations

    for ch in sig.chars() {
        match ch {
            ':' if !in_ann && depth == 0 => {
                in_ann = true; // enter annotation, drop the colon
            }
            '[' | '(' if in_ann => {
                depth += 1; // nested brackets inside annotation — keep skipping
            }
            ']' | ')' if in_ann && depth > 0 => {
                depth -= 1;
            }
            ',' if in_ann && depth == 0 => {
                in_ann = false;
                result.push(ch);
            }
            '=' if in_ann && depth == 0 => {
                in_ann = false;
                result.push(' ');
                result.push('=');
            }
            ')' if in_ann && depth == 0 => {
                in_ann = false;
                result.push(ch);
            }
            _ if in_ann => { /* skip annotation body */ }
            _ => result.push(ch),
        }
    }
    result
}

/// Convert a Python import line to a BioLang comment or import statement.
fn convert_import(line: &str, todos: &mut usize) -> String {
    // Mapping: (substring to detect, BioLang output, is_supported)
    let mappings: &[(&str, &str, bool)] = &[
        ("Bio.SeqIO",        "# SeqIO → read_fasta(), read_fastq(), write_fasta(), write_fastq() are builtins", true),
        ("Bio.Seq",          "# Bio.Seq → dna(), rna(), protein() constructors are builtins", true),
        ("Bio.SeqUtils",     "# SeqUtils → gc_content(), tm(), codon_usage() are builtins", true),
        ("Bio.SeqRecord",    "# SeqRecord → FASTA/FASTQ records have .id .seq .desc fields", true),
        ("Bio.Entrez",       "# Entrez → ncbi_search(), ncbi_fetch(), ncbi_gene(), ncbi_pubmed() are builtins", true),
        ("Bio.pairwise2",    "# pairwise2 → align() builtin (mode=\"global\"/\"local\")", true),
        ("Bio.Align",        "# Bio.Align → align() builtin", true),
        ("Bio.motifs",       "# motifs → pwm(), pwm_scan(), motif_find() are builtins", true),
        ("Bio.Restriction",  "# Restriction → restriction_sites() is a builtin", true),
        ("Bio.PopGen",       "# PopGen → hardy_weinberg(), fst() are builtins", true),
        ("Bio.Blast",        "# TODO: Bio.Blast not supported — use ncbi_search() for remote BLAST", false),
        ("Bio.PDB",          "# TODO: Bio.PDB not supported — 3D structure analysis coming soon", false),
        ("Bio.Phylo",        "# Bio.Phylo: use import \"phylo\" as ph — nw_parse(), tree_leaves(), patristic_distance() are builtins", true),
        ("Bio.codonalign",   "# TODO: Bio.codonalign not supported — Ka/Ks coming soon", false),
        ("Bio.Graphics",     "# TODO: Bio.Graphics not supported — use bio_plots builtins", false),
        ("Bio.KEGG",         "# KEGG → kegg_pathway(), kegg_compound() are builtins", true),
        ("Bio.ExPASy",       "# ExPASy → uniprot_entry() builtin", true),
        ("Bio.SwissProt",    "# TODO: Bio.SwissProt flat-file parser not supported", false),
        ("Bio",              "# Bio imports: most functions are BioLang builtins", true),
        ("scanpy",           "# scanpy: use singlecell builtins + import \"singlecell\" as sc", true),
        ("anndata",          "# anndata: AnnData object maps to BioLang Record", true),
        ("scvelo",           "# scvelo: use velocity_estimate() builtin + import \"velocity\" as vel", true),
        ("pyscenic",         "# pySCENIC: use grn builtins + import \"grn\" as grn", true),
        ("liana",            "# liana: use cellchat package — import \"cellchat\" as cc", true),
        ("cellchat",         "# cellchat: import \"cellchat\" as cc — lr_score(), lr_aggregate() are builtins", true),
        ("scrublet",         "# scrublet: use doublet_score() builtin", true),
        ("celltypist",       "# celltypist: use reference_classify() builtin or import \"celltypes\" as ct", true),
        ("numpy",            "# TODO: numpy not available — use matrix(), table(), stats builtins (mean/stdev/sum/etc.)", false),
        ("pandas",           "# TODO: pandas not available — use read_csv(), table(), filter(), sort_by(), group_by()", false),
        ("matplotlib",       "# TODO: matplotlib not available — use bar(), scatter(), heatmap(), line_plot()", false),
        ("seaborn",          "# TODO: seaborn not available — use BioLang plot builtins", false),
        ("scipy.stats",      "# scipy.stats → ttest(), wilcoxon(), chi_square(), anova(), fisher_exact() are builtins", true),
        ("scipy",            "# TODO: scipy not fully available — check BioLang stats/signal builtins", false),
        ("sklearn",          "# TODO: sklearn not available — use kmeans(), pca(), tsne(), umap(), lm() builtins", false),
        ("statsmodels",      "# TODO: statsmodels not available — use BioLang stats builtins", false),
        ("os.path",          "# os.path → path_join(), file_exists(), path_basename() are builtins", true),
        ("os",               "# os → file_exists(), list_dir(), mkdir(), read_file(), write_file() are builtins", true),
        ("sys",              "# sys → use env() for environment variables", true),
        ("pathlib",          "# pathlib → use read_file(), write_file(), file_exists() builtins", true),
        ("json",             "# json → json_parse(), json_dump() are builtins", true),
        ("csv",              "# csv → read_csv(), write_csv() are builtins", true),
        ("re",               "# re → match_re(), find_all_re(), replace_re() are builtins", true),
        ("collections",      "# collections → BioLang has native dict/list/set operations", true),
        ("itertools",        "# itertools → map/filter/reduce/zip/enumerate/flatten are builtins", true),
        ("argparse",         "# TODO: argparse not available — use env() for environment variables", false),
        ("subprocess",       "# TODO: subprocess → use shell() builtin to run external commands", false),
        ("multiprocessing",  "# TODO: multiprocessing → use await_all() for parallel tasks", false),
        ("threading",        "# TODO: threading → use await_all() for concurrent tasks", false),
        ("logging",          "# logging → use println() for output", true),
        ("math",             "# math → sqrt(), log(), exp(), sin(), cos(), floor(), ceil() are builtins", true),
        ("random",           "# random → random(), set_seed() are builtins", true),
        ("typing",           "# typing annotations stripped — BioLang is dynamically typed", true),
        ("dataclasses",      "# TODO: dataclasses → use BioLang records/tables", false),
        ("datetime",         "# datetime → now(), date_diff() are builtins", true),
        ("glob",             "# glob → list_dir() with pattern matching", true),
        ("shutil",           "# shutil → copy_file(), move_file() are builtins", true),
        ("tqdm",             "# tqdm → BioLang prints progress for long operations", true),
        // ── Tier-1 bioinformatics libraries ──────────────────────────────────────
        ("pysam",            "# pysam: vcf_parse(), vcf_filter() are builtins; BAM → read_bam(), depth()", true),
        ("cyvcf2",           "# cyvcf2: vcf_parse() is a builtin — use import \"variants\" as v", true),
        ("pyvcf",            "# pyvcf: vcf_parse() is a builtin — use import \"variants\" as v", true),
        ("allel",            "# allel: vcf_parse(), titv_ratio(), allele_freq(), variant_summary() are builtins", true),
        ("pydeseq2",         "# pydeseq2: use parse_salmon()/parse_featurecounts() + size_factors() + diff_expr(); import \"rnaseq\" as rna", true),
        ("salmon",           "# salmon output: use parse_salmon() builtin — import \"rnaseq\" as rna", true),
        ("ete3",             "# ete3: nw_parse(), tree_leaves(), patristic_distance() are builtins — import \"phylo\" as ph", true),
        ("dendropy",         "# dendropy: nw_parse(), tree_leaves(), patristic_distance() are builtins — import \"phylo\" as ph", true),
        ("pybedtools",       "# pybedtools: read_bed(), merge_peaks(), bed_intersect(), bed_subtract() are builtins — import \"chipseq\" as chip", true),
        ("deeptools",        "# deeptools: depth(), merge_peaks(), frip_score(), tss_enrichment() are builtins; plot functions → use BioLang plot builtins", true),
        ("qiime2",           "# qiime2: alpha_diversity(), beta_diversity(), rarefaction(), relative_abundance() are builtins — import \"microbiome\" as mb", true),
        ("skbio",            "# scikit-bio: alpha_diversity(), beta_diversity() builtins; import \"microbiome\" as mb", true),
        ("biom",             "# biom: read OTU table via read_tsv(); rarefaction(), relative_abundance() are builtins", true),
    ];

    for (fragment, output, supported) in mappings {
        if line.contains(fragment) {
            if !supported {
                *todos += 1;
            }
            return output.to_string();
        }
    }

    // Unknown import → pass through as comment
    format!("# {line}  # TODO: check if equivalent builtins exist")
}
