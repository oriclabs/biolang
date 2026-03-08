# SQLite & Notifications

BioLang includes built-in SQLite support and notification builtins so your
pipelines can persist results and alert you when they finish — no external
tools required.

## SQLite

Bioinformatics workflows constantly produce tabular results: QC metrics, variant
counts, sample manifests. SQLite gives you a zero-config embedded database to
store, query, and compare results across runs.

### Opening a Database

```
# File-based (creates if missing)
let db = sqlite("project_results.db")

# In-memory (temporary, fast)
let scratch = sqlite()
```

### Querying

`sql()` executes any SQL statement. SELECT queries return a Table; write
statements return the number of affected rows. Use `?` placeholders for
parameterized queries.

```
# Create a table
sql(db, "CREATE TABLE IF NOT EXISTS qc (
  sample TEXT PRIMARY KEY,
  total_reads INTEGER,
  pass_reads INTEGER,
  pass_rate REAL
)")

# Insert data
sql(db, "INSERT INTO qc VALUES (?, ?, ?, ?)",
    ["tumor_01", 5000000, 4850000, 97.0])

# Query — returns a Table
let results = sql(db, "SELECT * FROM qc WHERE pass_rate > ?", [95.0])
print(results)
```

Since `sql()` returns a standard BioLang Table, you can pipe it directly:

```
sql(db, "SELECT symbol, chrom, start, end FROM genes WHERE chrom = ?", ["chr17"])
  |> filter(|g| g.start > 40000000)
  |> sort_by(|g| g.start)
  |> print()
```

### Bulk Insert

`sql_insert()` inserts an entire Table or list of records in a single
transaction. This is much faster than individual INSERT statements.

```
# From a Table (e.g., read from TSV)
let stats = read_tsv("variant_stats.tsv")
sql_insert(db, "variants", stats)

# From a list of records
let samples = [
  {id: "S1", depth: 42.3, mapped_pct: 98.1},
  {id: "S2", depth: 38.7, mapped_pct: 97.5},
]
let inserted = sql_insert(db, "qc_results", samples)
print(str(inserted) + " rows inserted")
```

### Metadata

```
# List all tables
sql_tables(db)
# ["qc", "variants", "qc_results"]

# Inspect a table's schema
sql_schema(db, "qc")
#  cid | name        | type    | notnull | pk
# -----+-------------+---------+---------+-----
#  0   | sample      | TEXT    | false   | true
#  1   | total_reads | INTEGER | false   | false
#  2   | pass_reads  | INTEGER | false   | false
#  3   | pass_rate   | REAL    | false   | false
```

### Pipeline Example

Store QC results from every sample run, then query across all runs:

```
let db = sqlite("lab_results.db")

sql(db, "CREATE TABLE IF NOT EXISTS qc (
  sample TEXT, run_date TEXT, total INTEGER, passing INTEGER, rate REAL
)")

pipeline sample_qc(sample_id, fastq_path) {
  stage stats {
    let reads = read_fastq(fastq_path) |> collect()
    let total = len(reads)
    let passing = reads |> filter(|r| mean_phred(r.quality) >= 25) |> len()
    let rate = passing / total * 100.0

    sql(db, "INSERT INTO qc VALUES (?, date('now'), ?, ?, ?)",
        [sample_id, total, passing, rate])

    {total: total, passing: passing, rate: rate}
  }
}

# After processing many samples, query trends
let low_quality = sql(db,
  "SELECT sample, rate FROM qc WHERE rate < ? ORDER BY rate",
  [95.0])
print("Samples below 95% pass rate:")
print(low_quality)
```

### SQLite Builtins Summary

| Builtin | Returns | Description |
|---------|---------|-------------|
| `sqlite(path?)` | DbHandle | Open/create database |
| `sql(db, query, params?)` | Table or Int | Execute SQL |
| `sql_insert(db, table, data)` | Int | Bulk insert (transactional) |
| `sql_tables(db)` | List | List table names |
| `sql_schema(db, table)` | Table | Column metadata |
| `sql_close(db)` | Nil | Close connection (optional) |
| `is_db(value)` | Bool | Type check |

---

## Notifications

Long-running pipelines (alignment, variant calling, cohort analysis) can take
hours. BioLang's notification builtins send you a message when your analysis
finishes or fails — Slack, Teams, Telegram, Discord, or email.

