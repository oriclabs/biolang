use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug)]
pub(crate) struct TaskSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub columns: &'static str,
    pub minimum_columns: usize,
    pub default_method: &'static str,
    pub methods: &'static str,
    pub question: &'static str,
}

pub(crate) const TASKS: &[TaskSpec] = &[
    TaskSpec {
        id: "compare",
        title: "Compare two independent groups",
        columns: "control,treated",
        minimum_columns: 2,
        default_method: "welch",
        methods: "welch, student, mann_whitney, permutation_mean",
        question: "Do two independent groups differ?",
    },
    TaskSpec {
        id: "compare-many",
        title: "Compare several independent groups",
        columns: "control,low,high",
        minimum_columns: 2,
        default_method: "welch_anova",
        methods: "welch_anova, classical_anova, kruskal_wallis",
        question: "Do several independent groups differ?",
    },
    TaskSpec {
        id: "counts",
        title: "Test association in a count table",
        columns: "outcome_yes,outcome_no",
        minimum_columns: 2,
        default_method: "chi_square",
        methods: "chi_square, fisher",
        question: "Are two categorical variables associated?",
    },
    TaskSpec {
        id: "stratified",
        title: "Check odds-ratio homogeneity",
        columns: "a,b,c,d",
        minimum_columns: 4,
        default_method: "breslow_day_tarone_adjusted",
        methods: "breslow_day_tarone_adjusted",
        question: "Is one common odds ratio plausible across strata?",
    },
    TaskSpec {
        id: "relationship",
        title: "Study a numeric relationship",
        columns: "x,y",
        minimum_columns: 2,
        default_method: "linear",
        methods: "linear, pearson, spearman, kendall",
        question: "How are two numeric measurements related?",
    },
    TaskSpec {
        id: "paired",
        title: "Compare matched measurements",
        columns: "before,after",
        minimum_columns: 2,
        default_method: "paired_t",
        methods: "paired_t, paired_wilcoxon",
        question: "Did a measurement change within matched pairs?",
    },
    TaskSpec {
        id: "dose-response",
        title: "Study a dose-response trend",
        columns: "dose,outcome",
        minimum_columns: 2,
        default_method: "linear",
        methods: "linear, spearman",
        question: "Does the outcome change with dose?",
    },
    TaskSpec {
        id: "survival",
        title: "Summarize time-to-event data",
        columns: "time,event",
        minimum_columns: 2,
        default_method: "kaplan_meier",
        methods: "kaplan_meier",
        question: "What is the survival experience over time?",
    },
    TaskSpec {
        id: "meta",
        title: "Combine study estimates",
        columns: "effect,variance",
        minimum_columns: 2,
        default_method: "fixed",
        methods: "fixed, random",
        question: "What pooled effect is supported by several studies?",
    },
];

pub(crate) fn print_catalog() {
    println!("Guided statistics notebooks\n");
    println!(
        "Choose the scientific question first; BioLang will disclose, not hide, the method.\n"
    );
    for task in TASKS {
        println!("  {:<15} {}", task.id, task.title);
        println!("  {:<15} columns: {}", "", task.columns);
        println!("  {:<15} methods: {}", "", task.methods);
    }
    println!("\nExample:");
    println!(
        "  bl stats compare measurements.csv --columns control,treated --output comparison.bln"
    );
}

fn task(id: &str) -> Result<TaskSpec, String> {
    TASKS
        .iter()
        .copied()
        .find(|task| task.id == id)
        .ok_or_else(|| format!("unknown statistics task '{id}'"))
}

fn quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\r', "\\r")
            .replace('\n', "\\n")
    )
}

fn column(data: &str, name: &str) -> String {
    format!("{data}[{}]", quote(name))
}

fn method_option(method: &str) -> String {
    format!("{{method: {}}}", quote(method))
}

