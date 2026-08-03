//! Quality metrics for sequencing file previews.
//!
//! The workbench already showed FASTQ and VCF files as tables of raw lines,
//! which tells you no more than `head` does. What people actually open these
//! files to learn — whether quality falls off the end of the read, how many
//! variants are indels, which chromosome is over-represented — needed a
//! separate FastQC or bcftools run. These compute that from the bytes already
//! read for the preview.
//!
//! Everything here works on a prefix of the file, because previews are capped.
//! The numbers are reported as a sample and never as file totals.

use serde::Serialize;

/// A named quantity worth stating outright rather than plotting.
#[derive(Clone, Serialize)]
pub struct PreviewFact {
    pub label: String,
    pub value: String,
}

impl PreviewFact {
    fn new(label: &str, value: impl Into<String>) -> Self {
        Self {
            label: label.to_string(),
            value: value.into(),
        }
    }
}

/// One plotted series. Values align with the chart's categories.
#[derive(Clone, Serialize)]
pub struct PreviewSeries {
    pub name: String,
    pub values: Vec<f64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewChart {
    pub title: String,
    /// `line` for a positional profile, `bar` for a distribution.
    pub kind: String,
    pub x_label: String,
    pub y_label: String,
    pub categories: Vec<String>,
    pub series: Vec<PreviewSeries>,
}

#[derive(Clone, Serialize, Default)]
pub struct PreviewMetrics {
    pub facts: Vec<PreviewFact>,
    pub charts: Vec<PreviewChart>,
}

impl PreviewMetrics {
    fn is_empty(&self) -> bool {
        self.facts.is_empty() && self.charts.is_empty()
    }
}

/// Longest read prefix to profile. Illumina reads are 50-300 bp; beyond that
/// this is long-read data and a per-base plot stops being readable.
const MAX_PROFILE_POSITIONS: usize = 320;

/// Decide the Phred offset from the range of quality characters observed.
///
/// The two encodings overlap, so neither end alone is enough: a file of all
/// `I` is Q40 under Phred+33 and Q9 under Phred+64, and both are plausible.
/// The decisive evidence is a character outside the other encoding's range —
/// below ';' rules out Phred+64, above 'J' rules out Phred+33. When the sample
/// sits in the overlap, Phred+33 wins because Phred+64 was retired in 2011.
fn quality_offset(min_char: u8, max_char: u8) -> (u8, &'static str) {
    if min_char < 59 {
        (33, "Phred+33 (Sanger / Illumina 1.8+)")
    } else if max_char > 74 {
        (64, "Phred+64 (Illumina 1.3-1.7)")
    } else {
        (33, "Phred+33 (Sanger / Illumina 1.8+)")
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Per-position quality, length distribution, and GC for a FASTQ sample.
pub fn fastq_metrics(text: &str) -> Option<PreviewMetrics> {
    let lines: Vec<&str> = text.lines().collect();
    // A truncated final record would report a wrong length, so only whole
    // four-line records with matching sequence and quality count.
    let records: Vec<(&str, &str)> = lines
        .chunks(4)
        .filter(|chunk| chunk.len() == 4 && chunk[0].starts_with('@'))
        .map(|chunk| (chunk[1], chunk[3]))
        .filter(|(sequence, quality)| sequence.len() == quality.len() && !sequence.is_empty())
        .collect();
    if records.is_empty() {
        return None;
    }

    let quality_bytes = || records.iter().flat_map(|(_, quality)| quality.bytes());
    let (offset, encoding) = quality_offset(
        quality_bytes().min().unwrap_or(b'!'),
        quality_bytes().max().unwrap_or(b'!'),
    );

    let mut position_totals = vec![0f64; MAX_PROFILE_POSITIONS];
    let mut position_counts = vec![0usize; MAX_PROFILE_POSITIONS];
    let mut quality_histogram = vec![0usize; 46];
    let mut lengths: Vec<usize> = Vec::with_capacity(records.len());
    let mut gc = 0usize;
    let mut known = 0usize;
    let mut ambiguous = 0usize;

    for (sequence, quality) in &records {
        lengths.push(sequence.len());
        for base in sequence.bytes() {
            match base.to_ascii_uppercase() {
                b'G' | b'C' => {
                    gc += 1;
                    known += 1;
                }
                b'A' | b'T' => known += 1,
                _ => ambiguous += 1,
            }
        }
        for (index, character) in quality.bytes().enumerate() {
            let score = character.saturating_sub(offset) as f64;
            if index < MAX_PROFILE_POSITIONS {
                position_totals[index] += score;
                position_counts[index] += 1;
            }
            let bucket = (score as usize).min(quality_histogram.len() - 1);
            quality_histogram[bucket] += 1;
        }
    }

    let profiled = position_counts
        .iter()
        .take_while(|count| **count > 0)
        .count();
    let per_position: Vec<f64> = (0..profiled)
        .map(|index| position_totals[index] / position_counts[index] as f64)
        .collect();
    let overall_mean = mean(&per_position);
    lengths.sort_unstable();
    let shortest = *lengths.first().unwrap_or(&0);
    let longest = *lengths.last().unwrap_or(&0);
    let total_bases = known + ambiguous;

    let mut metrics = PreviewMetrics::default();
    metrics
        .facts
        .push(PreviewFact::new("Reads sampled", records.len().to_string()));
    metrics.facts.push(PreviewFact::new("Encoding", encoding));
    metrics.facts.push(PreviewFact::new(
        "Read length",
        if shortest == longest {
            format!("{longest} bp")
        } else {
            format!("{shortest}-{longest} bp")
        },
    ));
    metrics.facts.push(PreviewFact::new(
        "Mean quality",
        format!("Q{overall_mean:.1}"),
    ));
    if total_bases > 0 {
        metrics.facts.push(PreviewFact::new(
            "GC content",
            format!("{:.1}%", gc as f64 / total_bases as f64 * 100.0),
        ));
    }
    if ambiguous > 0 {
        metrics.facts.push(PreviewFact::new(
            "Ambiguous bases",
            format!("{:.2}%", ambiguous as f64 / total_bases as f64 * 100.0),
        ));
    }

    // The single most useful FASTQ plot: quality decaying towards the far end
    // of the read is what tells you where to trim.
    metrics.charts.push(PreviewChart {
        title: "Mean quality by position".into(),
        kind: "line".into(),
        x_label: "Position in read (bp)".into(),
        y_label: "Phred score".into(),
        categories: (1..=per_position.len())
            .map(|index| index.to_string())
            .collect(),
        series: vec![PreviewSeries {
            name: "Mean quality".into(),
            values: per_position,
        }],
    });

    let highest = quality_histogram
        .iter()
        .rposition(|count| *count > 0)
        .unwrap_or(0);
    metrics.charts.push(PreviewChart {
        title: "Quality score distribution".into(),
        kind: "bar".into(),
        x_label: "Phred score".into(),
        y_label: "Bases".into(),
        categories: (0..=highest).map(|score| score.to_string()).collect(),
        series: vec![PreviewSeries {
            name: "Bases".into(),
            values: quality_histogram[..=highest]
                .iter()
                .map(|count| *count as f64)
                .collect(),
        }],
    });

    Some(metrics)
}

fn is_transition(reference: &str, alternate: &str) -> bool {
    matches!(
        (reference, alternate),
        ("A", "G") | ("G", "A") | ("C", "T") | ("T", "C")
    )
}

fn bump(counts: &mut Vec<(String, usize)>, key: String) {
    match counts.iter_mut().find(|(name, _)| *name == key) {
        Some((_, count)) => *count += 1,
        None => counts.push((key, 1)),
    }
}

/// Variant class counts, Ti/Tv, per-chromosome density, and filter status.
pub fn vcf_metrics(text: &str) -> Option<PreviewMetrics> {
    let mut samples: Vec<String> = Vec::new();
    let mut chromosomes: Vec<(String, usize)> = Vec::new();
    let mut filters: Vec<(String, usize)> = Vec::new();
    let mut snv = 0usize;
    let mut insertions = 0usize;
    let mut deletions = 0usize;
    let mut other = 0usize;
    let mut transitions = 0usize;
    let mut transversions = 0usize;
    let mut reference_build: Option<String> = None;
    let mut total = 0usize;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("##reference=") {
            reference_build = Some(rest.trim().to_string());
            continue;
        }
        if line.starts_with("#CHROM") {
            samples = line.split('\t').skip(9).map(str::to_string).collect();
            continue;
        }
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 5 {
            continue;
        }
        total += 1;
        bump(&mut chromosomes, fields[0].to_string());

        if let Some(filter) = fields.get(6) {
            let label = if filter.is_empty() || *filter == "." {
                "(none)".to_string()
            } else {
                (*filter).to_string()
            };
            bump(&mut filters, label);
        }

        let reference = fields[3];
        // A multi-allelic ALT is classified by its first allele; splitting them
        // properly is a job for the analysis, not the preview.
        let alternate = fields[4].split(',').next().unwrap_or("");
        if alternate.is_empty() || alternate == "." {
            other += 1;
        } else if reference.len() == 1 && alternate.len() == 1 {
            snv += 1;
            if is_transition(&reference.to_uppercase(), &alternate.to_uppercase()) {
                transitions += 1;
            } else {
                transversions += 1;
            }
        } else if alternate.len() > reference.len() {
            insertions += 1;
        } else if alternate.len() < reference.len() {
            deletions += 1;
        } else {
            other += 1;
        }
    }

    if total == 0 {
        return None;
    }

    let mut metrics = PreviewMetrics::default();
    metrics
        .facts
        .push(PreviewFact::new("Variants sampled", total.to_string()));
    if let Some(build) = reference_build {
        metrics.facts.push(PreviewFact::new("Reference", build));
    }
    metrics.facts.push(PreviewFact::new(
        "Samples",
        if samples.is_empty() {
            "none (sites-only)".to_string()
        } else {
            format!("{} ({})", samples.len(), samples.join(", "))
        },
    ));
    metrics
        .facts
        .push(PreviewFact::new("SNVs", snv.to_string()));
    metrics.facts.push(PreviewFact::new(
        "Indels",
        format!(
            "{} ({insertions} ins, {deletions} del)",
            insertions + deletions
        ),
    ));
    if transversions > 0 {
        // Whole-genome sequencing sits near 2.0 and exome near 3.0; a value far
        // below that is the classic signature of false-positive calls.
        metrics.facts.push(PreviewFact::new(
            "Ti/Tv",
            format!("{:.2}", transitions as f64 / transversions as f64),
        ));
    }
    if other > 0 {
        metrics
            .facts
            .push(PreviewFact::new("Other", other.to_string()));
    }

    metrics.charts.push(PreviewChart {
        title: "Variant classes".into(),
        kind: "bar".into(),
        x_label: "Class".into(),
        y_label: "Variants".into(),
        categories: vec![
            "SNV".into(),
            "Insertion".into(),
            "Deletion".into(),
            "Other".into(),
        ],
        series: vec![PreviewSeries {
            name: "Variants".into(),
            values: vec![
                snv as f64,
                insertions as f64,
                deletions as f64,
                other as f64,
            ],
        }],
    });

    chromosomes.truncate(40);
    metrics.charts.push(PreviewChart {
        title: "Variants per chromosome".into(),
        kind: "bar".into(),
        x_label: "Chromosome".into(),
        y_label: "Variants".into(),
        categories: chromosomes.iter().map(|(name, _)| name.clone()).collect(),
        series: vec![PreviewSeries {
            name: "Variants".into(),
            values: chromosomes.iter().map(|(_, count)| *count as f64).collect(),
        }],
    });

    if filters.len() > 1 {
        filters.truncate(12);
        metrics.charts.push(PreviewChart {
            title: "FILTER status".into(),
            kind: "bar".into(),
            x_label: "Filter".into(),
            y_label: "Variants".into(),
            categories: filters.iter().map(|(name, _)| name.clone()).collect(),
            series: vec![PreviewSeries {
                name: "Variants".into(),
                values: filters.iter().map(|(_, count)| *count as f64).collect(),
            }],
        });
    }

    Some(metrics)
}

/// Metrics for the given preview kind, or None when there is nothing to add.
pub fn metrics_for(kind: &str, text: &str) -> Option<PreviewMetrics> {
    let metrics = match kind {
        "fastq" => fastq_metrics(text),
        "vcf" => vcf_metrics(text),
        _ => None,
    }?;
    (!metrics.is_empty()).then_some(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fastq(reads: &[(&str, &str)]) -> String {
        reads
            .iter()
            .enumerate()
            .map(|(index, (sequence, quality))| format!("@read{index}\n{sequence}\n+\n{quality}\n"))
            .collect()
    }

    fn fact<'a>(metrics: &'a PreviewMetrics, label: &str) -> &'a str {
        &metrics
            .facts
            .iter()
            .find(|entry| entry.label == label)
            .unwrap_or_else(|| panic!("missing fact {label}"))
            .value
    }

    #[test]
    fn fastq_reports_reads_length_and_gc() {
        let text = fastq(&[("ACGT", "IIII"), ("GGCC", "IIII")]);
        let metrics = fastq_metrics(&text).expect("metrics");
        assert_eq!(fact(&metrics, "Reads sampled"), "2");
        assert_eq!(fact(&metrics, "Read length"), "4 bp");
        // ACGT has 2 of 4 and GGCC has 4 of 4, so 6 of 8.
        assert_eq!(fact(&metrics, "GC content"), "75.0%");
    }

    #[test]
    fn fastq_detects_phred33_and_scores_it() {
        // 'I' is 73, so Phred+33 gives Q40.
        let metrics = fastq_metrics(&fastq(&[("ACGT", "IIII")])).expect("metrics");
        assert!(fact(&metrics, "Encoding").starts_with("Phred+33"));
        assert_eq!(fact(&metrics, "Mean quality"), "Q40.0");
    }

    #[test]
    fn fastq_detects_phred64_from_characters_above_the_phred33_range() {
        // 'h' is 104, which no Phred+33 file can produce.
        let metrics = fastq_metrics(&fastq(&[("ACGT", "hhhh")])).expect("metrics");
        assert!(fact(&metrics, "Encoding").starts_with("Phred+64"));
        assert_eq!(fact(&metrics, "Mean quality"), "Q40.0");
    }

    #[test]
    fn fastq_profiles_quality_falling_off_the_read() {
        let metrics = fastq_metrics(&fastq(&[("ACGTACGT", "IIII####")])).expect("metrics");
        let profile = &metrics.charts[0].series[0].values;
        assert_eq!(profile.len(), 8);
        assert!(profile[0] > profile[7], "quality should decay: {profile:?}");
    }

    #[test]
    fn fastq_reports_variable_read_lengths_as_a_range() {
        let metrics =
            fastq_metrics(&fastq(&[("ACGT", "IIII"), ("ACGTAC", "IIIIII")])).expect("metrics");
        assert_eq!(fact(&metrics, "Read length"), "4-6 bp");
    }

    #[test]
    fn fastq_ignores_a_truncated_trailing_record() {
        let text = format!("{}@read2\nACGT\n", fastq(&[("ACGT", "IIII")]));
        let metrics = fastq_metrics(&text).expect("metrics");
        assert_eq!(fact(&metrics, "Reads sampled"), "1");
    }

    #[test]
    fn fastq_counts_ambiguous_bases() {
        let metrics = fastq_metrics(&fastq(&[("ACGN", "IIII")])).expect("metrics");
        assert_eq!(fact(&metrics, "Ambiguous bases"), "25.00%");
    }

    #[test]
    fn empty_fastq_has_no_metrics() {
        assert!(fastq_metrics("").is_none());
        assert!(metrics_for("fastq", "not a fastq").is_none());
    }

    const VCF_HEADER: &str = "##fileformat=VCFv4.2\n##reference=GRCh38\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tNA12878\n";

    #[test]
    fn vcf_classifies_variants_and_computes_ti_tv() {
        let text = format!(
            "{VCF_HEADER}\
             chr1\t100\t.\tA\tG\t50\tPASS\t.\tGT\t0/1\n\
             chr1\t200\t.\tC\tT\t50\tPASS\t.\tGT\t0/1\n\
             chr1\t300\t.\tA\tC\t50\tPASS\t.\tGT\t0/1\n\
             chr2\t400\t.\tAT\tA\t50\tPASS\t.\tGT\t0/1\n\
             chr2\t500\t.\tG\tGTT\t50\tPASS\t.\tGT\t0/1\n"
        );
        let metrics = vcf_metrics(&text).expect("metrics");
        assert_eq!(fact(&metrics, "Variants sampled"), "5");
        assert_eq!(fact(&metrics, "SNVs"), "3");
        assert_eq!(fact(&metrics, "Indels"), "2 (1 ins, 1 del)");
        // Two transitions (A>G, C>T) over one transversion (A>C).
        assert_eq!(fact(&metrics, "Ti/Tv"), "2.00");
        assert_eq!(fact(&metrics, "Reference"), "GRCh38");
    }

    #[test]
    fn vcf_lists_samples_and_recognises_sites_only_files() {
        let with_sample = format!("{VCF_HEADER}chr1\t1\t.\tA\tG\t1\tPASS\t.\tGT\t0/1\n");
        assert_eq!(
            fact(&vcf_metrics(&with_sample).unwrap(), "Samples"),
            "1 (NA12878)"
        );

        let sites_only =
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\nchr1\t1\t.\tA\tG\t1\tPASS\t.\n";
        assert_eq!(
            fact(&vcf_metrics(sites_only).unwrap(), "Samples"),
            "none (sites-only)"
        );
    }

    #[test]
    fn vcf_counts_variants_per_chromosome_in_file_order() {
        let text = format!(
            "{VCF_HEADER}\
             chr2\t1\t.\tA\tG\t1\tPASS\t.\tGT\t0/1\n\
             chr1\t1\t.\tA\tG\t1\tPASS\t.\tGT\t0/1\n\
             chr2\t2\t.\tA\tG\t1\tPASS\t.\tGT\t0/1\n"
        );
        let metrics = vcf_metrics(&text).expect("metrics");
        let chart = metrics
            .charts
            .iter()
            .find(|chart| chart.title == "Variants per chromosome")
            .expect("chromosome chart");
        assert_eq!(chart.categories, vec!["chr2", "chr1"]);
        assert_eq!(chart.series[0].values, vec![2.0, 1.0]);
    }

    #[test]
    fn vcf_takes_the_first_allele_of_a_multi_allelic_site() {
        let text = format!("{VCF_HEADER}chr1\t1\t.\tA\tG,T\t1\tPASS\t.\tGT\t1/2\n");
        assert_eq!(fact(&vcf_metrics(&text).unwrap(), "SNVs"), "1");
    }

    #[test]
    fn vcf_charts_filter_status_only_when_it_varies() {
        let uniform = format!("{VCF_HEADER}chr1\t1\t.\tA\tG\t1\tPASS\t.\tGT\t0/1\n");
        assert!(vcf_metrics(&uniform)
            .unwrap()
            .charts
            .iter()
            .all(|chart| chart.title != "FILTER status"));

        let mixed = format!(
            "{VCF_HEADER}\
             chr1\t1\t.\tA\tG\t1\tPASS\t.\tGT\t0/1\n\
             chr1\t2\t.\tA\tG\t1\tLowQual\t.\tGT\t0/1\n"
        );
        assert!(vcf_metrics(&mixed)
            .unwrap()
            .charts
            .iter()
            .any(|chart| chart.title == "FILTER status"));
    }

    #[test]
    fn header_only_vcf_has_no_metrics() {
        assert!(vcf_metrics(VCF_HEADER).is_none());
    }
}