### The `notify()` Builtin

`notify()` is the smart router. It reads `BIOLANG_NOTIFY` to determine which
provider to use, then sends the message.

```
# Simple string
notify("Alignment complete: 24 samples processed")

# Structured record — provider formats it natively
notify({
  title: "QC Pipeline Complete",
  status: "success",
  fields: {
    samples: 24,
    pass_rate: "96%",
    output: "/data/results/cohort.vcf.gz"
  }
})
```

If `BIOLANG_NOTIFY` is not set, `notify()` prints to stderr as a fallback.

### Provider-Specific Builtins

Each provider has a dedicated builtin for direct use:

```
# Slack (env: SLACK_WEBHOOK)
slack("Variant calling finished: 1,234 SNPs, 456 indels")

# Microsoft Teams (env: TEAMS_WEBHOOK)
teams("RNA-seq pipeline complete: 12 samples normalized")

# Telegram (env: TELEGRAM_BOT_TOKEN, TELEGRAM_CHAT_ID)
telegram("Alignment done: tumor.sorted.bam")

# Discord (env: DISCORD_WEBHOOK)
discord("FASTQ QC complete: 98% pass rate")

# Email (env: SMTP_HOST, SMTP_USER, SMTP_PASS)
email("lab@example.com", "Pipeline Complete", "Analysis finished successfully")
```

All webhook-based builtins accept an optional first argument for an explicit
webhook URL, so you can skip env vars:

```
slack("https://hooks.slack.com/services/xxx", "Pipeline done")
teams("https://outlook.office.com/webhook/xxx", "Pipeline done")
```

### Structured Messages

Pass a record instead of a string for rich formatting. Each provider renders
it natively (Slack Block Kit, Teams Adaptive Cards, Discord embeds):

```
slack({
  title: "Cohort Analysis Complete",
  status: "success",
  fields: {
    samples: 48,
    variants: "2.3M",
    ti_tv: 2.1,
    output: "cohort.annotated.vcf.gz"
  }
})
```

### Pipeline Integration

Use `notify()` in pipeline stages or with `defer` for guaranteed delivery:

```
pipeline variant_pipeline {
  defer {
    notify("Pipeline variant_pipeline finished")
  }

  stage align {
    shell("bwa-mem2 mem -t 16 GRCh38.fa R1.fq.gz R2.fq.gz | samtools sort -o aligned.bam")
    "aligned.bam"
  }

  stage call {
    shell("gatk HaplotypeCaller -R GRCh38.fa -I " + align + " -O raw.vcf.gz")
    let variants = read_vcf("raw.vcf.gz") |> collect()
    let snps = variants |> filter(|v| v.is_snp) |> len()
    let indels = variants |> filter(|v| v.is_indel) |> len()

    notify({
      title: "Variant Calling Complete",
      fields: {SNPs: snps, Indels: indels}
    })

    "raw.vcf.gz"
  }
}
```

### Setup

Set environment variables once in your shell profile:

```
# Pick your provider
export BIOLANG_NOTIFY=slack

# Slack
export SLACK_WEBHOOK=https://hooks.slack.com/services/T.../B.../xxx

# Telegram
export TELEGRAM_BOT_TOKEN=123456:ABC-DEF
export TELEGRAM_CHAT_ID=-1001234567890

# Email (for notify() with BIOLANG_NOTIFY=email)
export SMTP_HOST=smtp.gmail.com
export SMTP_USER=you@gmail.com
export SMTP_PASS=app-password
export NOTIFY_EMAIL_TO=lab@example.com
```

### Notification Builtins Summary

| Builtin | Provider | Env Vars |
|---------|----------|----------|
| `notify(msg)` | Auto | `BIOLANG_NOTIFY` + provider vars |
| `slack(msg)` | Slack | `SLACK_WEBHOOK` |
| `teams(msg)` | Teams | `TEAMS_WEBHOOK` |
| `telegram(msg)` | Telegram | `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID` |
| `discord(msg)` | Discord | `DISCORD_WEBHOOK` |
| `email(to, subj, body)` | SMTP | `SMTP_HOST`, `SMTP_USER`, `SMTP_PASS` |