pub(crate) fn generate_notebook(
    task_id: &str,
    input: &Path,
    columns: &[String],
    requested_method: Option<&str>,
) -> Result<String, String> {
    let spec = task(task_id)?;
    if columns.len() < spec.minimum_columns {
        return Err(format!(
            "task '{}' needs at least {} columns (for example: {})",
            spec.id, spec.minimum_columns, spec.columns
        ));
    }
    if matches!(
        spec.id,
        "compare" | "relationship" | "paired" | "dose-response" | "survival" | "meta"
    ) && columns.len() != 2
    {
        return Err(format!("task '{}' needs exactly two columns", spec.id));
    }
    if spec.id == "stratified" && columns.len() != 4 {
        return Err("task 'stratified' needs exactly four columns: a,b,c,d".into());
    }

    let method = requested_method.unwrap_or(spec.default_method);
    if !spec
        .methods
        .split(", ")
        .any(|candidate| candidate == method)
    {
        return Err(format!(
            "method '{method}' is not available for '{}'; choose {}",
            spec.id, spec.methods
        ));
    }

    let data = "data";
    let call = match spec.id {
        "compare" => format!(
            "stat.compare_groups({}, {}, {})",
            column(data, &columns[0]), column(data, &columns[1]), method_option(method)
        ),
        "compare-many" => format!(
            "stat.compare_many([{}], {})",
            columns.iter().map(|name| column(data, name)).collect::<Vec<_>>().join(", "),
            method_option(method)
        ),
        "counts" => {
            let first = column(data, &columns[0]);
            let row = columns
                .iter()
                .map(|name| format!("{}[i]", column(data, name)))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "stat.count_association(range(0, len({first})) |> map(|i| [{row}]), {})",
                method_option(method)
            )
        }
        "stratified" => format!(
            "stat.stratified_association(range(0, len({a})) |> map(|i| [[{a}[i], {b}[i]], [{c}[i], {d}[i]]]))",
            a = column(data, &columns[0]), b = column(data, &columns[1]),
            c = column(data, &columns[2]), d = column(data, &columns[3])
        ),
        "relationship" => format!(
            "stat.numeric_relationship({}, {}, {})",
            column(data, &columns[0]), column(data, &columns[1]), method_option(method)
        ),
        "paired" => format!(
            "stat.paired_change({}, {}, {})",
            column(data, &columns[0]), column(data, &columns[1]), method_option(method)
        ),
        "dose-response" => format!(
            "stat.dose_response({}, {}, {})",
            column(data, &columns[0]), column(data, &columns[1]), method_option(method)
        ),
        "survival" => format!(
            "stat.survival_summary({}, {})",
            column(data, &columns[0]), column(data, &columns[1])
        ),
        "meta" => format!(
            "stat.meta_summary({}, {}, {})",
            column(data, &columns[0]), column(data, &columns[1]), method_option(method)
        ),
        _ => unreachable!(),
    };

    Ok(format!(
        "# {title}\n\n## Question\n\n{question}\n\nThe method is **{method}**. It is written explicitly below so another analyst can reproduce or change it. BioLang does not infer pairing, independence, study design, or experimental units.\n\n## Load and inspect the data\n\n```biolang\nimport \"statistics\" as stat\n\nlet data = read_csv({input})\n{{rows: nrow(data), columns: colnames(data)}}\n```\n\nConfirm that one row represents the intended observational unit and review missing values before interpreting a test.\n\n## Run the stated analysis\n\n```biolang\nlet result = {call}\nstat.show(result, {{detail: \"learning\", format: \"auto\"}})\n```\n\n## Inspect the full result\n\n```biolang\nresult\n```\n\nRead the effect estimate and interval before the p-value. Review `result.assumptions`, `result.alternatives`, and `result.reproducible_call`; the helper never changes the input data.\n",
        title = spec.title,
        question = spec.question,
        method = method,
        input = quote(&input.to_string_lossy()),
        call = call,
    ))
}

pub(crate) fn write_or_print(source: &str, output: Option<PathBuf>) -> Result<(), String> {
    match output {
        Some(path) => {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|error| {
                    if error.kind() == std::io::ErrorKind::AlreadyExists {
                        format!(
                            "refusing to overwrite '{}'; choose a new --output path or remove it explicitly",
                            path.display()
                        )
                    } else {
                        format!("could not create '{}': {error}", path.display())
                    }
                })?;
            file.write_all(source.as_bytes())
                .map_err(|error| format!("could not write '{}': {error}", path.display()))?;
            println!("Created {}", path.display());
            println!("Run it with: bl notebook {}", path.display());
        }
        None => print!("{source}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_explicit_welch_notebook() {
        let source = generate_notebook(
            "compare",
            Path::new("measurements.csv"),
            &["control".into(), "treated".into()],
            None,
        )
        .unwrap();
        assert!(source.contains(
            "stat.compare_groups(data[\"control\"], data[\"treated\"], {method: \"welch\"})"
        ));
        assert!(source.contains("does not infer pairing"));
    }

    #[test]
    fn rejects_an_incompatible_method() {
        let error = generate_notebook(
            "paired",
            Path::new("measurements.csv"),
            &["before".into(), "after".into()],
            Some("welch"),
        )
        .unwrap_err();
        assert!(error.contains("not available"));
    }

    #[test]
    fn stratified_rows_become_two_by_two_tables() {
        let source = generate_notebook(
            "stratified",
            Path::new("trials.csv"),
            &["a".into(), "b".into(), "c".into(), "d".into()],
            None,
        )
        .unwrap();
        assert!(source.contains("stat.stratified_association"));
        assert!(source.contains("[[data[\"a\"][i], data[\"b\"][i]]"));
    }

    #[test]
    fn quote_keeps_generated_notebooks_on_one_source_line() {
        assert_eq!(quote("line one\nline two"), "\"line one\\nline two\"");
    }

    #[test]
    fn output_creation_does_not_overwrite_an_existing_notebook() {
        let path = std::env::temp_dir().join(format!(
            "biolang-stats-guide-{}-{}.bln",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, "keep me").unwrap();
        let error = write_or_print("replace me", Some(path.clone())).unwrap_err();
        assert!(error.contains("refusing to overwrite"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep me");
        std::fs::remove_file(path).unwrap();
    }
}
